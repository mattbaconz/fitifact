use std::path::{Path, PathBuf};
use std::time::Duration;

use fitifact::artifact::Artifact;
use fitifact::capability::TransformId;
use fitifact::doctor::DoctorReport;
use fitifact::ffmpeg::FfmpegProvider;
use fitifact::image::ImageProvider;
use fitifact::inspect::DefaultInspector;
use fitifact::runtime::{ExecutionContext, SystemSpawner, TransformProvider};
use fitifact::{
    check, compile_from_json, compile_from_yaml, inspect, load_profile, plan, AdaptRequest,
    AdaptationResult, ConstraintSet, PlanOutcome,
};
use serde::{Deserialize, Serialize};

const HEADER_BYTES: usize = 4096;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSpec {
    pub profile: Option<String>,
    pub constraints_json: Option<String>,
    pub constraints_yaml: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CommandError {
    pub schema: &'static str,
    pub code: String,
    pub message: String,
}

impl CommandError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            schema: "fitifact.error/v1",
            code: code.into(),
            message: message.into(),
        }
    }
}

impl From<fitifact::Error> for CommandError {
    fn from(error: fitifact::Error) -> Self {
        Self::new(error.code.to_string(), error.message)
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for CommandError {}

pub fn open_user_file(path: &str) -> Result<PathBuf, CommandError> {
    let path = PathBuf::from(path);
    let canonical = path.canonicalize().map_err(|_| {
        CommandError::new(
            "INPUT_INVALID",
            format!("file not found: {}", path.display()),
        )
    })?;
    if !canonical.is_file() {
        return Err(CommandError::new(
            "INPUT_INVALID",
            format!("not a file: {}", canonical.display()),
        ));
    }
    Ok(canonical)
}

pub fn load_target(spec: &TargetSpec) -> Result<ConstraintSet, CommandError> {
    if let Some(id) = spec
        .profile
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        return load_profile(id).map_err(CommandError::from);
    }
    if let Some(json) = spec
        .constraints_json
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        return compile_from_json(json).map_err(CommandError::from);
    }
    if let Some(yaml) = spec
        .constraints_yaml
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        return compile_from_yaml(yaml).map_err(CommandError::from);
    }
    Err(CommandError::new(
        "INPUT_INVALID",
        "provide a profile id, constraint JSON, or constraint YAML",
    ))
}

pub fn inspect_path(path: &Path) -> Result<Artifact, CommandError> {
    let inspector = DefaultInspector::default();
    inspect(path, &inspector).map_err(CommandError::from)
}

pub fn check_path(
    path: &Path,
    spec: &TargetSpec,
) -> Result<fitifact::CompatibilityReport, CommandError> {
    let constraints = load_target(spec)?;
    let artifact = inspect_path(path)?;
    Ok(check(&artifact, &constraints))
}

pub fn plan_path(path: &Path, spec: &TargetSpec) -> Result<PlanOutcome, CommandError> {
    let constraints = load_target(spec)?;
    let artifact = inspect_path(path)?;
    Ok(plan(
        &artifact,
        &constraints,
        &fitifact::ffmpeg::catalog_for(&artifact),
    ))
}

#[cfg(test)]
pub fn adapt_path(path: &Path, spec: &TargetSpec) -> Result<AdaptationResult, CommandError> {
    adapt_path_cancellable(path, spec, &fitifact::CancellationToken::default())
}

pub fn adapt_path_cancellable(
    path: &Path,
    spec: &TargetSpec,
    token: &fitifact::CancellationToken,
) -> Result<AdaptationResult, CommandError> {
    token.check()?;
    let constraints = load_target(spec)?;
    let inspector =
        DefaultInspector::new(fitifact::runtime::CancellableSystemSpawner(token.clone()));
    let artifact = inspect(path, &inspector).map_err(CommandError::from)?;
    let catalog = fitifact::ffmpeg::catalog_for(&artifact);
    let outcome = plan(&artifact, &constraints, &catalog);
    let image;
    let ffmpeg;
    let provider: Option<&dyn TransformProvider> = match outcome
        .plan()
        .and_then(|planned| planned.steps.first())
        .map(|step| step.operation)
    {
        Some(TransformId::EncodeJpeg) | Some(TransformId::ImageAdapt) => {
            image = ImageProvider;
            Some(&image)
        }
        Some(_) => {
            ffmpeg =
                FfmpegProvider::new(fitifact::runtime::CancellableSystemSpawner(token.clone()));
            Some(&ffmpeg)
        }
        None => None,
    };
    fitifact::adapt_with_cancellation(
        AdaptRequest {
            input: path,
            constraints,
            output: None,
            catalog: Some(&catalog),
            inspector: &inspector,
            provider,
            execution: ExecutionContext {
                timeout: Duration::from_secs(1800),
                temp_dir: None,
            },
        },
        token,
    )
    .map_err(CommandError::from)
}

