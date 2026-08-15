use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::artifact::{Artifact, Container};
use crate::capability::{CapabilityCatalog, default_catalog};
use crate::check::{CompatibilityReport, check};
use crate::constraints::ConstraintSet;
use crate::contract::AdaptationSchema;
use crate::error::{Error, ErrorCode, ErrorEnvelope, Result};
use crate::inspect::Inspector;
use crate::plan::{Plan, PlanOutcome, plan};
use crate::report::{Explanation, explain_plan};
use crate::runtime::{ExecutionContext, TransformProvider, execute};
use crate::validate::{ValidationReport, ValidationStatus, validate_adaptation};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdaptationStatus {
    Compatible,
    Adapted,
    CannotSatisfy,
    Failed,
}

pub struct AdaptRequest<'a> {
    pub input: &'a Path,
    pub constraints: ConstraintSet,
    pub output: Option<PathBuf>,
    pub catalog: Option<&'a CapabilityCatalog>,
    pub inspector: &'a dyn Inspector,
    pub provider: &'a dyn TransformProvider,
    pub execution: ExecutionContext,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdaptationResult {
    pub schema: AdaptationSchema,
    pub status: AdaptationStatus,
    pub original: PathBuf,
    pub output: Option<PathBuf>,
    pub artifact: Artifact,
    pub report: CompatibilityReport,
    pub plan: Option<Plan>,
    pub validation: Option<ValidationReport>,
    pub explanation: Explanation,
    pub error: Option<ErrorEnvelope>,
}

