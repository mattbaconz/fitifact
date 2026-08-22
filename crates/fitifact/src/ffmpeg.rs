use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::artifact::{Artifact, Container, StreamType, VideoCodec};
use crate::capability::TransformId;
use crate::error::{Error, ErrorCode, Result};
use crate::plan::{Plan, PlanStep};
use crate::runtime::{
    ExecutionContext, ProcessSpawner, SpawnOutput, StreamHashes, SystemSpawner, TransformProvider,
};

const ALLOWED_PROTOCOLS: &str = "file";

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
        crate::runtime::validate_plan_shape(plan)?;

        let workspace = Workspace::create(ctx.temp_dir.as_deref())?;
        let result = self.run_steps(input, output, plan, ctx.timeout, &workspace);
        workspace.cleanup();
        result
    }

    fn stream_hashes(
        &self,
        path: &Path,
        artifact: &Artifact,
        ctx: &ExecutionContext,
    ) -> Result<StreamHashes> {
        let workspace = Workspace::create(ctx.temp_dir.as_deref())?;
        let hash_path = workspace.hash_path();
        let result = (|| {
            let args = streamhash_args(path, artifact, &hash_path)?;
            let output = self
                .spawner
                .spawn(&self.ffmpeg_program, &args, ctx.timeout)?;
            if !output.success() {
                return Err(Error::new(
                    ErrorCode::ValidationFailed,
                    "FFmpeg could not compute copied-stream provenance",
                ));
            }
            let metadata = std::fs::metadata(&hash_path).map_err(|_| {
                Error::new(
                    ErrorCode::ValidationFailed,
                    "FFmpeg did not produce stream provenance",
                )
            })?;
            if metadata.len() > 4096 {
                return Err(Error::new(
                    ErrorCode::ValidationFailed,
                    "FFmpeg stream provenance exceeded its safety bound",
                ));
            }
            let text = std::fs::read_to_string(&hash_path).map_err(|_| {
                Error::new(
                    ErrorCode::ValidationFailed,
                    "FFmpeg stream provenance was not readable",
                )
            })?;
            parse_streamhash(&text, artifact.first_audio().is_some())
        })();
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
    Err(Error::new(
        ErrorCode::ExecutionFailed,
        format!(
            "FFmpeg could not complete the {} operation (exit {})",
            step.operation.as_str(),
            output.status
        ),
    ))
}

/// Build an argv array from a typed step. Never interpolates untrusted strings.
pub fn ffmpeg_args(step: &PlanStep, input: &Path, output: &Path) -> Result<Vec<String>> {
    let mut args = vec![
        "-nostdin".into(),
        "-hide_banner".into(),
        "-n".into(),
        "-protocol_whitelist".into(),
        ALLOWED_PROTOCOLS.into(),
        "-i".into(),
        path_arg(input)?,
    ];

    match step.operation {
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
                "-color_range".into(),
                "tv".into(),
                "-colorspace".into(),
                "bt709".into(),
                "-color_trc".into(),
                "bt709".into(),
                "-color_primaries".into(),
                "bt709".into(),
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
        TransformId::EncodeJpeg | TransformId::ImageAdapt => {
            return Err(Error::new(
                ErrorCode::ExecutionFailed,
                "the FFmpeg provider does not encode images",
            ));
        }
    }

    args.push(path_arg(output)?);
    Ok(args)
}

fn streamhash_args(input: &Path, artifact: &Artifact, output: &Path) -> Result<Vec<String>> {
    let videos = artifact.video_streams().count();
    let audios = artifact.audio_streams().count();
    let unsafe_other = artifact
        .streams
        .iter()
        .any(|stream| !matches!(stream.stream_type(), StreamType::Video | StreamType::Audio));
    if videos != 1 || audios > 1 || unsafe_other {
        return Err(Error::new(
            ErrorCode::ValidationFailed,
            "stream provenance refused an unsafe or uncertain topology",
        ));
    }
    let mut args = vec![
        "-nostdin".into(),
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-n".into(),
        "-protocol_whitelist".into(),
        ALLOWED_PROTOCOLS.into(),
        "-i".into(),
        path_arg(input)?,
        "-map".into(),
        "0:v:0".into(),
    ];
    if audios == 1 {
        args.extend(["-map".into(), "0:a:0".into()]);
    }
    args.extend([
        "-c".into(),
        "copy".into(),
        "-f".into(),
        "streamhash".into(),
        "-hash".into(),
        "sha256".into(),
        path_arg(output)?,
    ]);
    Ok(args)
}

