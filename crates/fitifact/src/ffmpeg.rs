use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::artifact::{Container, VideoCodec};
use crate::capability::TransformId;
use crate::constraints::Field;
use crate::error::{Error, ErrorCode, Result};
use crate::plan::{Plan, PlanStep};
use crate::runtime::{
    ExecutionContext, ProcessSpawner, SpawnOutput, SystemSpawner, TransformProvider,
};

const ALLOWED_PROTOCOLS: &str = "file,crypto,data";

#[derive(Debug, Clone)]
pub struct FfmpegProvider<S = SystemSpawner> {
    pub ffmpeg_program: String,
    spawner: S,
}

impl Default for FfmpegProvider<SystemSpawner> {
    fn default() -> Self {
        Self::new(SystemSpawner)
    }
}

impl<S> FfmpegProvider<S> {
    pub fn new(spawner: S) -> Self {
        Self {
            ffmpeg_program: "ffmpeg".into(),
            spawner,
        }
    }
}

impl<S: ProcessSpawner> TransformProvider for FfmpegProvider<S> {
    fn execute(
        &self,
        input: &Path,
        output: &Path,
        plan: &Plan,
        ctx: &ExecutionContext,
    ) -> Result<()> {
        if plan.steps.is_empty() {
            return Err(Error::new(
                ErrorCode::ExecutionFailed,
                "ffmpeg provider received an empty plan",
            ));
        }

        let workspace = Workspace::create(ctx.temp_dir.as_deref())?;
        let result = self.run_steps(input, output, plan, ctx.timeout, &workspace);
        workspace.cleanup();
        result
    }
}

impl<S: ProcessSpawner> FfmpegProvider<S> {
    fn run_steps(
        &self,
        input: &Path,
        output: &Path,
        plan: &Plan,
        timeout: Duration,
        workspace: &Workspace,
    ) -> Result<()> {
        let mut current = input.to_path_buf();
        for (i, step) in plan.steps.iter().enumerate() {
            let last = i + 1 == plan.steps.len();
            let dest = if last {
                output.to_path_buf()
            } else {
                workspace.step_path(i, output)
            };
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|err| {
                    Error::new(
                        ErrorCode::ExecutionFailed,
                        format!("cannot create output directory: {err}"),
                    )
                })?;
            }
            let args = ffmpeg_args(step, &current, &dest)?;
            let spawned = self.spawner.spawn(&self.ffmpeg_program, &args, timeout)?;
            check_ffmpeg_status(spawned, step)?;
            if !last {
                current = dest;
            }
        }
        Ok(())
    }
}

fn check_ffmpeg_status(output: SpawnOutput, step: &PlanStep) -> Result<()> {
    if output.success() {
        return Ok(());
    }
    let stderr = output.stderr_str();
    let tail = stderr
        .lines()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    Err(Error::new(
        ErrorCode::ExecutionFailed,
        format!(
            "ffmpeg failed on {} (exit {}). {}",
            step.transform.as_str(),
            output.status,
            tail
        ),
    ))
}

/// Build an argv array from a typed step. Never interpolates untrusted strings.
pub fn ffmpeg_args(step: &PlanStep, input: &Path, output: &Path) -> Result<Vec<String>> {
    let mut args = vec![
        "-nostdin".into(),
        "-hide_banner".into(),
        "-y".into(),
        "-protocol_whitelist".into(),
        ALLOWED_PROTOCOLS.into(),
        "-i".into(),
        path_arg(input)?,
    ];

    match step.transform {
        TransformId::Remux => {
            let container = required_container(step)?;
            args.extend(["-map".into(), "0".into(), "-c".into(), "copy".into()]);
            append_movflags(&mut args, &container);
        }
        TransformId::TranscodeVideo => {
            let encoder = match required_video_codec(step)? {
                VideoCodec::H264 => "libx264",
                other => {
                    return Err(Error::new(
                        ErrorCode::ExecutionFailed,
                        format!("v0 ffmpeg provider cannot encode {}", other.as_str()),
                    ));
                }
            };
            args.extend([
                "-map".into(),
                "0:v:0".into(),
                "-map".into(),
                "0:a:0?".into(),
                "-c:v".into(),
                encoder.into(),
                "-pix_fmt".into(),
                "yuv420p".into(),
                "-preset".into(),
                "medium".into(),
                "-crf".into(),
                "23".into(),
                "-c:a".into(),
                "copy".into(),
            ]);
            let container = container_from_step(step).unwrap_or(Container::Mp4);
            append_movflags(&mut args, &container);
        }
    }

    args.push(path_arg(output)?);
    Ok(args)
}

