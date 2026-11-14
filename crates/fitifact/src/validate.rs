use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::artifact::{Artifact, DisplayMatrix, Family};
use crate::capability::TransformId;
use crate::check::{CheckResult, CompatibilityReport, check};
use crate::constraints::{ConstraintSet, Field};
use crate::error::{Error, ErrorCode, Result};
use crate::inspect::Inspector;
use crate::media_fit::{floor_video_bitrate_bps, frames_per_second, kept_audio_bps};
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

impl ValidationReport {
    /// Adds an integrity check made outside [`validate_adapted_artifact`] and folds in its status.
    pub fn add_integrity(&mut self, check: ValidationCheck) {
        self.status = combine_status(self.status, check.status);
        self.integrity.push(check);
    }
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
    Ok(validate_adapted_artifact(
        artifact,
        constraints,
        source,
        plan,
        source_hashes,
        output_hashes,
    ))
}

/// Validates a freshly inspected output, wherever it was produced, against the plan's claims.
pub fn validate_adapted_artifact(
    artifact: Artifact,
    constraints: &ConstraintSet,
    source: &Artifact,
    plan: &Plan,
    source_hashes: &StreamHashes,
    output_hashes: &StreamHashes,
) -> ValidationReport {
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
        if source.first_video().is_some() || artifact.first_video().is_some() {
            let preserve_dimensions = plan
                .preserved
                .contains(&crate::plan::PreservationClaim::VideoDimensions);
            let expected_width = plan.steps[0].target.width;
            let expected_height = plan.steps[0].target.height;
            if preserve_dimensions || expected_width.is_none() || expected_height.is_none() {
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
            } else {
                integrity.push(option_equality_check(
                    "width",
                    expected_width,
                    artifact.first_video().and_then(|video| video.width),
                ));
                integrity.push(option_equality_check(
                    "height",
                    expected_height,
                    artifact.first_video().and_then(|video| video.height),
                ));
            }
            // A copy keeps the display matrix; an encode turns the frames upright instead.
            let encode = operation == TransformId::TranscodeVideo;
            let expected_matrix = source
                .first_video()
                .and_then(|video| video.display_matrix)
                .filter(|_| !encode);
            let actual_matrix = artifact
                .first_video()
                .and_then(|video| video.display_matrix);
            integrity.push(simple_check(
                "display_matrix",
                expected_matrix == actual_matrix,
                describe_matrix(expected_matrix),
                describe_matrix(actual_matrix),
            ));
            if encode {
                integrity.push(display_aspect_check(source, &artifact));
                integrity.extend(frame_rate_check(source, plan, &artifact));
            }
        }
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
        if source.first_video().is_some() {
            let video_copied = operation == TransformId::Remux
                || operation == TransformId::TranscodeAudio
                || plan
                    .preserved
                    .contains(&crate::plan::PreservationClaim::AllStreamsCopied);
            let video_equal = source_hashes.video == output_hashes.video;
            provenance.push(provenance_check(
                if video_copied {
                    "video_copied"
                } else {
                    "video_changed"
                },
                "video",
                source_hashes,
                output_hashes,
                if video_copied {
                    video_equal
                } else {
                    !video_equal
                },
                &source_hashes.video,
                &output_hashes.video,
            ));
        }
        if source.first_audio().is_some() {
            let input_audio = source_hashes.audio.as_deref().unwrap_or_default();
            let output_audio = output_hashes.audio.as_deref().unwrap_or_default();
            let audio_copied = operation == TransformId::Remux
                || plan
                    .preserved
                    .contains(&crate::plan::PreservationClaim::AudioStreamCopied)
                || plan
                    .preserved
                    .contains(&crate::plan::PreservationClaim::AllStreamsCopied);
            provenance.push(provenance_check(
                if audio_copied {
                    "audio_copied"
                } else {
                    "audio_changed"
                },
                "audio",
                source_hashes,
                output_hashes,
                if audio_copied {
                    !input_audio.is_empty() && input_audio == output_audio
                } else {
                    input_audio != output_audio
                },
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
    ValidationReport {
        status,
        checks,
        artifact,
        integrity,
        provenance,
    }
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

fn describe_matrix(matrix: Option<DisplayMatrix>) -> String {
    match matrix {
        None => "shown as stored".into(),
        Some(DisplayMatrix {
            rotation: Some(degrees),
            mirrored,
        }) => format!(
            "turned {degrees} degrees clockwise{}",
            if mirrored { ", mirrored" } else { "" }
        ),
        Some(_) => "an unusual transform".into(),
    }
}

/// Rounding the planner may apply to each output edge, in pixels: the first fit rounds by up to
/// 1.5, and each 3/4 step by up to 2 more, which converges below 8.
const EDGE_ROUNDING_PX: f64 = 8.0;

/// An encode must keep the shape of the picture players show: the frame's proportions and its
/// pixels'. FFmpeg's scaler restores a mis-sized frame's shown proportions through the pixel
/// aspect ratio (a portrait frame squashed to 1280x720 comes out 81:256), so checking either one
/// alone lets a distorted file pass.
fn display_aspect_check(source: &Artifact, output: &Artifact) -> ValidationCheck {
    let shape = |artifact: &Artifact| {
        let video = artifact.first_video()?;
        let pixel = video.sample_aspect_ratio.map_or(1.0, |ratio| {
            f64::from(ratio.numerator) / f64::from(ratio.denominator)
        });
        // A quarter turn stands each stored pixel on its side.
        let quarter_turn = matches!(
            video.display_matrix,
            Some(DisplayMatrix {
                rotation: Some(90 | 270),
                ..
            })
        );
        let pixel = if quarter_turn { 1.0 / pixel } else { pixel };
        Some((f64::from(video.width?), f64::from(video.height?), pixel))
    };
    let describe = |shape: Option<(f64, f64, f64)>| {
        shape.map_or_else(
            || "unknown".into(),
            |(width, height, pixel)| format!("{width}x{height} frame, pixel aspect {pixel:.3}"),
        )
    };
    let (expected, actual) = (shape(source), shape(output));
    let status = match (expected, actual) {
        (Some((source_width, source_height, source_pixel)), Some((width, height, pixel))) => {
            let tolerance = EDGE_ROUNDING_PX * (1.0 / width + 1.0 / height);
            let frame = (width / height) / (source_width / source_height);
            if (frame - 1.0).abs() <= tolerance && (pixel / source_pixel - 1.0).abs() <= tolerance {
                ValidationStatus::Pass
            } else {
                ValidationStatus::Fail
            }
        }
        _ => ValidationStatus::Unknown,
    };
    ValidationCheck {
        id: "display_aspect".into(),
        status,
        expected: describe(expected),
        actual: describe(actual),
    }
}

/// How far over the plan's frame rate an output may measure: 2%, plus a frame and a half across
/// the whole clip, because a short clip reads a frame off (30 frames timed over 29/30 s).
const FRAME_RATE_TOLERANCE: f64 = 0.02;
const FRAME_RATE_SLACK_FRAMES: f64 = 1.5;

/// A size-fitted encode sets its video floor from the frame rate the plan counted on. An output
/// with more frames per second than that spreads its bits thinner than the floor allows, however
/// well the bytes fit. The output's rate is its frame count over the clip's length when the
/// container counts frames: a rate measured over the video track alone reads high when an encode
/// shortens a held last frame, though no frame was added. A rate the plan states must be met. A
/// rate the plan only read from the source may turn out higher when the picture still got the
/// floor's bits at its real rate. Fewer frames than planned pass: the floor still holds. `None`
/// when the plan set no floor.
fn frame_rate_check(source: &Artifact, plan: &Plan, output: &Artifact) -> Option<ValidationCheck> {
    let target = &plan.steps[0].target;
    target.video_floor_bps?;
    let source_rate = source.first_video().and_then(|video| video.frame_rate);
    let planned = frames_per_second(target.frame_rate.or(source_rate));
    let video = output.first_video();
    let seconds = output
        .duration_ms
        .filter(|duration| *duration > 0)
        .map(|duration| duration as f64 / 1000.0);
    let counted = video
        .and_then(|video| video.frame_count)
        .zip(seconds)
        .map(|(frames, seconds)| (frames, frames as f64 / seconds));
    let actual = counted.map(|(_, rate)| rate).or_else(|| {
        video
            .and_then(|video| video.frame_rate)
            .filter(|rate| rate.numerator > 0 && rate.denominator > 0)
            .map(|rate| frames_per_second(Some(rate)))
    });
    let status = match (
        actual,
        video.and_then(|video| video.width.zip(video.height)),
    ) {
        (Some(actual), Some((width, height))) => {
            let slack = seconds.map_or(0.0, |seconds| FRAME_RATE_SLACK_FRAMES / seconds);
            let as_planned = actual <= planned * (1.0 + FRAME_RATE_TOLERANCE) + slack;
            let fed = target.frame_rate.is_none()
                && seconds.is_some_and(|seconds| {
                    let sound = kept_audio_bps(output).unwrap_or(0) as f64;
                    output.byte_length as f64 * 8.0 / seconds - sound
                        >= floor_video_bitrate_bps(width, height, actual) as f64
                });
            if as_planned || fed {
                ValidationStatus::Pass
            } else {
                ValidationStatus::Fail
            }
        }
        _ => ValidationStatus::Unknown,
    };
    Some(ValidationCheck {
        id: "frame_rate".into(),
        status,
        expected: format!("at most {planned:.2} frames per second"),
        actual: match (counted, actual) {
            (Some((frames, rate)), _) => format!(
                "{frames} frames over {:.2} s, {rate:.2} per second",
                seconds.unwrap_or_default()
            ),
            (None, Some(rate)) => format!("{rate:.2} frames per second"),
            (None, None) => "unknown".into(),
        },
    })
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
        Field::MediaAudioCodec => artifact
            .first_audio()
            .and_then(|audio| audio.codec.clone())
            .map(ExpectedValue::AudioCodec),
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
    use crate::artifact::{Artifact, AudioCodec, Container, Rational, VideoCodec};
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

    fn video_shaped(
        codec: VideoCodec,
        (width, height): (u32, u32),
        sample_aspect_ratio: Option<(u32, u32)>,
        display_matrix: Option<DisplayMatrix>,
    ) -> Artifact {
        let mut artifact = Artifact::media(Container::Mp4, codec, Some(AudioCodec::Aac), 10);
        let video = artifact.first_video_mut().unwrap();
        (video.width, video.height) = (Some(width), Some(height));
        video.sample_aspect_ratio =
            sample_aspect_ratio.and_then(|(width, height)| Rational::new(width, height));
        video.display_matrix = display_matrix;
        artifact
    }

    const QUARTER_TURN: Option<DisplayMatrix> = Some(DisplayMatrix {
        rotation: Some(90),
        mirrored: false,
    });

    /// The bug this guards: FFmpeg turned a portrait clip upright, scaled it into a landscape
    /// frame planned from the stored size, and kept its shown proportions through the pixels.
    #[test]
    fn an_encode_that_squashes_the_shown_picture_fails() {
        let portrait = video_shaped(VideoCodec::Hevc, (1080, 1920), Some((1, 1)), QUARTER_TURN);
        let shown = |source: &Artifact, size, pixels| {
            display_aspect_check(source, &video_shaped(VideoCodec::H264, size, pixels, None)).status
        };
        assert_eq!(
            shown(&portrait, (720, 1280), Some((1, 1))),
            ValidationStatus::Pass
        );
        assert_eq!(
            shown(&portrait, (1280, 720), Some((81, 256))),
            ValidationStatus::Fail
        );
        // Without a declared pixel shape, the frame alone shows the squash.
        let undeclared = video_shaped(VideoCodec::Hevc, (1080, 1920), None, QUARTER_TURN);
        assert_eq!(
            shown(&undeclared, (1280, 720), None),
            ValidationStatus::Fail
        );
        // A turn ffprobe never reported: the frame matches, the pixels give it away.
        let hidden = video_shaped(VideoCodec::Hevc, (1920, 1080), Some((1, 1)), None);
        assert_eq!(
            shown(&hidden, (1280, 720), Some((81, 256))),
            ValidationStatus::Fail
        );
        // Planner rounding stays within tolerance: 1920x1080 fitted to 1000 wide is 1000x562.
        assert_eq!(
            shown(&hidden, (1000, 562), Some((1124, 1125))),
            ValidationStatus::Pass
        );
    }

    #[test]
    fn a_copy_keeps_the_display_matrix_and_an_encode_drops_it() {
        let matrix_row = |source: &Artifact, output: Artifact| {
            let report = validate_adaptation(
                Path::new("out.mp4"),
                &media_h264_mp4_aac(),
                &FakeInspector(output),
                source,
                &planned(source),
                &StreamHashes::new("video-a", Some("audio-a")),
                &StreamHashes::new("video-a", Some("audio-a")),
            )
            .unwrap();
            let row = report
                .integrity
                .iter()
                .find(|check| check.id == "display_matrix");
            (report.status, row.unwrap().status)
        };
        let mut mov = video_shaped(VideoCodec::H264, (1080, 1920), Some((1, 1)), QUARTER_TURN);
        mov.container = Some(Container::Mov);
        let kept = video_shaped(VideoCodec::H264, (1080, 1920), Some((1, 1)), QUARTER_TURN);
        assert_eq!(
            matrix_row(&mov, kept),
            (ValidationStatus::Pass, ValidationStatus::Pass)
        );
        // A remux that lost the turn shows the stored picture sideways at the same size.
        let lost = video_shaped(VideoCodec::H264, (1080, 1920), Some((1, 1)), None);
        assert_eq!(matrix_row(&mov, lost).1, ValidationStatus::Fail);

        let hevc = video_shaped(VideoCodec::Hevc, (1080, 1920), Some((1, 1)), QUARTER_TURN);
        let still_turned = video_shaped(VideoCodec::H264, (1080, 1920), Some((1, 1)), QUARTER_TURN);
        assert_eq!(matrix_row(&hevc, still_turned).1, ValidationStatus::Fail);
    }

    /// The gap this guards: the floor was a planning rule only, so an encoder that ignored the
    /// planned frame rate passed with half the bits per frame the plan promised.
    #[test]
    fn a_size_fitted_encode_with_more_frames_than_its_floor_counted_on_fails() {
        let limit = |max_bytes| {
            crate::constraints::compile(crate::constraints::ConstraintInput {
                max_bytes: Some(max_bytes),
                ..crate::constraints::ConstraintInput::default()
            })
            .unwrap()
        };
        let clip = |duration_ms, bytes, rate: Option<(u32, u32)>| {
            let mut clip = Artifact::media(
                Container::Mp4,
                VideoCodec::H264,
                Some(AudioCodec::Aac),
                bytes,
            );
            clip.duration_ms = Some(duration_ms);
            clip.first_video_mut().unwrap().frame_rate =
                rate.and_then(|(numerator, denominator)| Rational::new(numerator, denominator));
            clip
        };
        // The plan for `source` under `max_bytes`, and an output of `bytes` at the planned size
        // that reports `rate`, counts `frames`, and states `sound` as its AAC bitrate.
        let row =
            |source: &Artifact, max_bytes, bytes, rate, frames: Option<u64>, sound: Option<u64>| {
                let plan = crate::plan::plan(
                    source,
                    &limit(max_bytes),
                    &crate::capability::default_catalog(),
                )
                .plan()
                .expect("the clip fits")
                .clone();
                let target = &plan.steps[0].target;
                let mut output = clip(source.duration_ms.unwrap(), bytes, rate);
                let video = output.first_video_mut().unwrap();
                (video.width, video.height) = (target.width, target.height);
                video.frame_count = frames;
                output.first_audio_mut().unwrap().bitrate_bps = sound;
                frame_rate_check(source, &plan, &output).unwrap().status
            };
        let reported = |source: &Artifact, max_bytes, bytes, rate| {
            row(source, max_bytes, bytes, rate, None, None)
        };

        // 414.4 s of 1080p60 into 20 MB: the plan halves the frame rate, and says so.
        let recording = clip(414_400, 137_961_438, Some((60, 1)));
        let fitted = |rate| reported(&recording, 20_000_000, 17_170_243, rate);
        assert_eq!(fitted(Some((30, 1))), ValidationStatus::Pass);
        assert_eq!(fitted(Some((60, 1))), ValidationStatus::Fail);
        assert_eq!(fitted(Some((15, 1))), ValidationStatus::Pass);
        assert_eq!(fitted(None), ValidationStatus::Unknown);

        // A stated rate is strict. 60 s of 1080p60 into 21 MB: 60 fps misses the floor even with
        // 96 kbit/s sound, so the plan states 30. An output at 60 fps with 96 kbit/s sound that
        // fills the limit would clear the floor at 60 (2.69 against 2.49 Mbit/s), but it is not
        // the encode that was planned.
        let sixty = clip(60_000, 60_000_000, Some((60, 1)));
        let stated = crate::plan::plan(
            &sixty,
            &limit(21_000_000),
            &crate::capability::default_catalog(),
        );
        assert_eq!(
            stated.steps()[0].target.frame_rate,
            Rational::new(60, 2),
            "{stated:?}"
        );
        assert_eq!(
            row(
                &sixty,
                21_000_000,
                20_900_000,
                Some((60, 1)),
                None,
                Some(96_000)
            ),
            ValidationStatus::Fail
        );
        // The tolerance: 2%, plus 1.5 frames over the clip (30.625 per second over 60 s).
        let counted = |frames| {
            row(
                &sixty,
                21_000_000,
                20_000_000,
                Some((30, 1)),
                Some(frames),
                None,
            )
        };
        assert_eq!(counted(1_837), ValidationStatus::Pass);
        assert_eq!(counted(1_838), ValidationStatus::Fail);

        // One second measures a frame off, which is not a higher rate.
        let short = clip(1_000, 2_000_000, Some((60, 1)));
        assert_eq!(
            reported(&short, 37_000, 36_000, Some((31, 1))),
            ValidationStatus::Pass
        );

        // A source read as 30 fps that comes out at 60: fine while the picture got the floor's
        // bits at 60 (4.64 against 2.49 Mbit/s), a failure when it got those of 30 (1.28).
        let sampled = clip(60_000, 60_000_000, Some((30, 1)));
        let kept = |max_bytes, bytes| reported(&sampled, max_bytes, bytes, Some((60, 1)));
        assert_eq!(kept(40_000_000, 36_000_000), ValidationStatus::Pass);
        assert_eq!(kept(12_000_000, 10_800_000), ValidationStatus::Fail);

        // No readable source rate: the floor assumed 30.
        let unread = clip(60_000, 60_000_000, None);
        assert_eq!(
            reported(&unread, 12_000_000, 10_800_000, Some((60, 1))),
            ValidationStatus::Fail
        );
        assert_eq!(
            reported(&unread, 12_000_000, 10_800_000, Some((30, 1))),
            ValidationStatus::Pass
        );

        // A clip that holds its last frame for 3 of its 6 s, shrunk to 1440x810. FFmpeg's
        // scaler shortens that last frame, so the track's own rate reads 30, but the 90 frames
        // over 6 s are the 15 per second the plan counted on.
        let held = clip(6_000, 6_000_000, Some((15, 1)));
        let shrunk = |frames| row(&held, 438_941, 249_520, Some((30, 1)), frames, None);
        assert_eq!(shrunk(Some(90)), ValidationStatus::Pass);
        assert_eq!(shrunk(None), ValidationStatus::Fail);

        // Without a byte limit there is no floor to hold.
        let hevc = Artifact::media(Container::Mp4, VideoCodec::Hevc, Some(AudioCodec::Aac), 10);
        assert!(frame_rate_check(&hevc, &planned(&hevc), &hevc).is_none());
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