pub fn doctor_report() -> DoctorReport {
    let temporary = std::env::temp_dir();
    // A desktop launch directory is not an output destination (Explorer may
    // launch us from a protected directory). Adaptation checks the chosen
    // file's sibling destination when there is an actual output to create.
    let mut report = fitifact::doctor::diagnose(
        &SystemSpawner,
        &temporary,
        &temporary,
        Duration::from_secs(30),
    );
    report
        .workspaces
        .retain(|workspace| workspace.name != "destination");
    report
}

pub fn read_header(path: &Path) -> Result<Vec<u8>, CommandError> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|error| {
        CommandError::new("INPUT_INVALID", format!("cannot open file: {error}"))
    })?;
    let mut buf = vec![0_u8; HEADER_BYTES];
    let read = file.read(&mut buf).map_err(|error| {
        CommandError::new("INPUT_INVALID", format!("cannot read file: {error}"))
    })?;
    buf.truncate(read);
    Ok(buf)
}

pub fn read_limited_image(path: &Path) -> Result<Vec<u8>, CommandError> {
    let header = read_header(path)?;
    if !fitifact::image::looks_like_image(&header) {
        return Err(CommandError::new(
            "INSPECTION_UNSUPPORTED",
            "this command only reads still images for the local image engine",
        ));
    }
    let metadata = std::fs::metadata(path).map_err(|error| {
        CommandError::new("INPUT_INVALID", format!("cannot stat file: {error}"))
    })?;
    if metadata.len() > fitifact::MAX_IMAGE_INPUT_BYTES as u64 {
        return Err(CommandError::new(
            "INSPECTION_LIMIT",
            "this image is larger than Fitifact can process (32 MiB)",
        ));
    }
    std::fs::read(path)
        .map_err(|error| CommandError::new("INPUT_INVALID", format!("cannot read file: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/image")
            .join(name)
    }

    fn jpeg_target() -> TargetSpec {
        let constraints = fitifact::compile(fitifact::ConstraintInput {
            image_format: Some(vec!["jpeg".into()]),
            ..fitifact::ConstraintInput::default()
        })
        .unwrap();
        TargetSpec {
            constraints_json: Some(serde_json::to_string(&constraints).unwrap()),
            ..TargetSpec::default()
        }
    }

    #[test]
    fn inspect_missing_file_is_input_invalid() {
        let error = inspect_path(Path::new("definitely-missing-fitifact-desktop.jpg")).unwrap_err();
        assert_eq!(error.code, "INPUT_INVALID");
        assert_eq!(error.schema, "fitifact.error/v1");
    }

    #[test]
    fn inspect_jpeg_fixture_without_ffmpeg() {
        let artifact = inspect_path(&fixture("compatible-jpeg.jpg")).unwrap();
        assert_eq!(artifact.family.as_str(), "image");
        assert!(artifact.byte_length > 0);
    }

    #[test]
    fn check_jpeg_target_on_compatible_fixture() {
        let report = check_path(&fixture("compatible-jpeg.jpg"), &jpeg_target()).unwrap();
        assert!(report.compatible);
    }

    #[test]
    fn doctor_uses_the_v1_envelope() {
        let report = doctor_report();
        let value = serde_json::to_value(&report).unwrap();
        assert_eq!(value["schema"], "fitifact.doctor/v1");
        assert!(value["tools"].is_array());
        assert!(report
            .workspaces
            .iter()
            .all(|workspace| workspace.name != "destination"));
        assert!(report
            .workspaces
            .iter()
            .any(|workspace| workspace.name == "temporary"));
        let names: Vec<&str> = report.tools.iter().map(|tool| tool.name.as_str()).collect();
        assert!(names.contains(&"ffmpeg"));
        assert!(names.contains(&"ffprobe"));
        if report
            .tools
            .iter()
            .any(|tool| tool.name == "ffmpeg" && !tool.available)
        {
            assert!(!report.healthy);
        }
    }

    #[test]
    fn adapt_png_to_jpeg_writes_fitifact_sibling() {
        let dir = std::env::temp_dir().join(format!("fitifact-desktop-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let input = dir.join("mismatch-png.png");
        std::fs::copy(fixture("mismatch-png.png"), &input).unwrap();
        let result = adapt_path(&input, &jpeg_target()).unwrap();
        assert!(matches!(
            result.status,
            fitifact::AdaptationStatus::Adapted | fitifact::AdaptationStatus::Compatible
        ));
        if let Some(output) = &result.output {
            assert_eq!(output.parent(), input.parent());
            assert!(output
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap()
                .contains(".fitifact."));
            let _ = std::fs::remove_file(output);
        }
        let _ = std::fs::remove_file(&input);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn load_target_requires_one_source() {
        let error = load_target(&TargetSpec::default()).unwrap_err();
        assert_eq!(error.code, "INPUT_INVALID");
    }

    #[test]
    fn cancelled_native_request_never_inspects_or_publishes() {
        let token = fitifact::CancellationToken::default();
        assert!(token.cancel());
        let error = adapt_path_cancellable(
            Path::new("does-not-exist.mov"),
            &TargetSpec::default(),
            &token,
        )
        .unwrap_err();
        assert_eq!(error.code, "EXECUTION_CANCELLED");
    }

    #[test]
    #[ignore = "requires local FFmpeg and ffprobe"]
    fn live_native_cancellation_keeps_truthful_output_and_cleans_staging() {
        assert!(ffmpeg_available());
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "fitifact-native-cancel-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir(&dir).unwrap();
        let input = dir.join("source.mov");
        std::fs::copy(media_fixture("remux-h264-aac.mov"), &input).unwrap();
        let original = std::fs::read(&input).unwrap();
        let token = fitifact::CancellationToken::default();
        let worker = token.clone();
        let path = input.clone();
        let target = TargetSpec {
            constraints_json: Some(
                serde_json::to_string(&fitifact::constraints::media_h264_mp4_aac()).unwrap(),
            ),
            ..TargetSpec::default()
        };
        let job = std::thread::spawn(move || adapt_path_cancellable(&path, &target, &worker));
        let start = std::time::Instant::now();
        while !job.is_finished() && start.elapsed() < Duration::from_secs(10) {
            if std::fs::read_dir(&dir).unwrap().any(|entry| {
                entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".fitifact-stage-")
            }) {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let cancelled = token.cancel();
        let result = job.join().unwrap();
        if cancelled {
            match result {
                Ok(result) => {
                    assert_eq!(
                        result.error.unwrap().code,
                        fitifact::ErrorCode::ExecutionCancelled
                    );
                    assert!(result.output.is_none());
                }
                Err(error) => assert_eq!(error.code, "EXECUTION_CANCELLED"),
            }
        } else {
            let result = result.unwrap();
            assert_eq!(result.status, fitifact::AdaptationStatus::Adapted);
            let output = result.output.unwrap();
            assert!(output.is_file());
            assert_eq!(output.parent(), Some(dir.as_path()));
            assert_eq!(
                result.validation.unwrap().status,
                fitifact::validate::ValidationStatus::Pass
            );
            std::fs::remove_file(output).unwrap();
        }
        assert_eq!(std::fs::read(&input).unwrap(), original);
        assert_eq!(
            std::fs::read_dir(&dir).unwrap().count(),
            1,
            "no temporary output may remain"
        );
        std::fs::remove_file(input).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    fn media_fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/media")
            .join(name)
    }

    fn ffmpeg_available() -> bool {
        let tools = fitifact::resolve_media_tools();
        std::process::Command::new(tools.ffmpeg_program())
            .args(["-hide_banner", "-version"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn adapt_mov_or_doctor_when_ffmpeg_missing() {
        if !ffmpeg_available() {
            let report = doctor_report();
            assert!(!report.healthy);
            return;
        }
        let source = media_fixture("remux-h264-aac.mov");
        assert!(source.is_file(), "canonical MOV fixture must exist");
        let dir = std::env::temp_dir().join(format!("fitifact-desktop-mov-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let input = dir.join("remux-h264-aac.mov");
        std::fs::copy(&source, &input).unwrap();
        let result = adapt_path(
            &input,
            &TargetSpec {
                profile: Some("generic/video-upload".into()),
                ..TargetSpec::default()
            },
        )
        .unwrap();
        assert!(matches!(
            result.status,
            fitifact::AdaptationStatus::Adapted | fitifact::AdaptationStatus::Compatible
        ));
        if let Some(output) = &result.output {
            assert_eq!(output.parent(), input.parent());
            let _ = std::fs::remove_file(output);
        }
        let _ = std::fs::remove_file(&input);
        let _ = std::fs::remove_dir(&dir);
    }
}
