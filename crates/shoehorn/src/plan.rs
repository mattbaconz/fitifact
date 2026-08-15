use serde::{Deserialize, Serialize};

use crate::artifact::{Artifact, AudioCodec, Container, VideoCodec};
use crate::capability::{Capability, CapabilityCatalog, TransformId};
use crate::check::{CompatibilityReport, check};
use crate::constraints::{ConstraintSet, Field};

const MAX_DEPTH: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepParam {
    pub field: Field,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub transform: TransformId,
    pub params: Vec<StepParam>,
    pub reason: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub steps: Vec<PlanStep>,
    pub preserved: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlanOutcome {
    Compatible,
    Planned { plan: Plan },
    CannotSatisfy { blocking: Vec<String> },
}

impl PlanOutcome {
    pub fn steps(&self) -> &[PlanStep] {
        match self {
            Self::Planned { plan } => &plan.steps,
            _ => &[],
        }
    }
}

#[derive(Clone)]
struct Candidate {
    artifact: Artifact,
    steps: Vec<PlanStep>,
}

impl Candidate {
    fn rank(&self, catalog: &CapabilityCatalog) -> (u32, u32, u32, usize) {
        let mut semantic = 0;
        let mut lossy = 0;
        let mut streams = 0;
        for step in &self.steps {
            if let Some(cap) = catalog.capabilities.iter().find(|c| c.id == step.transform) {
                semantic += cap.semantic_penalty();
                lossy += cap.lossy_penalty();
                streams += cap.streams_changed;
            }
        }
        (semantic, lossy, streams, self.steps.len())
    }
}

pub fn plan(
    artifact: &Artifact,
    constraints: &ConstraintSet,
    catalog: &CapabilityCatalog,
) -> PlanOutcome {
    let initial = check(artifact, constraints);
    if initial.all_pass() {
        return PlanOutcome::Compatible;
    }

    let mut best: Option<Candidate> = None;
    let mut queue = vec![Candidate {
        artifact: artifact.clone(),
        steps: Vec::new(),
    }];

    while let Some(current) = queue.pop() {
        if current.steps.len() >= MAX_DEPTH {
            continue;
        }
        for cap in &catalog.capabilities {
            if !cap.preconditions_met(&current.artifact) {
                continue;
            }
            if current.steps.iter().any(|s| s.transform == cap.id) {
                continue;
            }
            let report = check(&current.artifact, constraints);
            let Some(step) = instantiate(cap, &report, current.steps.len()) else {
                continue;
            };
            let next_artifact = apply(&current.artifact, &step);
            if next_artifact == current.artifact {
                continue;
            }
            let mut next_steps = current.steps.clone();
            next_steps.push(step);
            let next = Candidate {
                artifact: next_artifact,
                steps: next_steps,
            };
            let next_report = check(&next.artifact, constraints);
            if next_report.all_pass() {
                let better = match &best {
                    None => true,
                    Some(existing) => next.rank(catalog) < existing.rank(catalog),
                };
                if better {
                    best = Some(next);
                }
            } else if next.steps.len() < MAX_DEPTH {
                queue.push(next);
            }
        }
    }

    match best {
        Some(winner) => PlanOutcome::Planned {
            plan: Plan {
                preserved: preserved(artifact, constraints, &winner.steps),
                steps: winner.steps,
            },
        },
        None => PlanOutcome::CannotSatisfy {
            blocking: initial.blocking_ids(),
        },
    }
}

fn instantiate(cap: &Capability, report: &CompatibilityReport, index: usize) -> Option<PlanStep> {
    let mut params = Vec::new();
    let mut reason = Vec::new();

    if cap.requires_video_codec_change {
        let video = report.failing_or_unknown(Field::MediaVideoCodec)?;
        params.push(StepParam {
            field: Field::MediaVideoCodec,
            value: first_required(&video.required),
        });
        reason.push(video.constraint_id.clone());
    }

    for field in &cap.can_set {
        if *field == Field::MediaVideoCodec && cap.requires_video_codec_change {
            continue;
        }
        if let Some(check) = report.failing_or_unknown(*field) {
            params.push(StepParam {
                field: *field,
                value: first_required(&check.required),
            });
            reason.push(check.constraint_id.clone());
        }
    }

    if params.is_empty() {
        return None;
    }

    Some(PlanStep {
        id: format!("step-{}", index + 1),
        transform: cap.id,
        params,
        reason,
    })
}

fn first_required(required: &str) -> String {
    required
        .split(',')
        .next()
        .unwrap_or(required)
        .trim()
        .to_string()
}

pub(crate) fn apply(artifact: &Artifact, step: &PlanStep) -> Artifact {
    let mut next = artifact.clone();
    for param in &step.params {
        match param.field {
            Field::MediaContainer => {
                next.container = Some(Container::parse_loose(&param.value));
            }
            Field::MediaVideoCodec => {
                let codec = VideoCodec::parse_loose(&param.value);
                if let Some(video) = &mut next.video {
                    video.codec = Some(codec);
                }
            }
            Field::MediaAudioCodec => {
                let codec = AudioCodec::parse_loose(&param.value);
                if let Some(audio) = &mut next.audio {
                    audio.codec = Some(codec);
                }
            }
            Field::MediaVideoWidth => {
                if let (Some(video), Ok(width)) = (&mut next.video, param.value.parse()) {
                    video.width = Some(width);
                }
            }
            Field::MediaVideoHeight => {
                if let (Some(video), Ok(height)) = (&mut next.video, param.value.parse()) {
                    video.height = Some(height);
                }
            }
            Field::FileBytes => {
                if let Ok(bytes) = param.value.parse() {
                    next.byte_length = bytes;
                }
            }
            Field::FileFamily => {}
        }
    }
    next
}

fn preserved(artifact: &Artifact, constraints: &ConstraintSet, steps: &[PlanStep]) -> Vec<String> {
    let mut out = Vec::new();
    let audio_changed = steps
        .iter()
        .any(|s| s.params.iter().any(|p| p.field == Field::MediaAudioCodec));
    if artifact.audio.is_some() && !audio_changed && constraints.preferences.preserve_audio {
        out.push("media.audio".into());
    }
    let resolution_changed = steps.iter().any(|s| {
        s.params
            .iter()
            .any(|p| p.field == Field::MediaVideoWidth || p.field == Field::MediaVideoHeight)
    });
    if artifact.video.is_some()
        && !resolution_changed
        && constraints.preferences.preserve_resolution
    {
        out.push("media.video.width".into());
        out.push("media.video.height".into());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{AudioCodec, Container, VideoCodec};
    use crate::capability::default_catalog;
    use crate::constraints::{ConstraintInput, compile};

    fn target() -> ConstraintSet {
        crate::constraints::media_h264_mp4_aac()
    }

    fn ids(outcome: &PlanOutcome) -> Vec<TransformId> {
        outcome.steps().iter().map(|s| s.transform).collect()
    }

    #[test]
    fn already_compatible_is_empty_plan() {
        let artifact = Artifact::media(
            Container::Mp4,
            VideoCodec::H264,
            Some(AudioCodec::Aac),
            1000,
        );
        let outcome = plan(&artifact, &target(), &default_catalog());
        assert_eq!(outcome, PlanOutcome::Compatible);
    }

    #[test]
    fn hevc_mp4_selects_video_transcode_only() {
        let artifact = Artifact::media(
            Container::Mp4,
            VideoCodec::Hevc,
            Some(AudioCodec::Aac),
            1000,
        );
        let outcome = plan(&artifact, &target(), &default_catalog());
        assert_eq!(ids(&outcome), vec![TransformId::TranscodeVideo]);
        if let PlanOutcome::Planned { plan } = outcome {
            assert!(plan.preserved.contains(&"media.audio".into()));
            assert!(
                !plan
                    .steps
                    .iter()
                    .any(|s| s.transform.as_str().contains("audio"))
            );
        } else {
            panic!("expected plan");
        }
    }

    #[test]
    fn h264_mov_selects_remux_only() {
        let artifact = Artifact::media(
            Container::Mov,
            VideoCodec::H264,
            Some(AudioCodec::Aac),
            1000,
        );
        let outcome = plan(&artifact, &target(), &default_catalog());
        assert_eq!(ids(&outcome), vec![TransformId::Remux]);
    }

    #[test]
    fn hevc_mov_selects_single_transcode_covering_container() {
        let artifact = Artifact::media(
            Container::Mov,
            VideoCodec::Hevc,
            Some(AudioCodec::Aac),
            1000,
        );
        let outcome = plan(&artifact, &target(), &default_catalog());
        assert_eq!(ids(&outcome), vec![TransformId::TranscodeVideo]);
        let container = outcome.steps()[0]
            .params
            .iter()
            .find(|p| p.field == Field::MediaContainer)
            .unwrap();
        assert_eq!(container.value, "mp4");
    }

    #[test]
    fn remux_beats_lossy_when_remux_suffices() {
        let artifact = Artifact::media(
            Container::Mov,
            VideoCodec::H264,
            Some(AudioCodec::Aac),
            1000,
        );
        let outcome = plan(&artifact, &target(), &default_catalog());
        assert_eq!(ids(&outcome), vec![TransformId::Remux]);
        assert!(!ids(&outcome).contains(&TransformId::TranscodeVideo));
    }

    #[test]
    fn image_family_is_unsatisfiable_for_media_catalog() {
        let artifact = Artifact::image_stub(100);
        let constraints = compile(ConstraintInput {
            family: Some("media".into()),
            video_codec: Some(vec!["h264".into()]),
            ..ConstraintInput::default()
        });
        let outcome = plan(&artifact, &constraints, &default_catalog());
        match outcome {
            PlanOutcome::CannotSatisfy { blocking } => {
                assert!(!blocking.is_empty());
            }
            other => panic!("expected cannot_satisfy, got {other:?}"),
        }
    }

    #[test]
    fn planner_does_not_emit_shell() {
        let artifact = Artifact::media(
            Container::Mp4,
            VideoCodec::Hevc,
            Some(AudioCodec::Aac),
            1000,
        );
        let outcome = plan(&artifact, &target(), &default_catalog());
        let encoded = serde_json::to_string(&outcome).unwrap();
        assert!(!encoded.contains("ffmpeg"));
        assert!(!encoded.contains("-c:v"));
        assert!(!encoded.contains("sh -c"));
    }
}