fn parse_streamhash(text: &str, expect_audio: bool) -> Result<StreamHashes> {
    let mut video = None;
    let mut audio = None;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let mut parts = line.trim().split(',');
        let _index = parts.next();
        let stream_type = parts.next();
        let digest = parts.next();
        if parts.next().is_some() {
            return invalid_streamhash();
        }
        let Some(digest) = digest.and_then(|value| value.strip_prefix("SHA256=")) else {
            return invalid_streamhash();
        };
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return invalid_streamhash();
        }
        match stream_type {
            Some("v") if video.is_none() => video = Some(digest.to_ascii_lowercase()),
            Some("a") if audio.is_none() => audio = Some(digest.to_ascii_lowercase()),
            _ => return invalid_streamhash(),
        }
    }
    let Some(video) = video else {
        return invalid_streamhash();
    };
    if expect_audio != audio.is_some() {
        return invalid_streamhash();
    }
    Ok(StreamHashes::new(video, audio))
}

fn invalid_streamhash<T>() -> Result<T> {
    Err(Error::new(
        ErrorCode::ValidationFailed,
        "FFmpeg returned malformed or incomplete stream provenance",
    ))
}

fn append_movflags(args: &mut Vec<String>, container: &Container) {
    if matches!(container, Container::Mp4 | Container::Mov) {
        args.extend(["-movflags".into(), "+faststart".into()]);
    }
}

