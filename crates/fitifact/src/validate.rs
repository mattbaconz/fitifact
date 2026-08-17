use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::artifact::{Artifact, Family};
use crate::capability::TransformId;
use crate::check::{CheckResult, CompatibilityReport, check};
use crate::constraints::{ConstraintSet, Field};
use crate::error::{Error, ErrorCode, Result};
use crate::inspect::Inspector;
use crate::plan::{ExpectedValue, Plan};
use crate::runtime::StreamHashes;

pub const DURATION_TOLERANCE_MS: u64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Pass,
    Fail,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub status: ValidationStatus,
    pub checks: CompatibilityReport,
    pub artifact: Artifact,
    pub integrity: Vec<ValidationCheck>,
    pub provenance: Vec<ProvenanceCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationCheck {
    pub id: String,
    pub status: ValidationStatus,
    pub expected: String,
    pub actual: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceCheck {
    pub claim: String,
    pub stream: String,
    pub algorithm: String,
    pub input_hash: String,
    pub output_hash: String,
    pub status: ValidationStatus,
}

pub fn validate(
    path: &Path,
    constraints: &ConstraintSet,
    inspector: &dyn Inspector,
) -> Result<ValidationReport> {
    let artifact = inspector.inspect(path)?;
    let checks = check(&artifact, constraints);
    let status = if checks.checks.iter().any(|c| c.result == CheckResult::Fail) {
        ValidationStatus::Fail
    } else if checks
        .checks
        .iter()
        .any(|c| c.result == CheckResult::Unknown)
    {
        ValidationStatus::Unknown
    } else {
        ValidationStatus::Pass
    };
    Ok(ValidationReport {
        status,
        checks,
        artifact,
        integrity: vec![ValidationCheck {
            id: "parseable".into(),
            status: ValidationStatus::Pass,
            expected: "fresh inspection succeeds".into(),
            actual: "parsed".into(),
        }],
        provenance: Vec::new(),
    })
}

pub fn validate_adaptation(
    path: &Path,
    constraints: &ConstraintSet,
    inspector: &dyn Inspector,
    source: &Artifact,
    plan: &Plan,
    source_hashes: &StreamHashes,
    output_hashes: &StreamHashes,
) -> Result<ValidationReport> {
    let artifact = inspector.inspect(path).map_err(|_| {
        Error::new(
            ErrorCode::ValidationFailed,
            "the staged output could not be freshly inspected",
        )
    })?;
    let checks = check(&artifact, constraints);
    let operation = plan.steps[0].operation;
    let image_job = operation == TransformId::EncodeJpeg || source.family == Family::Image;
    let mut integrity = vec![ValidationCheck {
        id: "parseable".into(),
        status: ValidationStatus::Pass,
        expected: "fresh inspection succeeds".into(),
        actual: "parsed".into(),
    }];
    integrity.push(simple_check(
        "nonzero_output",
        artifact.byte_length > 0,
        "more than zero bytes",
        artifact.byte_length.to_string(),
    ));
    if image_job {
        let source_image = source.image.as_ref();
        let output_image = artifact.image.as_ref();
        integrity.push(option_equality_check(
            "image_width",
            source_image.and_then(|image| image.width),
            output_image.and_then(|image| image.width),
        ));
        integrity.push(option_equality_check(
            "image_height",
            source_image.and_then(|image| image.height),
            output_image.and_then(|image| image.height),
        ));
    } else {
        integrity.push(simple_check(
            "stream_topology",
            stream_topology(source) == stream_topology(&artifact),
            format!("{:?}", stream_topology(source)),
            format!("{:?}", stream_topology(&artifact)),
        ));
        integrity.push(option_equality_check(
            "width",
            source.first_video().and_then(|video| video.width),
            artifact.first_video().and_then(|video| video.width),
        ));
        integrity.push(option_equality_check(
            "height",
            source.first_video().and_then(|video| video.height),
            artifact.first_video().and_then(|video| video.height),
        ));
        integrity.push(duration_check(source.duration_ms, artifact.duration_ms));
    }

    for expected in &plan.steps[0].expected {
        let actual = actual_value(&artifact, expected.field);
        integrity.push(ValidationCheck {
            id: format!("expected_{}", expected.field.as_str().replace('.', "_")),
            status: if actual.as_ref() == Some(&expected.value) {
                ValidationStatus::Pass
            } else if actual.is_none() {
                ValidationStatus::Unknown
            } else {
                ValidationStatus::Fail
            },
            expected: format!("{:?}", expected.value),
            actual: actual
                .map(|value| format!("{value:?}"))
                .unwrap_or_else(|| "unknown".into()),
        });
    }

    let mut provenance = Vec::new();
    if !image_job {
        let video_equal = source_hashes.video == output_hashes.video;
        provenance.push(provenance_check(
            if operation == TransformId::Remux {
                "video_copied"
            } else {
                "video_changed"
            },
            "video",
            source_hashes,
            output_hashes,
            if operation == TransformId::Remux {
                video_equal
            } else {
                !video_equal
            },
            &source_hashes.video,
            &output_hashes.video,
        ));
        if source.first_audio().is_some() {
            let input_audio = source_hashes.audio.as_deref().unwrap_or_default();
            let output_audio = output_hashes.audio.as_deref().unwrap_or_default();
            provenance.push(provenance_check(
                "audio_copied",
                "audio",
                source_hashes,
                output_hashes,
                !input_audio.is_empty() && input_audio == output_audio,
                input_audio,
                output_audio,
            ));
        }
    }

    let mut status = compatibility_status(&checks);
    for check in &integrity {
        status = combine_status(status, check.status);
    }
    for claim in &provenance {
        status = combine_status(status, claim.status);
    }
    Ok(ValidationReport {
        status,
        checks,
        artifact,
        integrity,
        provenance,
    })
}

fn compatibility_status(checks: &CompatibilityReport) -> ValidationStatus {
    if checks
        .checks
        .iter()
        .any(|check| check.result == CheckResult::Fail)
    {
        ValidationStatus::Fail
    } else if checks
        .checks
        .iter()
        .any(|check| check.result == CheckResult::Unknown)
    {
        ValidationStatus::Unknown
    } else {
        ValidationStatus::Pass
    }
}

fn combine_status(left: ValidationStatus, right: ValidationStatus) -> ValidationStatus {
    match (left, right) {
        (ValidationStatus::Fail, _) | (_, ValidationStatus::Fail) => ValidationStatus::Fail,
        (ValidationStatus::Unknown, _) | (_, ValidationStatus::Unknown) => {
            ValidationStatus::Unknown
        }
        _ => ValidationStatus::Pass,
    }
}

fn simple_check(
    id: &str,
    passed: bool,
    expected: impl Into<String>,
    actual: impl Into<String>,
) -> ValidationCheck {
    ValidationCheck {
        id: id.into(),
        status: if passed {
            ValidationStatus::Pass
        } else {
            ValidationStatus::Fail
        },
        expected: expected.into(),
        actual: actual.into(),
    }
}

fn option_equality_check<T: std::fmt::Debug + PartialEq>(
    id: &str,
    expected: Option<T>,
    actual: Option<T>,
) -> ValidationCheck {
    let status = match (&expected, &actual) {
        (Some(expected), Some(actual)) if expected == actual => ValidationStatus::Pass,
        (Some(_), Some(_)) => ValidationStatus::Fail,
        _ => ValidationStatus::Unknown,
    };
    ValidationCheck {
        id: id.into(),
        status,
        expected: expected
            .map(|value| format!("{value:?}"))
            .unwrap_or_else(|| "unknown".into()),
        actual: actual
            .map(|value| format!("{value:?}"))
            .unwrap_or_else(|| "unknown".into()),
    }
}

fn duration_check(expected: Option<u64>, actual: Option<u64>) -> ValidationCheck {
    let status = match (expected, actual) {
        (Some(expected), Some(actual)) if expected.abs_diff(actual) <= DURATION_TOLERANCE_MS => {
            ValidationStatus::Pass
        }
        (Some(_), Some(_)) => ValidationStatus::Fail,
        _ => ValidationStatus::Unknown,
    };
    ValidationCheck {
        id: "duration".into(),
        status,
        expected: expected
            .map(|value| format!("within {DURATION_TOLERANCE_MS} ms of {value} ms"))
            .unwrap_or_else(|| "known source duration".into()),
        actual: actual
            .map(|value| format!("{value} ms"))
            .unwrap_or_else(|| "unknown".into()),
    }
}

fn stream_topology(artifact: &Artifact) -> Vec<&'static str> {
    artifact
        .streams
        .iter()
        .map(|stream| match stream.stream_type() {
            crate::artifact::StreamType::Video => "video",
            crate::artifact::StreamType::Audio => "audio",
            crate::artifact::StreamType::Subtitle => "subtitle",
            crate::artifact::StreamType::Data => "data",
            crate::artifact::StreamType::Attachment => "attachment",
            crate::artifact::StreamType::Unknown(_) => "unknown",
        })
        .collect()
}

