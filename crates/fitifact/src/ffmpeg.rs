use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::artifact::{Artifact, Container, HdrStatus, StreamType, VideoCodec};
use crate::capability::{CapabilityCatalog, TransformId, default_catalog};
use crate::constraints::Field;
use crate::error::{Error, ErrorCode, Result};
use crate::media_fit::{
    DEFAULT_AUDIO_BPS, audio_bitrate_ladder, bitrate_ladder, floor_video_bitrate_bps,
    video_bitrate_from_budget,
};
use crate::media_tools::resolve_media_tools;
use crate::plan::{ExpectedFact, ExpectedValue, Plan, PlanStep};
use crate::runtime::{
    ExecutionContext, ProcessSpawner, SpawnOutput, StreamHashes, SystemSpawner, TransformProvider,
};

const ALLOWED_PROTOCOLS: &str = "file";
/// Linearize, map BT.2020 primaries to BT.709, compress highlights (Hable), and re-encode
/// for limited-range BT.709 SDR. Needs the zscale filter (libzimg).
const TONE_MAP_TO_SDR: &str = "zscale=t=linear:npl=100,format=gbrpf32le,zscale=p=bt709,tonemap=tonemap=hable:desat=0,zscale=t=bt709:m=bt709:r=tv,format=yuv420p";
/// FFmpeg ignores `-color_trc`/`-color_primaries` for frames that carry no color
/// tags, so tag the frames themselves; validation would otherwise see no transfer.
const TAG_BT709: &str = "setparams=color_primaries=bt709:color_trc=bt709";

/// True when this FFmpeg build has both filters the HDR to SDR chain needs.
pub fn ffmpeg_can_tone_map<S: ProcessSpawner>(spawner: &S, ffmpeg_program: &str) -> bool {
    spawner
        .spawn(
            ffmpeg_program,
            &["-hide_banner".into(), "-filters".into()],
            Duration::from_secs(15),
        )
        .ok()
        .filter(SpawnOutput::success)
        .is_some_and(|output| {
            let listing = output.stdout_str();
            ["zscale", "tonemap"].iter().all(|name| {
                listing
                    .lines()
                    .any(|line| line.split_whitespace().nth(1) == Some(name))
            })
        })
}