pub fn adapt(request: AdaptRequest<'_>) -> Result<AdaptationResult> {
    let original = request.input.to_path_buf();
    let artifact = request.inspector.inspect(request.input)?;
    let report = check(&artifact, &request.constraints);
    let catalog = request.catalog.cloned().unwrap_or_else(default_catalog);
    let outcome = plan(&artifact, &request.constraints, &catalog);
    let explanation = explain_plan(&artifact, &report, &outcome);

    match outcome {
        PlanOutcome::Compatible { .. } => Ok(AdaptationResult {
            schema: AdaptationSchema,
            status: AdaptationStatus::Compatible,
            original,
            output: None,
            artifact,
            report,
            plan: None,
            validation: None,
            explanation,
            error: None,
        }),
        PlanOutcome::CannotSatisfy { blocking, .. } => Ok(AdaptationResult {
            schema: AdaptationSchema,
            status: AdaptationStatus::CannotSatisfy,
            original,
            output: None,
            artifact,
            report,
            plan: None,
            validation: None,
            explanation: Explanation {
                summary: explanation.summary,
                details: {
                    let mut d = explanation.details;
                    if !blocking.is_empty() {
                        d.push(format!(
                            "Blocking: {}",
                            blocking
                                .iter()
                                .map(|reason| reason.message.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                    d
                },
            },
            error: Some({
                let mut envelope = ErrorEnvelope::from(Error::new(
                    ErrorCode::NoValidPlan,
                    "no acceptable plan satisfies the hard constraints",
                ));
                envelope.details.insert(
                    "blocking".into(),
                    serde_json::to_value(&blocking).unwrap_or(serde_json::Value::Null),
                );
                envelope
            }),
        }),
        PlanOutcome::Planned { plan, .. } => {
            let explicit_output = request.output.is_some();
            let output = match request.output {
                Some(path) => path,
                None => default_output(&original, &plan, artifact.container.as_ref()),
            };
            if explicit_output && path_is_occupied(&output) {
                return Ok(failed(
                    original,
                    artifact,
                    report,
                    Some(plan),
                    explanation,
                    Error::new(
                        ErrorCode::SecurityBlocked,
                        "the requested output already exists; choose a new output path",
                    ),
                    None,
                ));
            }
            let mut staged = match StagedOutput::new(&output) {
                Ok(staged) => staged,
                Err(err) => {
                    return Ok(failed(
                        original,
                        artifact,
                        report,
                        Some(plan),
                        explanation,
                        err,
                        None,
                    ));
                }
            };
            if let Err(err) = execute(
                request.provider,
                request.input,
                staged.path(),
                &plan,
                &artifact,
                &request.execution,
            ) {
                return Ok(failed(
                    original,
                    artifact,
                    report,
                    Some(plan),
                    explanation,
                    err,
                    None,
                ));
            }
            let source_hashes =
                match request
                    .provider
                    .stream_hashes(request.input, &artifact, &request.execution)
                {
                    Ok(hashes) => hashes,
                    Err(err) => {
                        return Ok(failed(
                            original,
                            artifact,
                            report,
                            Some(plan),
                            explanation,
                            err,
                            None,
                        ));
                    }
                };
            let output_hashes =
                match request
                    .provider
                    .stream_hashes(staged.path(), &artifact, &request.execution)
                {
                    Ok(hashes) => hashes,
                    Err(err) => {
                        return Ok(failed(
                            original,
                            artifact,
                            report,
                            Some(plan),
                            explanation,
                            err,
                            None,
                        ));
                    }
                };
            match validate_adaptation(
                staged.path(),
                &request.constraints,
                request.inspector,
                &artifact,
                &plan,
                &source_hashes,
                &output_hashes,
            ) {
                Ok(validation) if validation.status == ValidationStatus::Pass => {
                    if let Err(err) = staged.persist(&output) {
                        return Ok(failed(
                            original,
                            artifact,
                            report,
                            Some(plan),
                            explanation,
                            err,
                            Some(validation),
                        ));
                    }
                    Ok(AdaptationResult {
                        schema: AdaptationSchema,
                        status: AdaptationStatus::Adapted,
                        original,
                        output: Some(output),
                        artifact,
                        report,
                        plan: Some(plan),
                        validation: Some(validation),
                        explanation,
                        error: None,
                    })
                }
                Ok(validation) => Ok(failed(
                    original,
                    artifact,
                    report,
                    Some(plan),
                    explanation,
                    Error::new(
                        ErrorCode::ValidationFailed,
                        "output did not satisfy the same hard constraints",
                    ),
                    Some(validation),
                )),
                Err(err) => Ok(failed(
                    original,
                    artifact,
                    report,
                    Some(plan),
                    explanation,
                    err,
                    None,
                )),
            }
        }
    }
}

fn failed(
    original: PathBuf,
    artifact: Artifact,
    report: CompatibilityReport,
    plan: Option<Plan>,
    explanation: Explanation,
    err: Error,
    validation: Option<ValidationReport>,
) -> AdaptationResult {
    AdaptationResult {
        schema: AdaptationSchema,
        status: AdaptationStatus::Failed,
        original,
        output: None,
        artifact,
        report,
        plan,
        validation,
        explanation,
        error: Some(ErrorEnvelope::from(err)),
    }
}

fn default_output(input: &Path, plan: &Plan, current: Option<&Container>) -> PathBuf {
    let container = plan
        .steps
        .iter()
        .rev()
        .find_map(|step| step.target.container.clone())
        .or_else(|| current.cloned())
        .unwrap_or(Container::Mp4);
    let ext = match container {
        Container::Mov => "mov",
        Container::Webm => "webm",
        Container::Mkv => "mkv",
        _ => "mp4",
    };
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    for suffix in 1..=10_000_u32 {
        let name = if suffix == 1 {
            format!("{stem}.fitifact.{ext}")
        } else {
            format!("{stem}.fitifact.{suffix}.{ext}")
        };
        let candidate = parent.join(name);
        if !path_is_occupied(&candidate) {
            return candidate;
        }
    }
    parent.join(format!("{stem}.fitifact.{}.{ext}", std::process::id()))
}

fn path_is_occupied(path: &Path) -> bool {
    match std::fs::symlink_metadata(path) {
        Ok(_) => true,
        Err(error) => error.kind() != std::io::ErrorKind::NotFound,
    }
}

struct StagedOutput {
    path: PathBuf,
    armed: bool,
}

impl StagedOutput {
    fn new(destination: &Path) -> Result<Self> {
        let parent = destination
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let metadata = std::fs::metadata(parent).map_err(|_| {
            Error::new(
                ErrorCode::ExecutionFailed,
                "the output directory is unavailable or not writable",
            )
        })?;
        if !metadata.is_dir() {
            return Err(Error::new(
                ErrorCode::ExecutionFailed,
                "the output destination is not a directory",
            ));
        }
        let stem = destination
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("output");
        let extension = destination
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("mp4");
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        for attempt in 0..100_u32 {
            let path = parent.join(format!(
                ".{stem}.fitifact-stage-{}-{nonce}-{attempt}.{extension}",
                std::process::id()
            ));
            if !path_is_occupied(&path) {
                return Ok(Self { path, armed: true });
            }
        }
        Err(Error::new(
            ErrorCode::ExecutionFailed,
            "could not allocate a unique staging path",
        ))
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn persist(&mut self, destination: &Path) -> Result<()> {
        std::fs::hard_link(&self.path, destination).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                Error::new(
                    ErrorCode::SecurityBlocked,
                    "the output appeared before persistence; no file was overwritten",
                )
            } else {
                Error::new(
                    ErrorCode::ExecutionFailed,
                    "the validated output could not be persisted without overwriting",
                )
            }
        })?;
        if std::fs::remove_file(&self.path).is_err() {
            let _ = std::fs::remove_file(destination);
            return Err(Error::new(
                ErrorCode::ExecutionFailed,
                "the validated output could not be finalized safely",
            ));
        }
        self.armed = false;
        Ok(())
    }
}

impl Drop for StagedOutput {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{AudioCodec, Container, VideoCodec};
    use crate::constraints::media_h264_mp4_aac;
    use crate::inspect::Inspector;
    use crate::plan::Plan;
    use crate::runtime::{ExecutionContext, TransformProvider};
    use std::path::Path;

    fn test_dir(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("fitifact-{label}-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    struct FakeInspector(Artifact);

    impl Inspector for FakeInspector {
        fn inspect(&self, _path: &Path) -> Result<Artifact> {
            Ok(self.0.clone())
        }
    }

    struct PanicProvider;

    impl TransformProvider for PanicProvider {
        fn execute(
            &self,
            _in: &Path,
            _out: &Path,
            _plan: &Plan,
            _ctx: &ExecutionContext,
        ) -> Result<()> {
            panic!("encoder should not start for a compatible file");
        }
    }

    #[test]
    fn compatible_adapt_does_not_execute() {
        let artifact = Artifact::media(Container::Mp4, VideoCodec::H264, Some(AudioCodec::Aac), 10);
        let result = adapt(AdaptRequest {
            input: Path::new("already.mp4"),
            constraints: media_h264_mp4_aac(),
            output: None,
            catalog: None,
            inspector: &FakeInspector(artifact),
            provider: &PanicProvider,
            execution: ExecutionContext::default(),
        })
        .unwrap();
        assert_eq!(result.status, AdaptationStatus::Compatible);
        assert!(result.output.is_none());
        assert!(result.plan.is_none());
    }

    struct FailProvider;

    impl TransformProvider for FailProvider {
        fn execute(
            &self,
            _in: &Path,
            _out: &Path,
            _plan: &Plan,
            _ctx: &ExecutionContext,
        ) -> Result<()> {
            Err(Error::new(
                ErrorCode::ProviderMissing,
                "ffmpeg was not found on PATH",
            ))
        }
    }

    #[test]
    fn missing_provider_is_failed_not_adapted() {
        let artifact = Artifact::media(Container::Mp4, VideoCodec::Hevc, Some(AudioCodec::Aac), 10);
        let result = adapt(AdaptRequest {
            input: Path::new("bad.mp4"),
            constraints: media_h264_mp4_aac(),
            output: Some(PathBuf::from("bad.adapted.mp4")),
            catalog: None,
            inspector: &FakeInspector(artifact),
            provider: &FailProvider,
            execution: ExecutionContext::default(),
        })
        .unwrap();
        assert_eq!(result.status, AdaptationStatus::Failed);
        assert_eq!(
            result.error.as_ref().unwrap().code,
            ErrorCode::ProviderMissing
        );
    }

    struct RoutingInspector {
        input: PathBuf,
        source: Artifact,
        output: Artifact,
    }

    impl Inspector for RoutingInspector {
        fn inspect(&self, path: &Path) -> Result<Artifact> {
            if path == self.input {
                Ok(self.source.clone())
            } else {
                Ok(self.output.clone())
            }
        }
    }

    #[derive(Default)]
    struct PartialFailProvider {
        output: std::sync::Mutex<Option<PathBuf>>,
    }

    impl TransformProvider for PartialFailProvider {
        fn execute(
            &self,
            _in: &Path,
            out: &Path,
            _plan: &Plan,
            _ctx: &ExecutionContext,
        ) -> Result<()> {
            *self.output.lock().unwrap() = Some(out.to_path_buf());
            std::fs::write(out, b"partial").unwrap();
            Err(Error::new(
                ErrorCode::ExecutionFailed,
                "simulated provider failure",
            ))
        }
    }

    struct PassProvider;

    impl TransformProvider for PassProvider {
        fn execute(
            &self,
            _in: &Path,
            out: &Path,
            _plan: &Plan,
            _ctx: &ExecutionContext,
        ) -> Result<()> {
            std::fs::write(out, b"valid-output").unwrap();
            Ok(())
        }

        fn stream_hashes(
            &self,
            _path: &Path,
            _artifact: &Artifact,
            _ctx: &ExecutionContext,
        ) -> Result<crate::runtime::StreamHashes> {
            Ok(crate::runtime::StreamHashes::new(
                "a".repeat(64),
                Some("b".repeat(64)),
            ))
        }
    }

    #[test]
    fn existing_explicit_output_is_refused_before_transform_spawn() {
        let dir = test_dir("existing-output");
        let input = dir.join("input.mov");
        let output = dir.join("occupied.mp4");
        std::fs::write(&input, b"original").unwrap();
        std::fs::write(&output, b"keep-me").unwrap();
        let artifact = Artifact::media(Container::Mov, VideoCodec::H264, Some(AudioCodec::Aac), 8);
        let result = adapt(AdaptRequest {
            input: &input,
            constraints: media_h264_mp4_aac(),
            output: Some(output.clone()),
            catalog: None,
            inspector: &FakeInspector(artifact),
            provider: &PanicProvider,
            execution: ExecutionContext::default(),
        })
        .unwrap();
        assert_eq!(result.status, AdaptationStatus::Failed);
        assert_eq!(
            result.error.as_ref().unwrap().code,
            ErrorCode::SecurityBlocked
        );
        assert_eq!(std::fs::read(&output).unwrap(), b"keep-me");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn provider_failure_cleans_hidden_sibling_stage() {
        let dir = test_dir("stage-cleanup");
        let input = dir.join("input.mov");
        let output = dir.join("requested.mp4");
        std::fs::write(&input, b"original").unwrap();
        let artifact = Artifact::media(Container::Mov, VideoCodec::H264, Some(AudioCodec::Aac), 8);
        let provider = PartialFailProvider::default();
        let result = adapt(AdaptRequest {
            input: &input,
            constraints: media_h264_mp4_aac(),
            output: Some(output.clone()),
            catalog: None,
            inspector: &FakeInspector(artifact),
            provider: &provider,
            execution: ExecutionContext::default(),
        })
        .unwrap();
        assert_eq!(result.status, AdaptationStatus::Failed);
        let staged = provider.output.lock().unwrap().clone().unwrap();
        assert_eq!(staged.parent(), output.parent());
        assert!(
            staged
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with('.')
        );
        assert!(!staged.exists());
        assert!(!output.exists());
        assert_eq!(std::fs::read(&input).unwrap(), b"original");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn default_output_uses_unique_fitifact_sibling_name() {
        let dir = test_dir("default-name");
        let input = dir.join("clip.mov");
        let occupied = dir.join("clip.fitifact.mp4");
        std::fs::write(&input, b"original").unwrap();
        std::fs::write(&occupied, b"occupied").unwrap();
        let source = Artifact::media(Container::Mov, VideoCodec::H264, Some(AudioCodec::Aac), 8);
        let compatible =
            Artifact::media(Container::Mp4, VideoCodec::H264, Some(AudioCodec::Aac), 12);
        let inspector = RoutingInspector {
            input: input.clone(),
            source,
            output: compatible,
        };
        let result = adapt(AdaptRequest {
            input: &input,
            constraints: media_h264_mp4_aac(),
            output: None,
            catalog: None,
            inspector: &inspector,
            provider: &PassProvider,
            execution: ExecutionContext::default(),
        })
        .unwrap();
        assert_eq!(result.status, AdaptationStatus::Adapted);
        assert_eq!(result.output, Some(dir.join("clip.fitifact.2.mp4")));
        assert_eq!(std::fs::read(&occupied).unwrap(), b"occupied");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn validation_mismatch_cleans_stage_and_never_persists_output() {
        let dir = test_dir("validation-cleanup");
        let input = dir.join("clip.mov");
        let output = dir.join("clip.mp4");
        std::fs::write(&input, b"original").unwrap();
        let source = Artifact::media(Container::Mov, VideoCodec::H264, Some(AudioCodec::Aac), 8);
        let wrong = Artifact::media(Container::Mp4, VideoCodec::Hevc, Some(AudioCodec::Aac), 12);
        let inspector = RoutingInspector {
            input: input.clone(),
            source,
            output: wrong,
        };
        let result = adapt(AdaptRequest {
            input: &input,
            constraints: media_h264_mp4_aac(),
            output: Some(output.clone()),
            catalog: None,
            inspector: &inspector,
            provider: &PassProvider,
            execution: ExecutionContext::default(),
        })
        .unwrap();
        assert_eq!(result.status, AdaptationStatus::Failed);
        assert_eq!(
            result.error.as_ref().unwrap().code,
            ErrorCode::ValidationFailed
        );
        assert!(!output.exists());
        assert!(std::fs::read_dir(&dir).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("fitifact-stage")
        }));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