fn actual_value(artifact: &Artifact, field: Field) -> Option<ExpectedValue> {
    let video = artifact.first_video();
    match field {
        Field::MediaContainer => artifact.container.clone().map(ExpectedValue::Container),
        Field::MediaVideoCodec => video?.codec.clone().map(ExpectedValue::VideoCodec),
        Field::MediaVideoWidth => video?
            .width
            .map(|value| ExpectedValue::Integer(value.into())),
        Field::MediaVideoHeight => video?
            .height
            .map(|value| ExpectedValue::Integer(value.into())),
        Field::MediaVideoPixelFormat => video?.pixel_format.clone().map(ExpectedValue::Text),
        Field::MediaVideoBitDepth => video?
            .bit_depth
            .map(|value| ExpectedValue::Integer(value.into())),
        Field::MediaVideoColorRange => video?.color_range.clone().map(ExpectedValue::Text),
        Field::MediaVideoColorSpace => video?.color_space.clone().map(ExpectedValue::Text),
        Field::MediaVideoColorTransfer => video?.color_transfer.clone().map(ExpectedValue::Text),
        Field::MediaVideoColorPrimaries => video?.color_primaries.clone().map(ExpectedValue::Text),
        Field::MediaVideoHdr => Some(ExpectedValue::Text(video?.hdr.as_str().into())),
        Field::ImageFormat => artifact
            .image
            .as_ref()
            .and_then(|image| image.format.as_ref())
            .map(|format| ExpectedValue::Text(format.as_str().into())),
        Field::ImageWidth => artifact
            .image
            .as_ref()
            .and_then(|image| image.width)
            .map(|value| ExpectedValue::Integer(value.into())),
        Field::ImageHeight => artifact
            .image
            .as_ref()
            .and_then(|image| image.height)
            .map(|value| ExpectedValue::Integer(value.into())),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn provenance_check(
    claim: &str,
    stream: &str,
    input: &StreamHashes,
    output: &StreamHashes,
    hashes_match_claim: bool,
    input_hash: &str,
    output_hash: &str,
) -> ProvenanceCheck {
    let algorithm_valid = input.algorithm == "sha256" && output.algorithm == "sha256";
    ProvenanceCheck {
        claim: claim.into(),
        stream: stream.into(),
        algorithm: "sha256".into(),
        input_hash: input_hash.into(),
        output_hash: output_hash.into(),
        status: if algorithm_valid && hashes_match_claim {
            ValidationStatus::Pass
        } else {
            ValidationStatus::Fail
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{Artifact, AudioCodec, Container, VideoCodec};
    use crate::constraints::media_h264_mp4_aac;
    use crate::error::{Error, ErrorCode};
    use std::path::PathBuf;

    fn planned(source: &Artifact) -> Plan {
        crate::plan::plan(
            source,
            &media_h264_mp4_aac(),
            &crate::capability::default_catalog(),
        )
        .plan()
        .unwrap()
        .clone()
    }

    struct FakeInspector(Artifact);

    impl Inspector for FakeInspector {
        fn inspect(&self, _path: &Path) -> crate::error::Result<Artifact> {
            Ok(self.0.clone())
        }
    }

    struct Boom;

    impl Inspector for Boom {
        fn inspect(&self, _path: &Path) -> crate::error::Result<Artifact> {
            Err(Error::new(ErrorCode::InputInvalid, "boom"))
        }
    }

    #[test]
    fn validation_pass_uses_same_constraints() {
        let artifact = Artifact::media(Container::Mp4, VideoCodec::H264, Some(AudioCodec::Aac), 10);
        let report = validate(
            Path::new("out.mp4"),
            &media_h264_mp4_aac(),
            &FakeInspector(artifact),
        )
        .unwrap();
        assert_eq!(report.status, ValidationStatus::Pass);
    }

    #[test]
    fn provider_success_with_wrong_codec_is_validation_fail() {
        let artifact = Artifact::media(Container::Mp4, VideoCodec::Hevc, Some(AudioCodec::Aac), 10);
        let report = validate(
            Path::new("out.mp4"),
            &media_h264_mp4_aac(),
            &FakeInspector(artifact),
        )
        .unwrap();
        assert_eq!(report.status, ValidationStatus::Fail);
        assert!(!report.checks.compatible);
    }

    #[test]
    fn inspect_error_is_not_adapted() {
        let err = validate(Path::new("out.mp4"), &media_h264_mp4_aac(), &Boom).unwrap_err();
        assert_eq!(err.code, ErrorCode::InputInvalid);
        let _ = PathBuf::from("out.mp4");
    }

    #[test]
    fn remux_validation_proves_video_and_audio_hashes() {
        let source = Artifact::media(Container::Mov, VideoCodec::H264, Some(AudioCodec::Aac), 10);
        let output = Artifact::media(Container::Mp4, VideoCodec::H264, Some(AudioCodec::Aac), 11);
        let report = validate_adaptation(
            Path::new("out.mp4"),
            &media_h264_mp4_aac(),
            &FakeInspector(output),
            &source,
            &planned(&source),
            &StreamHashes::new("video-a", Some("audio-a")),
            &StreamHashes::new("video-a", Some("audio-a")),
        )
        .unwrap();
        assert_eq!(report.status, ValidationStatus::Pass);
        assert_eq!(report.provenance.len(), 2);
        assert!(
            report
                .provenance
                .iter()
                .all(|claim| claim.status == ValidationStatus::Pass)
        );
    }

    #[test]
    fn remux_hash_mismatch_is_validation_failure() {
        let source = Artifact::media(Container::Mov, VideoCodec::H264, Some(AudioCodec::Aac), 10);
        let output = Artifact::media(Container::Mp4, VideoCodec::H264, Some(AudioCodec::Aac), 11);
        let report = validate_adaptation(
            Path::new("out.mp4"),
            &media_h264_mp4_aac(),
            &FakeInspector(output),
            &source,
            &planned(&source),
            &StreamHashes::new("video-a", Some("audio-a")),
            &StreamHashes::new("video-b", Some("audio-a")),
        )
        .unwrap();
        assert_eq!(report.status, ValidationStatus::Fail);
    }

    #[test]
    fn transcode_proves_audio_preserved_and_video_changed() {
        let source = Artifact::media(Container::Mp4, VideoCodec::Hevc, Some(AudioCodec::Aac), 10);
        let output = Artifact::media(Container::Mp4, VideoCodec::H264, Some(AudioCodec::Aac), 11);
        let report = validate_adaptation(
            Path::new("out.mp4"),
            &media_h264_mp4_aac(),
            &FakeInspector(output),
            &source,
            &planned(&source),
            &StreamHashes::new("video-a", Some("audio-a")),
            &StreamHashes::new("video-b", Some("audio-a")),
        )
        .unwrap();
        assert_eq!(report.status, ValidationStatus::Pass);
        assert!(report.integrity.iter().any(|check| check.id == "duration"));
        assert!(
            report
                .provenance
                .iter()
                .any(|claim| claim.claim == "video_changed")
        );
    }
}