/// The planner catalog for this artifact. Probes FFmpeg only for HDR sources, so the
/// common path spawns nothing extra.
pub fn catalog_for(artifact: &Artifact) -> CapabilityCatalog {
    let hdr = artifact
        .first_video()
        .is_some_and(|video| video.hdr == HdrStatus::Hdr);
    default_catalog().with_hdr_tone_mapping(
        hdr && ffmpeg_can_tone_map(&SystemSpawner, &resolve_media_tools().ffmpeg_program()),
    )
}

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
        Self::with_program(spawner, resolve_media_tools().ffmpeg_program())
    }

    pub fn with_program(spawner: S, ffmpeg_program: impl Into<String>) -> Self {
        Self {
            ffmpeg_program: ffmpeg_program.into(),
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
            parse_streamhash(
                &text,
                artifact.first_video().is_some(),
                artifact.first_audio().is_some(),
            )
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
            if step.operation == TransformId::TranscodeVideo {
                self.run_transcode(step, &current, &dest, timeout)?;
            } else if step.operation == TransformId::TranscodeAudio
                && step.target.max_bytes.is_some()
            {
                self.run_audio_transcode(step, &current, &dest, timeout)?;
            } else {
                let args = ffmpeg_args(step, &current, &dest)?;
                let spawned = self.spawner.spawn(&self.ffmpeg_program, &args, timeout)?;
                check_ffmpeg_status(spawned, step)?;
            }
            if !last {
                current = dest;
            }
        }
        Ok(())
    }

    fn run_transcode(
        &self,
        step: &PlanStep,
        input: &Path,
        output: &Path,
        timeout: Duration,
    ) -> Result<()> {
        let limit = step.target.max_bytes;
        let input_already_over = limit.is_some_and(|ceiling| {
            std::fs::metadata(input)
                .map(|meta| meta.len() > ceiling)
                .unwrap_or(true)
        });
        if !input_already_over {
            let first = ffmpeg_args_with_bitrate(step, input, output, None, None)?;
            let spawned = self.spawner.spawn(&self.ffmpeg_program, &first, timeout)?;
            check_ffmpeg_status(spawned, step)?;
            let Some(ceiling) = limit else {
                return Ok(());
            };
            let size = std::fs::metadata(output)
                .map(|meta| meta.len())
                .unwrap_or(u64::MAX);
            if size <= ceiling {
                return Ok(());
            }
            let _ = std::fs::remove_file(output);
        }
        let Some(ceiling) = limit else {
            return Ok(());
        };
        let duration_ms = step.target.duration_ms.ok_or_else(|| {
            Error::new(
                ErrorCode::ExecutionFailed,
                "size-fitting encode is missing duration",
            )
        })?;
        let has_audio = step
            .preservation
            .contains(&crate::plan::PreservationClaim::AudioStreamCopied)
            || step.target.audio_codec.is_some();
        let Some(initial) = video_bitrate_from_budget(ceiling, duration_ms, has_audio) else {
            return Err(Error::new(
                ErrorCode::ExecutionLimit,
                "the size limit is below the quality floor this encoder will use",
            ));
        };
        let (width, height) = video_dimensions_from_step(step);
        let floor = floor_video_bitrate_bps(width, height);
        if initial < floor {
            return Err(Error::new(
                ErrorCode::ExecutionLimit,
                "the size limit is below the quality floor this encoder will use",
            ));
        }
        for bitrate in bitrate_ladder(initial, floor) {
            let args = ffmpeg_args_with_bitrate(step, input, output, Some(bitrate), None)?;
            let spawned = self.spawner.spawn(&self.ffmpeg_program, &args, timeout)?;
            check_ffmpeg_status(spawned, step)?;
            let size = std::fs::metadata(output)
                .map(|meta| meta.len())
                .unwrap_or(u64::MAX);
            if size <= ceiling {
                return Ok(());
            }
            let _ = std::fs::remove_file(output);
        }
        Err(Error::new(
            ErrorCode::ExecutionLimit,
            "the encoder reached its quality floor before the file fit the size limit",
        ))
    }

    fn run_audio_transcode(
        &self,
        step: &PlanStep,
        input: &Path,
        output: &Path,
        timeout: Duration,
    ) -> Result<()> {
        let limit = step.target.max_bytes;
        let input_already_over = limit.is_some_and(|ceiling| {
            std::fs::metadata(input)
                .map(|meta| meta.len() > ceiling)
                .unwrap_or(true)
        });
        if !input_already_over {
            let first =
                ffmpeg_args_with_bitrate(step, input, output, None, Some(DEFAULT_AUDIO_BPS))?;
            let spawned = self.spawner.spawn(&self.ffmpeg_program, &first, timeout)?;
            check_ffmpeg_status(spawned, step)?;
            let Some(ceiling) = limit else {
                return Ok(());
            };
            let size = std::fs::metadata(output)
                .map(|meta| meta.len())
                .unwrap_or(u64::MAX);
            if size <= ceiling {
                return Ok(());
            }
            let _ = std::fs::remove_file(output);
        }
        let Some(ceiling) = limit else {
            return Ok(());
        };
        for bitrate in audio_bitrate_ladder() {
            let args = ffmpeg_args_with_bitrate(step, input, output, None, Some(*bitrate))?;
            let spawned = self.spawner.spawn(&self.ffmpeg_program, &args, timeout)?;
            check_ffmpeg_status(spawned, step)?;
            let size = std::fs::metadata(output)
                .map(|meta| meta.len())
                .unwrap_or(u64::MAX);
            if size <= ceiling {
                return Ok(());
            }
            let _ = std::fs::remove_file(output);
        }
        Err(Error::new(
            ErrorCode::ExecutionLimit,
            "the encoder reached its quality floor before the file fit the size limit",
        ))
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
    ffmpeg_args_with_bitrate(step, input, output, None, None)
}

