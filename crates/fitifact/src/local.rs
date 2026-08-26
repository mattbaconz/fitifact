use serde::Serialize;

use crate::adapt::AdaptationStatus;
use crate::artifact::Artifact;
use crate::capability::{TransformId, default_catalog};
use crate::check::{CompatibilityReport, check};
use crate::constraints::ConstraintSet;
use crate::contract::AdaptationSchema;
use crate::error::{Error, ErrorCode, ErrorEnvelope, Result};
use crate::image::{artifact_from_bytes, encode_jpeg_bytes, looks_like_image};
use crate::plan::{Plan, PlanOutcome, plan};
use crate::report::{Explanation, explain_plan};

/// In-memory image inspect/adapt for the WASM surface and tests.
/// Never constructs or loads a media provider.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LocalImageResult {
    pub schema: AdaptationSchema,
    pub status: AdaptationStatus,
    pub artifact: Artifact,
    pub report: CompatibilityReport,
    pub plan: Option<Plan>,
    pub explanation: Explanation,
    pub error: Option<ErrorEnvelope>,
    pub media_runtime_loaded: bool,
    #[serde(skip)]
    pub output: Option<Vec<u8>>,
}

pub fn inspect_local_bytes(bytes: &[u8]) -> Result<Artifact> {
    if looks_like_image(bytes) {
        return artifact_from_bytes(None, bytes);
    }
    Err(Error::new(
        ErrorCode::InspectionUnsupported,
        "this surface inspects local images only; use the Fitifact CLI for media",
    ))
}

pub fn adapt_local_image_bytes(
    bytes: &[u8],
    constraints: &ConstraintSet,
) -> Result<LocalImageResult> {
    let artifact = inspect_local_bytes(bytes)?;
    let report = check(&artifact, constraints);
    let outcome = plan(&artifact, constraints, &default_catalog());
    let explanation = explain_plan(&artifact, &report, &outcome);
    match outcome {
        PlanOutcome::Compatible { .. } => Ok(LocalImageResult {
            schema: AdaptationSchema,
            status: AdaptationStatus::Compatible,
            artifact,
            report,
            plan: None,
            explanation,
            error: None,
            media_runtime_loaded: false,
            output: None,
        }),
        PlanOutcome::CannotSatisfy { blocking, .. } => {
            let mut envelope = ErrorEnvelope::from(Error::new(
                ErrorCode::NoValidPlan,
                "no acceptable plan satisfies the hard constraints",
            ));
            envelope.details.insert(
                "blocking".into(),
                serde_json::to_value(&blocking).unwrap_or(serde_json::Value::Null),
            );
            Ok(LocalImageResult {
                schema: AdaptationSchema,
                status: AdaptationStatus::CannotSatisfy,
                artifact,
                report,
                plan: None,
                explanation,
                error: Some(envelope),
                media_runtime_loaded: false,
                output: None,
            })
        }
        PlanOutcome::Planned { plan, .. } => {
            if plan.steps.first().map(|step| step.operation) != Some(TransformId::EncodeJpeg) {
                return Ok(failed(
                    artifact,
                    report,
                    Some(plan),
                    explanation,
                    Error::new(
                        ErrorCode::ProviderMissing,
                        "this surface executes only in-process JPEG encode",
                    ),
                ));
            }
            let encoded = match encode_jpeg_bytes(bytes) {
                Ok(bytes) => bytes,
                Err(err) => {
                    return Ok(failed(artifact, report, Some(plan), explanation, err));
                }
            };
            let output_artifact = match artifact_from_bytes(None, &encoded) {
                Ok(artifact) => artifact,
                Err(err) => {
                    return Ok(failed(artifact, report, Some(plan), explanation, err));
                }
            };
            let output_report = check(&output_artifact, constraints);
            let source_size = artifact
                .image
                .as_ref()
                .and_then(|image| image.width.zip(image.height));
            let output_size = output_artifact
                .image
                .as_ref()
                .and_then(|image| image.width.zip(image.height));
            if !output_report.compatible || source_size.is_none() || source_size != output_size {
                return Ok(failed(
                    artifact,
                    report,
                    Some(plan),
                    explanation,
                    Error::new(
                        ErrorCode::ValidationFailed,
                        "the encoded JPEG did not satisfy the target or preserve dimensions",
                    ),
                ));
            }
            Ok(LocalImageResult {
                schema: AdaptationSchema,
                status: AdaptationStatus::Adapted,
                artifact,
                report,
                plan: Some(plan),
                explanation,
                error: None,
                media_runtime_loaded: false,
                output: Some(encoded),
            })
        }
    }
}

fn failed(
    artifact: Artifact,
    report: CompatibilityReport,
    plan: Option<Plan>,
    explanation: Explanation,
    error: Error,
) -> LocalImageResult {
    LocalImageResult {
        schema: AdaptationSchema,
        status: AdaptationStatus::Failed,
        artifact,
        report,
        plan,
        explanation,
        error: Some(ErrorEnvelope::from(error)),
        media_runtime_loaded: false,
        output: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraints::image_jpeg;
    use crate::image::{sample_jpeg_rgb, sample_png_rgb};

    #[test]
    fn jpeg_bytes_are_compatible_without_output() {
        let result = adapt_local_image_bytes(&sample_jpeg_rgb(8, 8), &image_jpeg()).unwrap();
        assert_eq!(result.status, AdaptationStatus::Compatible);
        assert!(result.output.is_none());
        assert!(!result.media_runtime_loaded);
    }

    #[test]
    fn png_bytes_encode_jpeg_without_a_media_runtime() {
        let result = adapt_local_image_bytes(&sample_png_rgb(8, 8), &image_jpeg()).unwrap();
        assert_eq!(result.status, AdaptationStatus::Adapted);
        assert!(!result.media_runtime_loaded);
        let output = result.output.expect("jpeg bytes");
        let artifact = inspect_local_bytes(&output).unwrap();
        assert_eq!(
            artifact.image.unwrap().format,
            Some(crate::artifact::ImageFormat::Jpeg)
        );
    }

    #[test]
    fn media_magic_is_not_inspected_here() {
        let mut bytes = vec![0_u8; 12];
        bytes[4..8].copy_from_slice(b"ftyp");
        bytes[8..12].copy_from_slice(b"isom");
        let err = inspect_local_bytes(&bytes).unwrap_err();
        assert_eq!(err.code, ErrorCode::InspectionUnsupported);
        assert!(err.message.contains("CLI"));
    }
}
