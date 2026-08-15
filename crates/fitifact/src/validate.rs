use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::artifact::Artifact;
use crate::check::{CheckResult, CompatibilityReport, check};
use crate::constraints::ConstraintSet;
use crate::error::Result;
use crate::inspect::Inspector;

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
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{Artifact, AudioCodec, Container, VideoCodec};
    use crate::constraints::media_h264_mp4_aac;
    use crate::error::{Error, ErrorCode};
    use std::path::PathBuf;

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
}
