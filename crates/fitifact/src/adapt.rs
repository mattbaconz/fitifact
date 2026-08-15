use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::artifact::{Artifact, Container};
use crate::capability::{CapabilityCatalog, default_catalog};
use crate::check::{CompatibilityReport, check};
use crate::constraints::ConstraintSet;
use crate::contract::{AdaptationSchema, ErrorSchema};
use crate::error::{Error, ErrorCode, Result};
use crate::inspect::Inspector;
use crate::plan::{Plan, PlanOutcome, plan};
use crate::report::{Explanation, explain_plan};
use crate::runtime::{ExecutionContext, TransformProvider, execute};
use crate::validate::{ValidationReport, ValidationStatus, validate};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub schema: ErrorSchema,
    pub code: ErrorCode,
    pub message: String,
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
    pub error: Option<ErrorInfo>,
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
            error: Some(ErrorInfo {
                schema: ErrorSchema,
                code: ErrorCode::NoValidPlan,
                message: "no acceptable plan satisfies the hard constraints".into(),
            }),
        }),
        PlanOutcome::Planned { plan, .. } => {
            let output = match request.output {
                Some(path) => path,
                None => default_output(&original, &plan, artifact.container.as_ref()),
            };
            if let Err(err) = execute(
                request.provider,
                request.input,
                &output,
                &plan,
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
            match validate(&output, &request.constraints, request.inspector) {
                Ok(validation) if validation.status == ValidationStatus::Pass => {
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
        error: Some(ErrorInfo {
            schema: ErrorSchema,
            code: err.code,
            message: err.message,
        }),
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
    parent.join(format!("{stem}.adapted.{ext}"))
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
}