fn ffmpeg_args_with_bitrate(
    step: &PlanStep,
    input: &Path,
    output: &Path,
    video_bitrate_bps: Option<u64>,
    audio_bitrate_bps: Option<u64>,
) -> Result<Vec<String>> {
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
        TransformId::TranscodeAudio => {
            let audio_rate = audio_bitrate_arg(audio_bitrate_bps.unwrap_or(DEFAULT_AUDIO_BPS));
            if step.is_audio_only_transcode() {
                args.extend([
                    "-vn".into(),
                    "-map".into(),
                    "0:a:0".into(),
                    "-c:a".into(),
                    "aac".into(),
                    "-b:a".into(),
                    audio_rate,
                    "-ac".into(),
                    "2".into(),
                ]);
            } else {
                args.extend([
                    "-map".into(),
                    "0:v:0?".into(),
                    "-map".into(),
                    "0:a:0".into(),
                    "-c:v".into(),
                    "copy".into(),
                    "-c:a".into(),
                    "aac".into(),
                    "-b:a".into(),
                    audio_rate,
                    "-ac".into(),
                    "2".into(),
                ]);
            }
            let container = container_from_step(step).unwrap_or(Container::Mp4);
            append_movflags(&mut args, &container);
        }
        TransformId::TranscodeVideo => {
            let encoder = match required_video_codec(step)? {
                VideoCodec::H264 => "libx264",
                other => {
                    return Err(Error::new(
                        ErrorCode::ExecutionFailed,
                        format!("this ffmpeg provider cannot encode {}", other.as_str()),
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
            ]);
            let mut filters = Vec::new();
            if step.target.tone_map_hdr {
                filters.push(TONE_MAP_TO_SDR.to_string());
            }
            if let Some((width, height)) = scale_from_step(step) {
                filters.push(format!("scale={width}:{height}:flags=lanczos"));
            }
            filters.push(TAG_BT709.to_string());
            args.extend(["-vf".into(), filters.join(",")]);
            args.extend([
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
            ]);
            if let Some(bitrate) = video_bitrate_bps {
                args.extend([
                    "-b:v".into(),
                    format!("{bitrate}"),
                    "-maxrate".into(),
                    format!("{bitrate}"),
                    "-bufsize".into(),
                    format!("{}", bitrate.saturating_mul(2)),
                ]);
            } else {
                args.extend(["-crf".into(), "23".into()]);
            }
            if step
                .preservation
                .contains(&crate::plan::PreservationClaim::AudioStreamCopied)
            {
                args.extend(["-c:a".into(), "copy".into()]);
            } else {
                args.extend([
                    "-c:a".into(),
                    "aac".into(),
                    "-b:a".into(),
                    "160k".into(),
                    "-ac".into(),
                    "2".into(),
                ]);
            }
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
    if (videos != 1 || audios > 1 || unsafe_other) && !(videos == 0 && audios == 1 && !unsafe_other)
    {
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
    ];
    if videos == 1 {
        args.extend(["-map".into(), "0:v:0".into()]);
    }
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

fn parse_streamhash(text: &str, expect_video: bool, expect_audio: bool) -> Result<StreamHashes> {
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
    if expect_video != video.is_some() {
        return invalid_streamhash();
    }
    if expect_audio != audio.is_some() {
        return invalid_streamhash();
    }
    Ok(StreamHashes::new(video.unwrap_or_default(), audio))
}

fn invalid_streamhash<T>() -> Result<T> {
    Err(Error::new(
        ErrorCode::ValidationFailed,
        "FFmpeg returned malformed or incomplete stream provenance",
    ))
}

fn audio_bitrate_arg(bps: u64) -> String {
    format!("{}k", (bps / 1000).max(1))
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

fn video_dimensions_from_step(step: &PlanStep) -> (u32, u32) {
    match (step.target.width, step.target.height) {
        (Some(width), Some(height)) if width > 0 && height > 0 => return (width, height),
        _ => {}
    }
    let mut width = 16u32;
    let mut height = 16u32;
    for fact in &step.expected {
        match fact {
            ExpectedFact {
                field: Field::MediaVideoWidth,
                value: ExpectedValue::Integer(value),
            } => {
                width = u32::try_from(*value).unwrap_or(16).max(1);
            }
            ExpectedFact {
                field: Field::MediaVideoHeight,
                value: ExpectedValue::Integer(value),
            } => {
                height = u32::try_from(*value).unwrap_or(16).max(1);
            }
            _ => {}
        }
    }
    (width, height)
}

fn scale_from_step(step: &PlanStep) -> Option<(u32, u32)> {
    if step
        .preservation
        .contains(&crate::plan::PreservationClaim::VideoDimensions)
    {
        return None;
    }
    match (step.target.width, step.target.height) {
        (Some(width), Some(height)) if width > 1 && height > 1 => Some((width, height)),
        _ => None,
    }
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
    use crate::artifact::{Artifact, AudioCodec, VideoCodec};
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
                ..StepTarget::default()
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
                container: Some(Container::Mp4),
                ..StepTarget::default()
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
    fn transcode_args_copy_audio_when_preservation_claims_it() {
        let mut step = transcode_step();
        step.preservation = vec![crate::plan::PreservationClaim::AudioStreamCopied];
        let args = ffmpeg_args(&step, Path::new("in.mp4"), Path::new("out.mp4")).unwrap();
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
        assert!(args.windows(2).any(|w| w == ["-vf", TAG_BT709]));
        assert!(
            args.windows(2)
                .any(|w| w == ["-protocol_whitelist", "file"])
        );
        assert!(args.iter().any(|a| a == "-n"));
        assert!(!args.iter().any(|a| a == "-y"));
        assert!(!args.join(" ").contains("sh -c"));
    }

    #[test]
    fn transcode_args_encode_aac_when_audio_is_not_copied() {
        let args = ffmpeg_args(
            &transcode_step(),
            Path::new("in.webm"),
            Path::new("out.mp4"),
        )
        .unwrap();
        assert!(args.windows(2).any(|w| w == ["-c:a", "aac"]));
        assert!(args.windows(2).any(|w| w == ["-b:a", "160k"]));
        assert!(!args.windows(2).any(|w| w == ["-c:a", "copy"]));
    }

    #[test]
    fn transcode_args_scale_when_dimensions_are_not_preserved() {
        let mut step = transcode_step();
        step.target.width = Some(1280);
        step.target.height = Some(720);
        let args = ffmpeg_args(&step, Path::new("in.mp4"), Path::new("out.mp4")).unwrap();
        let filters = format!("scale=1280:720:flags=lanczos,{TAG_BT709}");
        assert!(args.windows(2).any(|w| w[0] == "-vf" && w[1] == filters));
    }

    #[test]
    fn transcode_args_tone_map_hdr_before_scaling() {
        let mut step = transcode_step();
        step.target.tone_map_hdr = true;
        step.target.width = Some(1280);
        step.target.height = Some(720);
        let args = ffmpeg_args(&step, Path::new("in.mp4"), Path::new("out.mp4")).unwrap();
        let filters = format!("{TONE_MAP_TO_SDR},scale=1280:720:flags=lanczos,{TAG_BT709}");
        assert!(args.windows(2).any(|w| w[0] == "-vf" && w[1] == filters));
    }

    struct FiltersSpawner(&'static str);

    impl ProcessSpawner for FiltersSpawner {
        fn spawn(
            &self,
            _program: &str,
            args: &[String],
            _timeout: Duration,
        ) -> Result<SpawnOutput> {
            assert_eq!(args.join(" "), "-hide_banner -filters");
            Ok(SpawnOutput {
                status: 0,
                stdout: self.0.as_bytes().to_vec(),
                stderr: Vec::new(),
                stdout_truncated: false,
                stderr_truncated: false,
            })
        }
    }

    #[test]
    fn tone_mapping_needs_both_zscale_and_tonemap_filters() {
        let both = " T.. zscale            V->V       Apply resizing.\n ... tonemap           V->V       Conversion.\n";
        assert!(ffmpeg_can_tone_map(&FiltersSpawner(both), "ffmpeg"));
        let tonemap_only = " ... tonemap           V->V       Conversion.\n";
        assert!(!ffmpeg_can_tone_map(
            &FiltersSpawner(tonemap_only),
            "ffmpeg"
        ));
        let lookalikes =
            " ... tonemap_opencl    V->V       x\n ... zscale_cuda       V->V       x\n";
        assert!(!ffmpeg_can_tone_map(&FiltersSpawner(lookalikes), "ffmpeg"));
    }

    #[test]
    fn transcode_audio_args_copy_video() {
        let step = PlanStep {
            id: "step-1".into(),
            operation: TransformId::TranscodeAudio,
            target: crate::plan::StepTarget {
                container: Some(Container::Mp4),
                audio_codec: Some(AudioCodec::Aac),
                ..crate::plan::StepTarget::default()
            },
            reasons: Vec::new(),
            expected: vec![crate::plan::ExpectedFact {
                field: Field::MediaVideoCodec,
                value: crate::plan::ExpectedValue::VideoCodec(VideoCodec::H264),
            }],
            preservation: vec![crate::plan::PreservationClaim::VideoDimensions],
            warnings: Vec::new(),
        };
        let args = ffmpeg_args(&step, Path::new("in.mov"), Path::new("out.mp4")).unwrap();
        assert!(args.windows(2).any(|w| w == ["-c:v", "copy"]));
        assert!(args.windows(2).any(|w| w == ["-c:a", "aac"]));
        assert!(args.windows(2).any(|w| w == ["-map", "0:a:0"]));
        assert!(!args.iter().any(|arg| arg == "-vn"));
    }

    #[test]
    fn transcode_audio_only_drops_the_picture_map() {
        let step = PlanStep {
            id: "step-1".into(),
            operation: TransformId::TranscodeAudio,
            target: crate::plan::StepTarget {
                container: Some(Container::Mp4),
                audio_codec: Some(AudioCodec::Aac),
                ..crate::plan::StepTarget::default()
            },
            reasons: Vec::new(),
            expected: vec![
                crate::plan::ExpectedFact {
                    field: Field::MediaContainer,
                    value: crate::plan::ExpectedValue::Container(Container::Mp4),
                },
                crate::plan::ExpectedFact {
                    field: Field::MediaAudioCodec,
                    value: crate::plan::ExpectedValue::AudioCodec(AudioCodec::Aac),
                },
            ],
            preservation: Vec::new(),
            warnings: Vec::new(),
        };
        let args = ffmpeg_args(&step, Path::new("in.wav"), Path::new("out.mp4")).unwrap();
        assert!(args.iter().any(|arg| arg == "-vn"));
        assert!(args.windows(2).any(|w| w == ["-c:a", "aac"]));
        assert!(!args.windows(2).any(|w| w == ["-c:v", "copy"]));
    }

    #[test]
    fn transcode_bitrate_args_use_an_allowlisted_ladder() {
        let args = ffmpeg_args_with_bitrate(
            &transcode_step(),
            Path::new("in.mp4"),
            Path::new("out.mp4"),
            Some(400_000),
            None,
        )
        .unwrap();
        assert!(args.windows(2).any(|w| w == ["-b:v", "400000"]));
        assert!(args.windows(2).any(|w| w == ["-maxrate", "400000"]));
        assert!(args.windows(2).any(|w| w == ["-bufsize", "800000"]));
        assert!(!args.windows(2).any(|w| w == ["-crf", "23"]));
        assert!(args.windows(2).any(|w| w == ["-c:a", "aac"]));
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
            true,
        )
        .unwrap();
        assert_eq!(hashes.video, "a".repeat(64));
        assert_eq!(hashes.audio, Some("b".repeat(64)));
    }
}
