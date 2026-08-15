use serde::{Deserialize, Serialize};

use crate::artifact::{Artifact, Family};
use crate::constraints::{Constraint, ConstraintSet, ConstraintValue, Field, Operator};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckResult {
    Pass,
    Fail,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstraintCheck {
    pub constraint_id: String,
    pub field: Field,
    pub actual: Option<String>,
    pub required: String,
    pub result: CheckResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityReport {
    pub compatible: bool,
    pub checks: Vec<ConstraintCheck>,
}

impl CompatibilityReport {
    pub fn all_pass(&self) -> bool {
        self.checks.iter().all(|c| c.result == CheckResult::Pass)
    }

    pub fn blocking_ids(&self) -> Vec<String> {
        self.checks
            .iter()
            .filter(|c| c.result != CheckResult::Pass)
            .map(|c| c.constraint_id.clone())
            .collect()
    }

    pub fn failing_or_unknown(&self, field: Field) -> Option<&ConstraintCheck> {
        self.checks
            .iter()
            .find(|c| c.field == field && c.result != CheckResult::Pass)
    }
}

pub fn check(artifact: &Artifact, constraints: &ConstraintSet) -> CompatibilityReport {
    let checks: Vec<ConstraintCheck> = constraints
        .hard
        .iter()
        .map(|constraint| evaluate(artifact, constraint))
        .collect();
    let compatible = checks.iter().all(|c| c.result == CheckResult::Pass);
    CompatibilityReport { compatible, checks }
}

fn evaluate(artifact: &Artifact, constraint: &Constraint) -> ConstraintCheck {
    let required = constraint.value.display();
    let actual = fact(artifact, constraint.field);
    let result = match actual {
        None => CheckResult::Unknown,
        Some(ref value) => match constraint.op {
            Operator::Eq => {
                if matches_eq(constraint.field, value, &constraint.value) {
                    CheckResult::Pass
                } else {
                    CheckResult::Fail
                }
            }
            Operator::In => {
                if matches_in(constraint.field, value, &constraint.value) {
                    CheckResult::Pass
                } else {
                    CheckResult::Fail
                }
            }
            Operator::Lte => match (value.as_int(), constraint.value.as_int()) {
                (Some(actual), Some(limit)) if actual <= limit => CheckResult::Pass,
                (Some(_), Some(_)) => CheckResult::Fail,
                _ => CheckResult::Unknown,
            },
            Operator::Gte => match (value.as_int(), constraint.value.as_int()) {
                (Some(actual), Some(limit)) if actual >= limit => CheckResult::Pass,
                (Some(_), Some(_)) => CheckResult::Fail,
                _ => CheckResult::Unknown,
            },
        },
    };

    ConstraintCheck {
        constraint_id: constraint.id.clone(),
        field: constraint.field,
        actual: actual.map(|v| v.display()),
        required,
        result,
    }
}

enum Fact {
    Int(u64),
    Text(String),
}

impl Fact {
    fn display(&self) -> String {
        match self {
            Self::Int(n) => n.to_string(),
            Self::Text(s) => s.clone(),
        }
    }

    fn as_int(&self) -> Option<u64> {
        match self {
            Self::Int(n) => Some(*n),
            Self::Text(s) => s.parse().ok(),
        }
    }

    fn as_text(&self) -> String {
        match self {
            Self::Int(n) => n.to_string(),
            Self::Text(s) => s.clone(),
        }
    }
}

impl ConstraintValue {
    fn as_int(&self) -> Option<u64> {
        match self {
            Self::Integer(n) => Some(*n),
            Self::Text(s) => s.parse().ok(),
            Self::List(_) => None,
        }
    }
}

fn fact(artifact: &Artifact, field: Field) -> Option<Fact> {
    match field {
        Field::FileBytes => Some(Fact::Int(artifact.byte_length)),
        Field::FileFamily => Some(Fact::Text(artifact.family.as_str().to_string())),
        Field::MediaContainer => artifact
            .container
            .as_ref()
            .map(|c| Fact::Text(c.as_str().to_string())),
        Field::MediaVideoCodec => artifact
            .video
            .as_ref()
            .and_then(|v| v.codec.as_ref())
            .map(|c| Fact::Text(c.as_str().to_string())),
        Field::MediaAudioCodec => artifact
            .audio
            .as_ref()
            .and_then(|a| a.codec.as_ref())
            .map(|c| Fact::Text(c.as_str().to_string())),
        Field::MediaVideoWidth => artifact
            .video
            .as_ref()
            .and_then(|v| v.width)
            .map(|w| Fact::Int(u64::from(w))),
        Field::MediaVideoHeight => artifact
            .video
            .as_ref()
            .and_then(|v| v.height)
            .map(|h| Fact::Int(u64::from(h))),
    }
}

fn matches_eq(field: Field, actual: &Fact, expected: &ConstraintValue) -> bool {
    let expected_text = match expected {
        ConstraintValue::Text(s) => canonicalize(field, s),
        ConstraintValue::Integer(n) => n.to_string(),
        ConstraintValue::List(items) if items.len() == 1 => canonicalize(field, &items[0]),
        ConstraintValue::List(_) => return false,
    };
    canonicalize(field, &actual.as_text()) == expected_text
}

fn matches_in(field: Field, actual: &Fact, expected: &ConstraintValue) -> bool {
    let actual_text = canonicalize(field, &actual.as_text());
    expected
        .as_text_list()
        .iter()
        .any(|item| canonicalize(field, item) == actual_text)
}

fn canonicalize(field: Field, raw: &str) -> String {
    match field {
        Field::MediaContainer => crate::artifact::Container::parse_loose(raw)
            .as_str()
            .to_string(),
        Field::MediaVideoCodec => crate::artifact::VideoCodec::parse_loose(raw)
            .as_str()
            .to_string(),
        Field::MediaAudioCodec => crate::artifact::AudioCodec::parse_loose(raw)
            .as_str()
            .to_string(),
        Field::FileFamily => Family::from_str_loose(raw).as_str().to_string(),
        _ => raw.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{AudioCodec, Container, VideoCodec};
    use crate::constraints::{ConstraintInput, compile};

    fn target() -> ConstraintSet {
        crate::constraints::media_h264_mp4_aac()
    }

    #[test]
    fn compatible_h264_mp4_aac_passes() {
        let artifact = Artifact::media(
            Container::Mp4,
            VideoCodec::H264,
            Some(AudioCodec::Aac),
            1_000,
        );
        let report = check(&artifact, &target());
        assert!(report.compatible);
        assert!(report.all_pass());
    }

    #[test]
    fn hevc_inside_mp4_fails_video_codec_only() {
        let artifact = Artifact::media(
            Container::Mp4,
            VideoCodec::Hevc,
            Some(AudioCodec::Aac),
            1_000,
        );
        let report = check(&artifact, &target());
        assert!(!report.compatible);
        let video = report
            .checks
            .iter()
            .find(|c| c.constraint_id == "video-codec")
            .unwrap();
        assert_eq!(video.result, CheckResult::Fail);
        assert_eq!(video.actual.as_deref(), Some("hevc"));
        let audio = report
            .checks
            .iter()
            .find(|c| c.constraint_id == "audio-codec")
            .unwrap();
        assert_eq!(audio.result, CheckResult::Pass);
    }

    #[test]
    fn missing_video_codec_is_unknown_never_pass() {
        let mut artifact = Artifact::media(
            Container::Mp4,
            VideoCodec::H264,
            Some(AudioCodec::Aac),
            1_000,
        );
        artifact.video.as_mut().unwrap().codec = None;
        let report = check(&artifact, &target());
        assert!(!report.compatible);
        let video = report
            .checks
            .iter()
            .find(|c| c.constraint_id == "video-codec")
            .unwrap();
        assert_eq!(video.result, CheckResult::Unknown);
    }

    #[test]
    fn one_byte_over_max_size_fails() {
        let artifact = Artifact::media(Container::Mp4, VideoCodec::H264, Some(AudioCodec::Aac), 26);
        let constraints = compile(ConstraintInput {
            max_bytes: Some(25),
            ..ConstraintInput::default()
        });
        let report = check(&artifact, &constraints);
        assert_eq!(report.checks[0].result, CheckResult::Fail);
    }

    #[test]
    fn exact_max_bytes_passes() {
        let artifact = Artifact::media(Container::Mp4, VideoCodec::H264, Some(AudioCodec::Aac), 25);
        let constraints = compile(ConstraintInput {
            max_bytes: Some(25),
            ..ConstraintInput::default()
        });
        assert!(check(&artifact, &constraints).compatible);
    }
}