fn append_movflags(args: &mut Vec<String>, container: &Container) {
    if matches!(container, Container::Mp4 | Container::Mov) {
        args.extend(["-movflags".into(), "+faststart".into()]);
    }
}

fn container_from_step(step: &PlanStep) -> Option<Container> {
    step.params
        .iter()
        .find(|p| p.field == Field::MediaContainer)
        .map(|p| Container::parse_loose(&p.value))
}

fn required_container(step: &PlanStep) -> Result<Container> {
    container_from_step(step).ok_or_else(|| {
        Error::new(
            ErrorCode::ExecutionFailed,
            "remux step is missing a container param",
        )
    })
}

fn required_video_codec(step: &PlanStep) -> Result<VideoCodec> {
    let raw = step
        .params
        .iter()
        .find(|p| p.field == Field::MediaVideoCodec)
        .map(|p| p.value.as_str())
        .ok_or_else(|| {
            Error::new(
                ErrorCode::ExecutionFailed,
                "transcode_video step is missing a codec param",
            )
        })?;
    Ok(VideoCodec::parse_loose(raw))
}

fn path_arg(path: &Path) -> Result<String> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| Error::new(ErrorCode::InputInvalid, "path is not valid UTF-8"))
}

struct Workspace {
    dir: PathBuf,
}

impl Workspace {
    fn create(base: Option<&Path>) -> Result<Self> {
        let root = base
            .map(Path::to_path_buf)
            .unwrap_or_else(std::env::temp_dir);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = root.join(format!("fitifact-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|err| {
            Error::new(
                ErrorCode::ExecutionFailed,
                format!("cannot create workspace: {err}"),
            )
        })?;
        Ok(Self { dir })
    }

    fn step_path(&self, index: usize, output: &Path) -> PathBuf {
        let ext = output.extension().and_then(|e| e.to_str()).unwrap_or("mp4");
        self.dir.join(format!("step-{index}.{ext}"))
    }

    fn cleanup(&self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::StepParam;

    fn transcode_step() -> PlanStep {
        PlanStep {
            id: "step-1".into(),
            transform: TransformId::TranscodeVideo,
            params: vec![
                StepParam {
                    field: Field::MediaVideoCodec,
                    value: "h264".into(),
                },
                StepParam {
                    field: Field::MediaContainer,
                    value: "mp4".into(),
                },
            ],
            reason: vec!["video-codec".into()],
        }
    }

    #[test]
    fn remux_args_are_stream_copy_without_encoder() {
        let step = PlanStep {
            id: "step-1".into(),
            transform: TransformId::Remux,
            params: vec![StepParam {
                field: Field::MediaContainer,
                value: "mp4".into(),
            }],
            reason: vec!["container".into()],
        };
        let args = ffmpeg_args(&step, Path::new("in.mov"), Path::new("out.mp4")).unwrap();
        assert!(args.windows(2).any(|w| w == ["-c", "copy"]));
        assert!(!args.iter().any(|a| a.contains("libx264")));
        assert_eq!(args.last().map(String::as_str), Some("out.mp4"));
    }

    #[test]
    fn transcode_args_copy_audio_and_use_allowlisted_encoder() {
        let args =
            ffmpeg_args(&transcode_step(), Path::new("in.mp4"), Path::new("out.mp4")).unwrap();
        assert!(args.windows(2).any(|w| w == ["-c:v", "libx264"]));
        assert!(args.windows(2).any(|w| w == ["-c:a", "copy"]));
        assert!(
            args.windows(2)
                .any(|w| w == ["-protocol_whitelist", ALLOWED_PROTOCOLS])
        );
        assert!(!args.join(" ").contains("sh -c"));
    }

    #[test]
    fn transcode_rejects_non_h264_target() {
        let mut step = transcode_step();
        step.params[0].value = "hevc".into();
        let err = ffmpeg_args(&step, Path::new("in.mp4"), Path::new("out.mp4")).unwrap_err();
        assert_eq!(err.code, ErrorCode::ExecutionFailed);
    }
}
