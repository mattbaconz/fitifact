use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::artifact::{Artifact, AudioCodec, Container, Family, StreamType, VideoCodec};
use crate::capability::TransformId;
use crate::constraints::Field;
use crate::error::{Error, ErrorCode, Result};
use crate::plan::{ExpectedFact, ExpectedValue, PLANNER_VERSION, Plan, PreservationClaim};

pub const MAX_PROCESS_STDOUT_BYTES: usize = 1024 * 1024;
pub const MAX_PROCESS_STDERR_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub timeout: Duration,
    pub temp_dir: Option<std::path::PathBuf>,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30 * 60),
            temp_dir: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnOutput {
    pub status: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

impl SpawnOutput {
    pub fn stdout_str(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    pub fn stderr_str(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }

    pub fn success(&self) -> bool {
        self.status == 0
    }
}

pub trait ProcessSpawner {
    fn spawn(&self, program: &str, args: &[String], timeout: Duration) -> Result<SpawnOutput>;
}

#[derive(Debug, Default)]
pub struct SystemSpawner;

impl ProcessSpawner for SystemSpawner {
    fn spawn(&self, program: &str, args: &[String], timeout: Duration) -> Result<SpawnOutput> {
        spawn_system(program, args, timeout, None)
    }
}

/// Records program names so tests can prove no-op never starts ffmpeg.
#[derive(Debug)]
pub struct RecordingSpawner<S> {
    inner: S,
    pub calls: std::sync::Mutex<Vec<String>>,
}

impl<S: ProcessSpawner> RecordingSpawner<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn programs(&self) -> Vec<String> {
        self.calls.lock().expect("call log").clone()
    }

    pub fn ffmpeg_spawn_count(&self) -> usize {
        self.programs()
            .iter()
            .filter(|p| program_is_ffmpeg(p))
            .count()
    }

    pub fn ffprobe_spawn_count(&self) -> usize {
        self.programs()
            .iter()
            .filter(|p| program_is_ffprobe(p))
            .count()
    }
}

impl<S: ProcessSpawner + ?Sized> ProcessSpawner for &S {
    fn spawn(&self, program: &str, args: &[String], timeout: Duration) -> Result<SpawnOutput> {
        (**self).spawn(program, args, timeout)
    }
}

impl<S: ProcessSpawner> ProcessSpawner for RecordingSpawner<S> {
    fn spawn(&self, program: &str, args: &[String], timeout: Duration) -> Result<SpawnOutput> {
        self.calls
            .lock()
            .expect("call log")
            .push(program.to_string());
        self.inner.spawn(program, args, timeout)
    }
}

pub fn program_is_ffmpeg(program: &str) -> bool {
    let name = Path::new(program)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(program);
    name.eq_ignore_ascii_case("ffmpeg")
}

pub fn program_is_ffprobe(program: &str) -> bool {
    let name = Path::new(program)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(program);
    name.eq_ignore_ascii_case("ffprobe")
}

pub trait TransformProvider {
    fn execute(
        &self,
        input: &Path,
        output: &Path,
        plan: &Plan,
        ctx: &ExecutionContext,
    ) -> Result<()>;

    fn stream_hashes(
        &self,
        _path: &Path,
        _artifact: &Artifact,
        _ctx: &ExecutionContext,
    ) -> Result<StreamHashes> {
        Err(Error::new(
            ErrorCode::ValidationFailed,
            "the media provider cannot prove copied-stream provenance",
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamHashes {
    pub algorithm: String,
    pub video: String,
    pub audio: Option<String>,
}

impl StreamHashes {
    pub fn new(video: impl Into<String>, audio: Option<impl Into<String>>) -> Self {
        Self {
            algorithm: "sha256".into(),
            video: video.into(),
            audio: audio.map(Into::into),
        }
    }
}

pub fn execute(
    provider: &dyn TransformProvider,
    input: &Path,
    output: &Path,
    plan: &Plan,
    artifact: &Artifact,
    ctx: &ExecutionContext,
) -> Result<()> {
    validate_plan_for_execution(plan, artifact)?;
    if same_path(input, output) {
        return Err(Error::new(
            ErrorCode::SecurityBlocked,
            "refusing to overwrite the original file",
        ));
    }
    provider.execute(input, output, plan, ctx)
}

pub fn validate_plan_for_execution(plan: &Plan, artifact: &Artifact) -> Result<()> {
    validate_plan_shape(plan)?;
    if artifact.family == Family::Image {
        return validate_image_plan(plan, artifact);
    }
    if artifact.family != Family::Media
        || artifact
            .streams
            .iter()
            .any(|stream| !matches!(stream.stream_type(), StreamType::Video | StreamType::Audio))
    {
        return Err(forged_plan());
    }
    let videos = artifact.video_streams().count();
    let audios = artifact.audio_streams().count();
    let audio_only = videos == 0 && audios == 1;
    if !(audio_only || videos == 1 && audios <= 1) {
        return Err(forged_plan());
    }
    let video = artifact.first_video();
    let audio = artifact.first_audio();
    if !audio_only
        && (video.is_none_or(|stream| stream.width.is_none() || stream.height.is_none())
            || artifact.duration_ms.is_none())
    {
        return Err(forged_plan());
    }

    let step = &plan.steps[0];
    match step.operation {
        TransformId::Remux => {
            let Some(video) = video else {
                return Err(forged_plan());
            };
            if !matches!(
                artifact.container,
                Some(Container::Mov) | Some(Container::Mkv) | Some(Container::Webm)
            ) || video.codec != Some(VideoCodec::H264)
                || audio.is_some_and(|stream| stream.codec != Some(AudioCodec::Aac))
                || step.expected
                    != vec![ExpectedFact {
                        field: Field::MediaContainer,
                        value: ExpectedValue::Container(Container::Mp4),
                    }]
                || step.preservation != vec![PreservationClaim::AllStreamsCopied]
            {
                return Err(forged_plan());
            }
        }
        TransformId::TranscodeAudio => {
            if step
                .preservation
                .contains(&PreservationClaim::AudioStreamCopied)
                || step
                    .preservation
                    .contains(&PreservationClaim::AllStreamsCopied)
            {
                return Err(forged_plan());
            }
            if !transcode_audio_expected_shape(&step.expected) {
                return Err(forged_plan());
            }
            if audio_only {
                if !step.is_audio_only_transcode()
                    || audio.is_none()
                    || step.target.max_bytes.is_some() && artifact.duration_ms.is_none()
                {
                    return Err(forged_plan());
                }
            } else {
                let Some(video) = video else {
                    return Err(forged_plan());
                };
                if !matches!(
                    artifact.container,
                    Some(Container::Mp4)
                        | Some(Container::Mov)
                        | Some(Container::Mkv)
                        | Some(Container::Webm)
                ) || video.codec != Some(VideoCodec::H264)
                    || audio.is_none()
                {
                    return Err(forged_plan());
                }
            }
        }
        TransformId::TranscodeVideo => {
            let Some(video) = video else {
                return Err(forged_plan());
            };
            if !matches!(
                artifact.container,
                Some(Container::Mp4)
                    | Some(Container::Mov)
                    | Some(Container::Webm)
                    | Some(Container::Mkv)
            ) || !matches!(
                video.codec,
                Some(VideoCodec::Hevc)
                    | Some(VideoCodec::H264)
                    | Some(VideoCodec::Vp9)
                    | Some(VideoCodec::Vp8)
                    | Some(VideoCodec::Av1)
            ) || !transcode_expected_shape(&step.expected)
            {
                return Err(forged_plan());
            }
            if step.target.max_bytes.is_some() && step.target.duration_ms != artifact.duration_ms {
                return Err(forged_plan());
            }
            if step
                .preservation
                .contains(&PreservationClaim::AudioStreamCopied)
                && audio.is_some_and(|stream| stream.codec != Some(AudioCodec::Aac))
            {
                return Err(forged_plan());
            }
            if step
                .preservation
                .contains(&PreservationClaim::VideoDimensions)
            {
                let (width, height) = (video.width, video.height);
                if step.target.width != width || step.target.height != height {
                    return Err(forged_plan());
                }
            }
        }
        TransformId::EncodeJpeg | TransformId::ImageAdapt => return Err(forged_plan()),
    }
    if plan.preserved != step.preservation {
        return Err(forged_plan());
    }
    Ok(())
}

fn validate_image_plan(plan: &Plan, artifact: &Artifact) -> Result<()> {
    let step = &plan.steps[0];
    if step.operation != TransformId::EncodeJpeg {
        return Err(forged_plan());
    }
    let image = artifact.image.as_ref().ok_or_else(forged_plan)?;
    let width = image.width.ok_or_else(forged_plan)?;
    let height = image.height.ok_or_else(forged_plan)?;
    if image.format != Some(crate::artifact::ImageFormat::Png)
        || image.animated != Some(false)
        || step.expected != crate::image::jpeg_expected(width, height)
        || step.preservation != vec![PreservationClaim::ImageDimensions]
        || plan.preserved != step.preservation
    {
        return Err(forged_plan());
    }
    Ok(())
}

pub(crate) fn validate_plan_shape(plan: &Plan) -> Result<()> {
    if plan.planner_version != PLANNER_VERSION || plan.steps.len() != 1 {
        return Err(forged_plan());
    }
    let step = &plan.steps[0];
    if step.id != "step-1" || step.reasons.is_empty() {
        return Err(forged_plan());
    }
    let valid_target = match step.operation {
        TransformId::Remux => {
            step.target.container == Some(Container::Mp4)
                && step.target.video_codec.is_none()
                && step.target.audio_codec.is_none()
                && step.target.max_bytes.is_none()
                && step.expected
                    == vec![ExpectedFact {
                        field: Field::MediaContainer,
                        value: ExpectedValue::Container(Container::Mp4),
                    }]
                && step.preservation == vec![PreservationClaim::AllStreamsCopied]
        }
        TransformId::TranscodeAudio => {
            step.target.container == Some(Container::Mp4)
                && step.target.audio_codec == Some(AudioCodec::Aac)
                && step.target.video_codec.is_none()
                && transcode_audio_expected_shape(&step.expected)
                && !step
                    .preservation
                    .contains(&PreservationClaim::AllStreamsCopied)
                && !step
                    .preservation
                    .contains(&PreservationClaim::AudioStreamCopied)
        }
        TransformId::TranscodeVideo => {
            step.target.container == Some(Container::Mp4)
                && step.target.video_codec == Some(VideoCodec::H264)
                && (step.target.max_bytes.is_none() || step.target.duration_ms.is_some())
                && step.target.width.is_some()
                && step.target.height.is_some()
                && transcode_expected_shape(&step.expected)
        }
        TransformId::EncodeJpeg => {
            step.target.image_format == Some(crate::artifact::ImageFormat::Jpeg)
                && step.preservation == vec![PreservationClaim::ImageDimensions]
        }
        TransformId::ImageAdapt => {
            step.target.container.is_none()
                && step.target.video_codec.is_none()
                && step.target.image_format.is_none()
                && step.target.image.is_some()
        }
    };
    if !valid_target || plan.preserved != step.preservation {
        return Err(forged_plan());
    }
    Ok(())
}

fn transcode_audio_expected_shape(expected: &[ExpectedFact]) -> bool {
    expected.iter().any(|fact| {
        *fact
            == ExpectedFact {
                field: Field::MediaContainer,
                value: ExpectedValue::Container(Container::Mp4),
            }
    }) && expected.iter().any(|fact| {
        *fact
            == ExpectedFact {
                field: Field::MediaAudioCodec,
                value: ExpectedValue::AudioCodec(AudioCodec::Aac),
            }
    })
}

fn transcode_expected_shape(expected: &[ExpectedFact]) -> bool {
    if expected.len() != 11 && expected.len() != 12 {
        return false;
    }
    let Some(ExpectedFact {
        field: Field::MediaVideoWidth,
        value: ExpectedValue::Integer(width),
    }) = expected.get(2)
    else {
        return false;
    };
    let Some(ExpectedFact {
        field: Field::MediaVideoHeight,
        value: ExpectedValue::Integer(height),
    }) = expected.get(3)
    else {
        return false;
    };
    let video_ok = *width > 0
        && *height > 0
        && expected[0]
            == (ExpectedFact {
                field: Field::MediaVideoCodec,
                value: ExpectedValue::VideoCodec(VideoCodec::H264),
            })
        && expected[1]
            == (ExpectedFact {
                field: Field::MediaContainer,
                value: ExpectedValue::Container(Container::Mp4),
            })
        && expected[4..11] == transcode_expected(1, 1)[4..];
    if !video_ok {
        return false;
    }
    if expected.len() == 11 {
        return true;
    }
    expected.get(11)
        == Some(&ExpectedFact {
            field: Field::MediaAudioCodec,
            value: ExpectedValue::AudioCodec(AudioCodec::Aac),
        })
}

fn transcode_expected(width: u32, height: u32) -> Vec<ExpectedFact> {
    vec![
        ExpectedFact {
            field: Field::MediaVideoCodec,
            value: ExpectedValue::VideoCodec(VideoCodec::H264),
        },
        ExpectedFact {
            field: Field::MediaContainer,
            value: ExpectedValue::Container(Container::Mp4),
        },
        ExpectedFact {
            field: Field::MediaVideoWidth,
            value: ExpectedValue::Integer(u64::from(width)),
        },
        ExpectedFact {
            field: Field::MediaVideoHeight,
            value: ExpectedValue::Integer(u64::from(height)),
        },
        ExpectedFact {
            field: Field::MediaVideoPixelFormat,
            value: ExpectedValue::Text("yuv420p".into()),
        },
        ExpectedFact {
            field: Field::MediaVideoBitDepth,
            value: ExpectedValue::Integer(8),
        },
        ExpectedFact {
            field: Field::MediaVideoColorRange,
            value: ExpectedValue::Text("tv".into()),
        },
        ExpectedFact {
            field: Field::MediaVideoColorSpace,
            value: ExpectedValue::Text("bt709".into()),
        },
        ExpectedFact {
            field: Field::MediaVideoColorTransfer,
            value: ExpectedValue::Text("bt709".into()),
        },
        ExpectedFact {
            field: Field::MediaVideoColorPrimaries,
            value: ExpectedValue::Text("bt709".into()),
        },
        ExpectedFact {
            field: Field::MediaVideoHdr,
            value: ExpectedValue::Text("sdr".into()),
        },
    ]
}

fn forged_plan() -> Error {
    Error::new(
        ErrorCode::SecurityBlocked,
        "execution refused because the typed plan does not match the inspected safe topology",
    )
}

fn same_path(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

#[derive(Debug, Clone)]
pub struct CancellableSystemSpawner(pub crate::CancellationToken);

impl ProcessSpawner for CancellableSystemSpawner {
    fn spawn(&self, program: &str, args: &[String], timeout: Duration) -> Result<SpawnOutput> {
        spawn_system(program, args, timeout, Some(&self.0))
    }
}

fn spawn_system(
    program: &str,
    args: &[String],
    timeout: Duration,
    cancellation: Option<&crate::CancellationToken>,
) -> Result<SpawnOutput> {
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Instant;

    if let Some(token) = cancellation {
        token.check()?;
    }
    let mut command = Command::new(program);
    // All subprocess output is captured below. A GUI host must not create a
    // console window for every ffprobe/FFmpeg operation on Windows.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut child = command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| map_spawn_error(program, err))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let out_handle = thread::spawn(move || {
        stdout
            .map(|mut stream| read_bounded_prefix(&mut stream, MAX_PROCESS_STDOUT_BYTES))
            .unwrap_or_default()
    });
    let err_handle = thread::spawn(move || {
        stderr
            .map(|mut stream| read_bounded_tail(&mut stream, MAX_PROCESS_STDERR_BYTES))
            .unwrap_or_default()
    });

    let start = Instant::now();
    loop {
        if let Some(error) = cancellation.and_then(|token| token.check().err()) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = out_handle.join();
            let _ = err_handle.join();
            return Err(error);
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = out_handle.join().unwrap_or_default();
                let stderr = err_handle.join().unwrap_or_default();
                let code = status.code().unwrap_or(-1);
                return Ok(SpawnOutput {
                    status: code,
                    stdout: stdout.bytes,
                    stderr: stderr.bytes,
                    stdout_truncated: stdout.truncated,
                    stderr_truncated: stderr.truncated,
                });
            }
            Ok(None) if start.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = out_handle.join();
                let _ = err_handle.join();
                return Err(Error::new(
                    ErrorCode::ExecutionLimit,
                    "external media tool exceeded its configured timeout",
                ));
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = out_handle.join();
                let _ = err_handle.join();
                return Err(Error::new(
                    ErrorCode::ExecutionFailed,
                    format!("external media tool could not be monitored: {}", err.kind()),
                ));
            }
        }
    }
}

#[derive(Debug, Default)]
struct BoundedCapture {
    bytes: Vec<u8>,
    truncated: bool,
}

fn read_bounded_prefix(reader: &mut dyn std::io::Read, limit: usize) -> BoundedCapture {
    let mut capture = BoundedCapture::default();
    let mut chunk = [0_u8; 8192];
    while let Ok(read) = reader.read(&mut chunk) {
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(capture.bytes.len());
        capture
            .bytes
            .extend_from_slice(&chunk[..read.min(remaining)]);
        capture.truncated |= read > remaining;
    }
    capture
}

fn read_bounded_tail(reader: &mut dyn std::io::Read, limit: usize) -> BoundedCapture {
    use std::collections::VecDeque;

    let mut tail = VecDeque::with_capacity(limit);
    let mut total = 0_usize;
    let mut chunk = [0_u8; 8192];
    while let Ok(read) = reader.read(&mut chunk) {
        if read == 0 {
            break;
        }
        total = total.saturating_add(read);
        for byte in &chunk[..read] {
            if tail.len() == limit {
                tail.pop_front();
            }
            tail.push_back(*byte);
        }
    }
    BoundedCapture {
        bytes: tail.into_iter().collect(),
        truncated: total > limit,
    }
}

fn map_spawn_error(program: &str, err: std::io::Error) -> Error {
    let _ = program;
    if err.kind() == std::io::ErrorKind::NotFound {
        Error::new(
            ErrorCode::ProviderMissing,
            "required FFmpeg tool was not found on PATH; install FFmpeg and run `fitifact doctor`",
        )
    } else {
        Error::new(
            ErrorCode::ExecutionFailed,
            format!("external media tool could not be started: {}", err.kind()),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnLog {
    pub programs: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::Container;
    use crate::capability::TransformId;
    use crate::contract::PlanSchema;
    use crate::plan::{PLANNER_VERSION, PlanStep, StepTarget};
    #[cfg(windows)]
    use std::time::Instant;

    struct ForbiddenSpawner;

    impl ProcessSpawner for ForbiddenSpawner {
        fn spawn(
            &self,
            program: &str,
            _args: &[String],
            _timeout: Duration,
        ) -> Result<SpawnOutput> {
            panic!("should not spawn {program}");
        }
    }

    #[test]
    fn execute_refuses_overwrite() {
        let plan = Plan {
            schema: PlanSchema,
            planner_version: PLANNER_VERSION.into(),
            steps: vec![PlanStep {
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
            }],
            preserved: Vec::new(),
            warnings: Vec::new(),
        };
        let provider = crate::ffmpeg::FfmpegProvider::new(ForbiddenSpawner);
        let artifact = crate::artifact::Artifact::media(
            Container::Mov,
            crate::artifact::VideoCodec::H264,
            Some(crate::artifact::AudioCodec::Aac),
            10,
        );
        let err = execute(
            &provider,
            Path::new("in.mp4"),
            Path::new("in.mp4"),
            &plan,
            &artifact,
            &ExecutionContext::default(),
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::SecurityBlocked);
    }

    #[test]
    fn execution_defaults_to_thirty_minutes() {
        assert_eq!(
            ExecutionContext::default().timeout,
            Duration::from_secs(30 * 60)
        );
    }

    #[test]
    fn runtime_rejects_unknown_container_for_the_transcode_capability() {
        let mp4_hevc = crate::artifact::Artifact::media(
            Container::Mp4,
            crate::artifact::VideoCodec::Hevc,
            Some(crate::artifact::AudioCodec::Aac),
            10,
        );
        let plan = crate::plan::plan(
            &mp4_hevc,
            &crate::constraints::media_h264_mp4_aac(),
            &crate::capability::default_catalog(),
        )
        .plan()
        .unwrap()
        .clone();
        let mut unknown = mp4_hevc;
        unknown.container = Some(Container::Unknown("ambiguous".into()));
        let error = validate_plan_for_execution(&plan, &unknown).unwrap_err();
        assert_eq!(error.code, ErrorCode::SecurityBlocked);
    }

    #[test]
    fn runtime_rejects_missing_dimensions_or_duration_as_forged() {
        let complete = crate::artifact::Artifact::media(
            Container::Mov,
            crate::artifact::VideoCodec::H264,
            Some(crate::artifact::AudioCodec::Aac),
            10,
        );
        let plan = crate::plan::plan(
            &complete,
            &crate::constraints::media_h264_mp4_aac(),
            &crate::capability::default_catalog(),
        )
        .plan()
        .unwrap()
        .clone();

        let mut missing_width = complete.clone();
        missing_width.first_video_mut().unwrap().width = None;
        assert_eq!(
            validate_plan_for_execution(&plan, &missing_width)
                .unwrap_err()
                .code,
            ErrorCode::SecurityBlocked
        );

        let mut missing_height = complete.clone();
        missing_height.first_video_mut().unwrap().height = None;
        assert_eq!(
            validate_plan_for_execution(&plan, &missing_height)
                .unwrap_err()
                .code,
            ErrorCode::SecurityBlocked
        );

        let mut missing_duration = complete;
        missing_duration.duration_ms = None;
        assert_eq!(
            validate_plan_for_execution(&plan, &missing_duration)
                .unwrap_err()
                .code,
            ErrorCode::SecurityBlocked
        );
    }

    #[cfg(windows)]
    #[test]
    fn system_spawner_bounds_stdout_and_keeps_only_stderr_tail() {
        let args = vec![
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-Command".into(),
            "$o='o' * (1024*1024+17); $e=('A' * 1024)+('Z' * (256*1024)); [Console]::Out.Write($o); [Console]::Error.Write($e)".into(),
        ];
        let output = SystemSpawner
            .spawn("powershell", &args, Duration::from_secs(10))
            .unwrap();
        assert_eq!(output.stdout.len(), 1024 * 1024);
        assert_eq!(output.stderr.len(), 256 * 1024);
        assert!(output.stderr.iter().all(|byte| *byte == b'Z'));
    }

    #[cfg(windows)]
    #[test]
    fn system_spawner_timeout_is_stable_and_reaped() {
        let args = vec![
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-Command".into(),
            "Start-Sleep -Seconds 5".into(),
        ];
        let started = Instant::now();
        let err = SystemSpawner
            .spawn("powershell", &args, Duration::from_millis(50))
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::ExecutionLimit);
        assert_eq!(
            err.message,
            "external media tool exceeded its configured timeout"
        );
        assert!(started.elapsed() < Duration::from_secs(3));
    }
}
