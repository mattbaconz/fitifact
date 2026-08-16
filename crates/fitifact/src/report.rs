use serde::{Deserialize, Serialize};

use crate::artifact::{Artifact, VideoCodec};
use crate::capability::TransformId;
use crate::check::{CheckResult, CompatibilityReport};
use crate::plan::{Plan, PlanOutcome, PlanStep};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Explanation {
    pub summary: String,
    pub details: Vec<String>,
}

pub fn explain_check(artifact: &Artifact, report: &CompatibilityReport) -> Explanation {
    if report.compatible {
        return Explanation {
            summary: "This file already fits. Nothing needs to change.".into(),
            details: Vec::new(),
        };
    }

    let mut details = Vec::new();
    for check in &report.checks {
        if check.result == CheckResult::Pass {
            continue;
        }
        details.push(mismatch_line(
            artifact,
            check.field.as_str(),
            check.actual.as_deref(),
            &check.required,
            check.result,
        ));
    }

    let summary = video_codec_summary(artifact, report)
        .unwrap_or_else(|| "This file does not meet the target requirements.".into());

    Explanation { summary, details }
}

pub fn explain_plan(
    artifact: &Artifact,
    report: &CompatibilityReport,
    outcome: &PlanOutcome,
) -> Explanation {
    match outcome {
        PlanOutcome::Compatible { .. } => explain_check(artifact, report),
        PlanOutcome::CannotSatisfy { blocking, .. } => Explanation {
            summary: "I can't meet all requirements without breaking your priorities.".into(),
            details: vec![format!(
                "Blocking constraints: {}",
                blocking
                    .iter()
                    .map(|reason| reason.message.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )],
        },
        PlanOutcome::Planned { plan, .. } => {
            let mismatch = mismatch_summary(artifact, report);
            let mut details = Vec::new();
            for step in &plan.steps {
                details.push(step_line(step));
            }
            if !plan.preserved.is_empty() {
                details.push(format!(
                    "Keeping: {}",
                    plan.preserved
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            Explanation {
                summary: format!("{} {}", mismatch, plan_summary(plan)),
                details,
            }
        }
    }
}

fn mismatch_summary(artifact: &Artifact, report: &CompatibilityReport) -> String {
    if let Some(summary) = video_codec_summary(artifact, report) {
        return summary;
    }
    if let Some(container) = report
        .checks
        .iter()
        .find(|c| c.constraint_id == "container" && c.result != CheckResult::Pass)
    {
        let actual = container
            .actual
            .as_deref()
            .unwrap_or("unknown")
            .to_ascii_uppercase();
        let required = container.required.to_ascii_uppercase();
        return format!("Your file is {actual}; this target needs {required}.");
    }
    "This file does not meet the target requirements.".into()
}

fn video_codec_summary(artifact: &Artifact, report: &CompatibilityReport) -> Option<String> {
    let video = report
        .checks
        .iter()
        .find(|c| c.constraint_id == "video-codec")?;
    if video.result == CheckResult::Pass {
        return None;
    }
    let actual = video.actual.as_deref().unwrap_or("unknown");
    let container = artifact
        .container
        .as_ref()
        .map(|c| c.as_str().to_ascii_uppercase())
        .unwrap_or_else(|| "this file".into());
    let required = pretty_codec(&video.required);
    let pretty_actual = pretty_codec(actual);
    Some(format!(
        "Your video is {container}, but it contains {pretty_actual} video. This target needs {required}."
    ))
}

fn pretty_codec(raw: &str) -> String {
    match VideoCodec::parse_loose(raw) {
        VideoCodec::Hevc => "HEVC".into(),
        VideoCodec::H264 => "H.264".into(),
        VideoCodec::Vp9 => "VP9".into(),
        VideoCodec::Av1 => "AV1".into(),
        VideoCodec::Unknown(other) => other,
    }
}

fn mismatch_line(
    _artifact: &Artifact,
    field: &str,
    actual: Option<&str>,
    required: &str,
    result: CheckResult,
) -> String {
    let actual = actual.unwrap_or("unknown");
    match result {
        CheckResult::Unknown => format!("{field}: unknown (required {required})"),
        _ => format!("{field}: actual {actual}, required {required}"),
    }
}

fn plan_summary(plan: &Plan) -> String {
    if plan.steps.len() == 1 && plan.steps[0].operation == TransformId::TranscodeVideo {
        return "I can change only the video stream.".into();
    }
    if plan.steps.len() == 1 && plan.steps[0].operation == TransformId::Remux {
        return "I can remux the container without re-encoding.".into();
    }
    format!("I need to change {} things.", plan.steps.len())
}

fn step_line(step: &PlanStep) -> String {
    match step.operation {
        TransformId::Remux => "Remux to the required container (stream copy).".into(),
        TransformId::TranscodeVideo => "Transcode video; copy audio if present.".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{Artifact, AudioCodec, Container, VideoCodec};
    use crate::capability::default_catalog;
    use crate::check::check;
    use crate::constraints::media_h264_mp4_aac;
    use crate::plan::plan;

    #[test]
    fn hevc_explanation_is_plain_language() {
        let artifact = Artifact::media(
            Container::Mp4,
            VideoCodec::Hevc,
            Some(AudioCodec::Aac),
            1000,
        );
        let constraints = media_h264_mp4_aac();
        let report = check(&artifact, &constraints);
        let outcome = plan(&artifact, &constraints, &default_catalog());
        let explanation = explain_plan(&artifact, &report, &outcome);
        assert!(explanation.summary.contains("HEVC"));
        assert!(explanation.summary.contains("H.264"));
        assert!(
            explanation
                .summary
                .contains("I can change only the video stream.")
                || explanation.details.iter().any(|d| d.contains("video"))
        );
        let keeping = explanation
            .details
            .iter()
            .find(|line| line.starts_with("Keeping: "))
            .expect("transcode explanation should name preserved facts");
        assert!(
            keeping.contains("video dimensions")
                && keeping.contains("video pixel format")
                && keeping.contains("video color metadata")
                && keeping.contains("audio stream copied"),
            "expected readable preservation claims, got {keeping}"
        );
        assert!(
            !keeping.contains("videodimensions") && !keeping.contains("audiostreamcopied"),
            "preservation claims must not dump debug enum names, got {keeping}"
        );
    }

    #[test]
    fn remux_explanation_mentions_container() {
        let artifact = Artifact::media(
            Container::Mov,
            VideoCodec::H264,
            Some(AudioCodec::Aac),
            1000,
        );
        let constraints = media_h264_mp4_aac();
        let report = check(&artifact, &constraints);
        let outcome = plan(&artifact, &constraints, &default_catalog());
        let explanation = explain_plan(&artifact, &report, &outcome);
        assert!(explanation.summary.contains("MOV"));
        assert!(explanation.summary.contains("MP4"));
        assert!(explanation.summary.contains("remux"));
        assert!(
            explanation
                .details
                .iter()
                .any(|line| line == "Keeping: all streams copied"),
            "remux explanation should say streams are copied, got {:?}",
            explanation.details
        );
    }

    #[test]
    fn compatible_explanation_says_nothing_changes() {
        let artifact = Artifact::media(
            Container::Mp4,
            VideoCodec::H264,
            Some(AudioCodec::Aac),
            1000,
        );
        let constraints = media_h264_mp4_aac();
        let report = check(&artifact, &constraints);
        let explanation = explain_check(&artifact, &report);
        assert!(explanation.summary.contains("already fits"));
    }
}