fn container_from_step(step: &PlanStep) -> Option<Container> {
    step.target.container.clone()
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
    step.target.video_codec.clone().ok_or_else(|| {
        Error::new(
            ErrorCode::ExecutionFailed,
            "transcode_video step is missing a codec param",
        )
    })
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
        for attempt in 0..100_u32 {
            let dir = root.join(format!("fitifact-{}-{nanos}-{attempt}", std::process::id()));
            match std::fs::create_dir(&dir) {
                Ok(()) => return Ok(Self { dir }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => {
                    return Err(Error::new(
                        ErrorCode::ExecutionFailed,
                        "cannot create the isolated FFmpeg workspace",
                    ));
                }
            }
        }
        Err(Error::new(
            ErrorCode::ExecutionFailed,
            "cannot allocate a unique FFmpeg workspace",
        ))
    }

    fn step_path(&self, index: usize, output: &Path) -> PathBuf {
        let ext = output.extension().and_then(|e| e.to_str()).unwrap_or("mp4");
        self.dir.join(format!("step-{index}.{ext}"))
    }

    fn hash_path(&self) -> PathBuf {
        self.dir.join("streamhash.txt")
    }

    fn cleanup(&self) {
        let _ = std::fs::remove_file(self.hash_path());
        let _ = std::fs::remove_dir(&self.dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{Artifact, AudioCodec};
    use crate::constraints::media_h264_mp4_aac;
    use crate::plan::StepTarget;

    struct NeverSpawner;

    impl ProcessSpawner for NeverSpawner {
        fn spawn(
            &self,
            program: &str,
            _args: &[String],
            _timeout: Duration,
        ) -> Result<SpawnOutput> {
            panic!("forged plan must be rejected before spawning {program}")
        }
    }

    fn transcode_step() -> PlanStep {
        PlanStep {
            id: "step-1".into(),
            operation: TransformId::TranscodeVideo,
            target: StepTarget {
                video_codec: Some(VideoCodec::H264),
                container: Some(Container::Mp4),
                image_format: None,
                image: None,
            },
            reasons: Vec::new(),
            expected: Vec::new(),
            preservation: Vec::new(),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn remux_args_are_stream_copy_without_encoder() {
        let step = PlanStep {
            id: "step-1".into(),
            operation: TransformId::Remux,
            target: StepTarget {
                video_codec: None,
                container: Some(Container::Mp4),
                image_format: None,
                image: None,
            },
            reasons: Vec::new(),
            expected: Vec::new(),
            preservation: Vec::new(),
            warnings: Vec::new(),
        };
        let args = ffmpeg_args(&step, Path::new("in.mov"), Path::new("out.mp4")).unwrap();
        assert!(args.windows(2).any(|w| w == ["-map", "0"]));
        assert!(args.windows(2).any(|w| w == ["-c", "copy"]));
        assert!(!args.iter().any(|a| a.contains("libx264")));
        assert!(args.iter().any(|a| a == "-n"));
        assert!(!args.iter().any(|a| a == "-y"));
        assert!(
            args.windows(2)
                .any(|w| w == ["-protocol_whitelist", "file"])
        );
        assert_eq!(args.last().map(String::as_str), Some("out.mp4"));
    }

    #[test]
    fn transcode_args_copy_audio_and_use_allowlisted_encoder() {
        let args =
            ffmpeg_args(&transcode_step(), Path::new("in.mp4"), Path::new("out.mp4")).unwrap();
        assert!(args.windows(2).any(|w| w == ["-c:v", "libx264"]));
        assert!(args.windows(2).any(|w| w == ["-c:a", "copy"]));
        assert!(args.windows(2).any(|w| w == ["-map", "0:v:0"]));
        assert!(args.windows(2).any(|w| w == ["-map", "0:a:0?"]));
        for (flag, value) in [
            ("-pix_fmt", "yuv420p"),
            ("-color_range", "tv"),
            ("-colorspace", "bt709"),
            ("-color_trc", "bt709"),
            ("-color_primaries", "bt709"),
        ] {
            assert!(args.windows(2).any(|w| w == [flag, value]));
        }
        assert!(
            args.windows(2)
                .any(|w| w == ["-protocol_whitelist", "file"])
        );
        assert!(args.iter().any(|a| a == "-n"));
        assert!(!args.iter().any(|a| a == "-y"));
        assert!(!args.join(" ").contains("sh -c"));
    }

    #[test]
    fn transcode_rejects_non_h264_target() {
        let mut step = transcode_step();
        step.target.video_codec = Some(VideoCodec::Hevc);
        let err = ffmpeg_args(&step, Path::new("in.mp4"), Path::new("out.mp4")).unwrap_err();
        assert_eq!(err.code, ErrorCode::ExecutionFailed);
    }

    #[test]
    fn provider_rejects_forged_plan_before_spawn() {
        let artifact =
            Artifact::media(Container::Mov, VideoCodec::H264, Some(AudioCodec::Aac), 100);
        let mut plan = crate::plan::plan(
            &artifact,
            &media_h264_mp4_aac(),
            &crate::capability::default_catalog(),
        )
        .plan()
        .unwrap()
        .clone();
        plan.steps[0].expected.clear();
        let provider = FfmpegProvider::new(NeverSpawner);
        let err = provider
            .execute(
                Path::new("input.mov"),
                Path::new("output.mp4"),
                &plan,
                &ExecutionContext::default(),
            )
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::SecurityBlocked);
    }

    #[test]
    fn streamhash_argv_is_file_only_typed_and_maps_represented_streams() {
        let artifact =
            Artifact::media(Container::Mp4, VideoCodec::H264, Some(AudioCodec::Aac), 100);
        let args =
            streamhash_args(Path::new("input.mp4"), &artifact, Path::new("hashes.txt")).unwrap();
        assert!(args.windows(2).any(|w| w == ["-map", "0:v:0"]));
        assert!(args.windows(2).any(|w| w == ["-map", "0:a:0"]));
        assert!(args.windows(2).any(|w| w == ["-c", "copy"]));
        assert!(args.windows(2).any(|w| w == ["-f", "streamhash"]));
        assert!(args.windows(2).any(|w| w == ["-hash", "sha256"]));
        assert!(
            args.windows(2)
                .any(|w| w == ["-protocol_whitelist", "file"])
        );
        assert!(args.iter().any(|arg| arg == "-n"));
        assert!(!args.iter().any(|arg| arg == "-y"));
    }

    #[test]
    fn parses_exact_sha256_streamhash_output() {
        let hashes = parse_streamhash(
            "0,v,SHA256=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n1,a,SHA256=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n",
            true,
        )
        .unwrap();
        assert_eq!(hashes.video, "a".repeat(64));
        assert_eq!(hashes.audio, Some("b".repeat(64)));
    }
}
