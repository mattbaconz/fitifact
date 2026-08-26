use std::collections::{HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::artifact::{
    Artifact, AudioCodec, Container, Family, HdrStatus, ImageFormat, StreamType, VideoCodec,
};
use crate::capability::{CapabilityCatalog, TransformId};
use crate::check::{CheckResult, CompatibilityReport, check};
use crate::constraints::{ConstraintSet, ConstraintValue, Field};
pub use crate::contract::{PLAN_SCHEMA, PlanSchema};
use crate::image_adapt::ImageAdaptStepTarget;
use crate::media_fit::{can_fit_media, file_bytes_limit};

pub const PLANNER_VERSION: &str = "0.1.0";
const MAX_DEPTH: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StepTarget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<Container>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_codec: Option<VideoCodec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_format: Option<ImageFormat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageAdaptStepTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanReason {
    pub constraint_id: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExpectedValue {
    Container(Container),
    VideoCodec(VideoCodec),
    AudioCodec(AudioCodec),
    Integer(u64),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedFact {
    pub field: Field,
    pub value: ExpectedValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreservationClaim {
    AllStreamsCopied,
    AudioStreamCopied,
    VideoDimensions,
    VideoPixelFormat,
    VideoColorMetadata,
    ImageDimensions,
}

impl std::fmt::Display for PreservationClaim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::AllStreamsCopied => "all streams copied",
            Self::AudioStreamCopied => "audio stream copied",
            Self::VideoDimensions => "video dimensions",
            Self::VideoPixelFormat => "video pixel format",
            Self::VideoColorMetadata => "video color metadata",
            Self::ImageDimensions => "image dimensions",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub operation: TransformId,
    pub target: StepTarget,
    pub reasons: Vec<PlanReason>,
    pub expected: Vec<ExpectedFact>,
    pub preservation: Vec<PreservationClaim>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub schema: PlanSchema,
    pub planner_version: String,
    pub steps: Vec<PlanStep>,
    pub preserved: Vec<PreservationClaim>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockingCode {
    UnsafeStreamTopology,
    NonMediaUnsupported,
    NonMp4Target,
    UnsupportedVideoTarget,
    UnsupportedAudioTarget,
    AudioTranscodeUnsupported,
    ResizeUnsupported,
    SizeFittingUnsupported,
    UncertainPostTransformSize,
    UnsupportedVideoCodec,
    UnsupportedContainer,
    HdrConversionUnsupported,
    BitDepthConversionUnsupported,
    PixelFormatConversionUnsupported,
    ColorConversionUnsupported,
    UnknownRequiredFact,
    NoProvenPlan,
    UnsupportedImageFormat,
    UnsupportedImageTarget,
    AnimationUnsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockingReason {
    pub code: BlockingCode,
    pub constraint_ids: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlanOutcome {
    Compatible {
        schema: PlanSchema,
        planner_version: String,
        warnings: Vec<String>,
    },
    Planned {
        schema: PlanSchema,
        planner_version: String,
        plan: Plan,
    },
    CannotSatisfy {
        schema: PlanSchema,
        planner_version: String,
        blocking: Vec<BlockingReason>,
    },
}

impl PlanOutcome {
    pub fn steps(&self) -> &[PlanStep] {
        match self {
            Self::Planned { plan, .. } => &plan.steps,
            _ => &[],
        }
    }

    pub fn plan(&self) -> Option<&Plan> {
        match self {
            Self::Planned { plan, .. } => Some(plan),
            _ => None,
        }
    }

    pub fn is_compatible(&self) -> bool {
        matches!(self, Self::Compatible { .. })
    }

    pub fn blocking(&self) -> &[BlockingReason] {
        match self {
            Self::CannotSatisfy { blocking, .. } => blocking,
            _ => &[],
        }
    }

    pub fn blocking_codes(&self) -> Vec<BlockingCode> {
        self.blocking().iter().map(|reason| reason.code).collect()
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
            if let Some(capability) = catalog
                .capabilities
                .iter()
                .find(|capability| capability.id == step.operation)
            {
                semantic += capability.semantic_penalty();
                lossy += capability.lossy_penalty();
                streams += capability.streams_changed;
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
    if let Some(reason) = topology_blocker(artifact) {
        return cannot(vec![reason]);
    }
    if let Some(reason) = target_blocker(constraints) {
        return cannot(vec![reason]);
    }

    let initial = check(artifact, constraints);
    if let Some(reason) = check_only_blocker(&initial) {
        return cannot(vec![reason]);
    }
    if initial.all_pass() {
        return PlanOutcome::Compatible {
            schema: PlanSchema,
            planner_version: PLANNER_VERSION.into(),
            warnings: Vec::new(),
        };
    }
    if let Some(reason) = mutation_blocker(artifact, &initial) {
        return cannot(vec![reason]);
    }
    match file_bytes_limit(constraints) {
        Some(limit) if artifact.byte_length > limit && !can_fit_media(artifact, limit) => {
            return cannot(vec![blocking(
                BlockingCode::SizeFittingUnsupported,
                ids_for(&initial, Field::FileBytes),
                "the size limit is below the quality floor this encoder will use",
            )]);
        }
        _ => {}
    }

    let mut best: Option<Candidate> = None;
    let mut queue = VecDeque::from([Candidate {
        artifact: artifact.clone(),
        steps: Vec::new(),
    }]);
    while let Some(current) = queue.pop_front() {
        if current.steps.len() >= MAX_DEPTH {
            continue;
        }
        let report = check(&current.artifact, constraints);
        for capability in &catalog.capabilities {
            if current
                .steps
                .iter()
                .any(|step| step.operation == capability.id)
                || !capability.preconditions_met(&current.artifact)
            {
                continue;
            }
            let Some(step) = instantiate(
                capability.id,
                &current.artifact,
                &report,
                current.steps.len(),
            ) else {
                continue;
            };
            let next_artifact = apply(&current.artifact, &step);
            let mut next_steps = current.steps.clone();
            next_steps.push(step);
            let next = Candidate {
                artifact: next_artifact,
                steps: next_steps,
            };
            if check(&next.artifact, constraints).all_pass() {
                if best
                    .as_ref()
                    .is_none_or(|existing| next.rank(catalog) < existing.rank(catalog))
                {
                    best = Some(next);
                }
            } else if next.steps.len() < MAX_DEPTH {
                queue.push_back(next);
            }
        }
    }

    match best {
        Some(winner) => {
            let mut preserved = Vec::new();
            for claim in winner
                .steps
                .iter()
                .flat_map(|step| step.preservation.iter().copied())
            {
                if !preserved.contains(&claim) {
                    preserved.push(claim);
                }
            }
            PlanOutcome::Planned {
                schema: PlanSchema,
                planner_version: PLANNER_VERSION.into(),
                plan: Plan {
                    schema: PlanSchema,
                    planner_version: PLANNER_VERSION.into(),
                    steps: winner.steps,
                    preserved,
                    warnings: Vec::new(),
                },
            }
        }
        None => cannot(vec![blocking(
            BlockingCode::NoProvenPlan,
            initial.blocking_ids(),
            "the bounded v0.1 capability catalog has no proven plan",
        )]),
    }
}

fn topology_blocker(artifact: &Artifact) -> Option<BlockingReason> {
    if artifact.family == Family::Image {
        return image_topology_blocker(artifact);
    }
    if artifact.family != Family::Media {
        return Some(blocking(
            BlockingCode::NonMediaUnsupported,
            Vec::new(),
            "v0.1 plans media files only",
        ));
    }
    let videos = artifact.video_streams().count();
    let audios = artifact.audio_streams().count();
    let other = artifact
        .streams
        .iter()
        .any(|stream| !matches!(stream.stream_type(), StreamType::Video | StreamType::Audio));
    if videos != 1 || audios > 1 || other {
        return Some(blocking(
            BlockingCode::UnsafeStreamTopology,
            Vec::new(),
            "v0.1 requires exactly one video, at most one audio, and no other streams",
        ));
    }
    None
}

fn image_topology_blocker(artifact: &Artifact) -> Option<BlockingReason> {
    let image = artifact.image.as_ref()?;
    if image.animated == Some(true) {
        return Some(blocking(
            BlockingCode::AnimationUnsupported,
            Vec::new(),
            "the first image slice refuses animated sources",
        ));
    }
    None
}

fn is_image_target(constraints: &ConstraintSet) -> bool {
    constraints.hard.iter().any(|constraint| {
        constraint.field == Field::ImageFormat
            || (constraint.field == Field::FileFamily
                && constraint.value == ConstraintValue::Text("image".into()))
    })
}

fn is_media_target(constraints: &ConstraintSet) -> bool {
    constraints.hard.iter().any(|constraint| {
        matches!(
            constraint.field,
            Field::MediaContainer | Field::MediaVideoCodec | Field::MediaAudioCodec
        ) || (constraint.field == Field::FileFamily
            && constraint.value == ConstraintValue::Text("media".into()))
    })
}

fn target_blocker(constraints: &ConstraintSet) -> Option<BlockingReason> {
    if is_image_target(constraints) && is_media_target(constraints) {
        return Some(blocking(
            BlockingCode::NonMediaUnsupported,
            Vec::new(),
            "image and media targets cannot be mixed",
        ));
    }
    if is_image_target(constraints) {
        if effective_values(constraints, Field::ImageFormat)
            .is_some_and(|values| !values.contains("jpeg"))
        {
            return Some(blocking(
                BlockingCode::UnsupportedImageTarget,
                constraint_ids(constraints, Field::ImageFormat),
                "the first image slice can produce only JPEG",
            ));
        }
        return None;
    }
    if let Some(constraint) = constraints
        .hard
        .iter()
        .find(|constraint| constraint.field == Field::FileFamily)
        .filter(|constraint| constraint.value != ConstraintValue::Text("media".into()))
    {
        return Some(blocking(
            BlockingCode::NonMediaUnsupported,
            vec![constraint.id.clone()],
            "v0.1 supports only media targets",
        ));
    }
    if effective_values(constraints, Field::MediaContainer)
        .is_some_and(|values| !values.contains("mp4"))
    {
        return Some(blocking(
            BlockingCode::NonMp4Target,
            constraint_ids(constraints, Field::MediaContainer),
            "v0.1 can produce only MP4",
        ));
    }
    if effective_values(constraints, Field::MediaVideoCodec)
        .is_some_and(|values| !values.contains("h264"))
    {
        return Some(blocking(
            BlockingCode::UnsupportedVideoTarget,
            constraint_ids(constraints, Field::MediaVideoCodec),
            "v0.1 can target only H.264 video",
        ));
    }
    if effective_values(constraints, Field::MediaAudioCodec)
        .is_some_and(|values| !values.contains("aac"))
    {
        return Some(blocking(
            BlockingCode::UnsupportedAudioTarget,
            constraint_ids(constraints, Field::MediaAudioCodec),
            "v0.1 can preserve only already-valid AAC audio",
        ));
    }
    None
}

fn effective_values(constraints: &ConstraintSet, field: Field) -> Option<HashSet<String>> {
    let mut matching = constraints
        .hard
        .iter()
        .filter(|constraint| constraint.field == field);
    let first = matching.next()?;
    let mut effective: HashSet<String> = first.value.as_text_list().into_iter().collect();
    for constraint in matching {
        let allowed: HashSet<String> = constraint.value.as_text_list().into_iter().collect();
        effective.retain(|value| allowed.contains(value));
    }
    Some(effective)
}

fn constraint_ids(constraints: &ConstraintSet, field: Field) -> Vec<String> {
    constraints
        .hard
        .iter()
        .filter(|constraint| constraint.field == field)
        .map(|constraint| constraint.id.clone())
        .collect()
}

fn check_only_blocker(report: &CompatibilityReport) -> Option<BlockingReason> {
    for field in [
        Field::MediaVideoWidth,
        Field::MediaVideoHeight,
        Field::ImageWidth,
        Field::ImageHeight,
    ] {
        if let Some(check) = report.failing_or_unknown(field) {
            return Some(blocking(
                if check.result == CheckResult::Unknown {
                    BlockingCode::UnknownRequiredFact
                } else {
                    BlockingCode::ResizeUnsupported
                },
                vec![check.constraint_id.clone()],
                "v0.1 can check dimensions but cannot resize",
            ));
        }
    }
    match report.failing_or_unknown(Field::FileBytes) {
        Some(check) if check.result == CheckResult::Unknown => {
            return Some(blocking(
                BlockingCode::UnknownRequiredFact,
                vec![check.constraint_id.clone()],
                "the encoded size is unknown",
            ));
        }
        _ => {}
    }
    None
}

fn mutation_blocker(artifact: &Artifact, report: &CompatibilityReport) -> Option<BlockingReason> {
    if artifact.family == Family::Image {
        return image_mutation_blocker(artifact, report);
    }
    if let Some(check) = report.failing_or_unknown(Field::MediaAudioCodec) {
        return Some(blocking(
            if check.result == CheckResult::Unknown {
                BlockingCode::UnknownRequiredFact
            } else {
                BlockingCode::AudioTranscodeUnsupported
            },
            vec![check.constraint_id.clone()],
            "v0.1 cannot transcode audio",
        ));
    }
    let audio_supported = artifact
        .first_audio()
        .is_none_or(|audio| audio.codec == Some(AudioCodec::Aac));
    if !audio_supported {
        return Some(blocking(
            BlockingCode::AudioTranscodeUnsupported,
            ids_for(report, Field::MediaAudioCodec),
            "v0.1 operations can copy only AAC audio",
        ));
    }
    let video = artifact.first_video().expect("topology checked");
    if video.width.is_none() || video.height.is_none() || artifact.duration_ms.is_none() {
        return Some(blocking(
            BlockingCode::UnknownRequiredFact,
            Vec::new(),
            "v0.1 cannot execute a transform without known video dimensions and duration",
        ));
    }
    let bytes_fail = report.failing_or_unknown(Field::FileBytes).is_some();
    let codec_fail = report.failing_or_unknown(Field::MediaVideoCodec).is_some();
    if bytes_fail || codec_fail {
        return encode_mutation_blocker(artifact, report);
    }
    if let Some(check) = report
        .failing_or_unknown(Field::MediaContainer)
        .filter(|_| artifact.container != Some(Container::Mov))
    {
        return Some(blocking(
            BlockingCode::UnsupportedContainer,
            vec![check.constraint_id.clone()],
            "the v0.1 remux capability accepts only MOV source containers",
        ));
    }
    None
}

fn encode_mutation_blocker(
    artifact: &Artifact,
    report: &CompatibilityReport,
) -> Option<BlockingReason> {
    let video = artifact.first_video().expect("topology checked");
    if video.codec.is_none() {
        return Some(blocking(
            BlockingCode::UnknownRequiredFact,
            ids_for(report, Field::MediaVideoCodec),
            "the video codec is unknown",
        ));
    }
    let codec_fail = report.failing_or_unknown(Field::MediaVideoCodec).is_some();
    if codec_fail && video.codec != Some(VideoCodec::Hevc) {
        return Some(blocking(
            BlockingCode::UnsupportedVideoCodec,
            ids_for(report, Field::MediaVideoCodec),
            "v0.1 can transcode only HEVC video to H.264",
        ));
    }
    if !codec_fail && video.codec != Some(VideoCodec::H264) && video.codec != Some(VideoCodec::Hevc)
    {
        return Some(blocking(
            BlockingCode::UnsupportedVideoCodec,
            ids_for(report, Field::MediaVideoCodec),
            "v0.1 can re-encode only H.264 or HEVC video to H.264",
        ));
    }
    if !matches!(
        artifact.container,
        Some(Container::Mp4) | Some(Container::Mov)
    ) {
        return Some(blocking(
            BlockingCode::UnsupportedContainer,
            ids_for(report, Field::MediaContainer),
            "the v0.1 video-encode capability accepts only MP4 and MOV source containers",
        ));
    }
    if video.hdr != HdrStatus::Sdr {
        return Some(blocking(
            BlockingCode::HdrConversionUnsupported,
            ids_for(report, Field::MediaVideoCodec),
            "v0.1 does not perform or assume HDR semantic conversion",
        ));
    }
    if video.bit_depth != Some(8) {
        return Some(blocking(
            BlockingCode::BitDepthConversionUnsupported,
            ids_for(report, Field::MediaVideoCodec),
            "v0.1 transcodes only known 8-bit video and does not assume bit-depth conversion",
        ));
    }
    if video.pixel_format.as_deref() != Some("yuv420p") {
        return Some(blocking(
            BlockingCode::PixelFormatConversionUnsupported,
            ids_for(report, Field::MediaVideoCodec),
            "v0.1 transcodes only known yuv420p video and will not force pixel-format conversion",
        ));
    }
    if !approved_sdr_color(video) {
        return Some(blocking(
            BlockingCode::ColorConversionUnsupported,
            ids_for(report, Field::MediaVideoCodec),
            "v0.1 transcodes only known limited-range BT.709 color and will not assume color conversion",
        ));
    }
    None
}

fn image_mutation_blocker(
    artifact: &Artifact,
    report: &CompatibilityReport,
) -> Option<BlockingReason> {
    let image = artifact.image.as_ref();
    let format = image.and_then(|facts| facts.format.clone());
    match format {
        Some(ImageFormat::Jpeg)
        | Some(ImageFormat::Png)
        | Some(ImageFormat::Webp)
        | Some(ImageFormat::Tiff)
        | Some(ImageFormat::Bmp)
        | Some(ImageFormat::Gif) => {}
        Some(_) => {
            return Some(blocking(
                BlockingCode::UnsupportedImageFormat,
                ids_for(report, Field::ImageFormat),
                "the image slice adapts JPEG, PNG, WebP, TIFF, BMP, and GIF",
            ));
        }
        None => {
            return Some(blocking(
                BlockingCode::UnknownRequiredFact,
                ids_for(report, Field::ImageFormat),
                "the image format is unknown",
            ));
        }
    }
    if image.and_then(|facts| facts.width).is_none()
        || image.and_then(|facts| facts.height).is_none()
    {
        return Some(blocking(
            BlockingCode::UnknownRequiredFact,
            Vec::new(),
            "the first image slice cannot encode without known dimensions",
        ));
    }
    None
}

fn approved_sdr_color(video: &crate::artifact::VideoStream) -> bool {
    video.color_range.as_deref() == Some("tv")
        && video.color_space.as_deref() == Some("bt709")
        && video.color_transfer.as_deref() == Some("bt709")
        && video.color_primaries.as_deref() == Some("bt709")
}

fn instantiate(
    operation: TransformId,
    artifact: &Artifact,
    report: &CompatibilityReport,
    index: usize,
) -> Option<PlanStep> {
    match operation {
        TransformId::Remux => {
            let container = report.failing_or_unknown(Field::MediaContainer)?;
            let video = artifact.first_video()?;
            if artifact.container != Some(Container::Mov)
                || video.codec != Some(VideoCodec::H264)
                || video.width.is_none()
                || video.height.is_none()
                || artifact.duration_ms.is_none()
                || report.failing_or_unknown(Field::FileBytes).is_some()
            {
                return None;
            }
            Some(PlanStep {
                id: format!("step-{}", index + 1),
                operation,
                target: StepTarget {
                    container: Some(Container::Mp4),
                    ..StepTarget::default()
                },
                reasons: vec![PlanReason {
                    constraint_id: container.constraint_id.clone(),
                    message: "The target requires an MP4 container.".into(),
                }],
                expected: vec![ExpectedFact {
                    field: Field::MediaContainer,
                    value: ExpectedValue::Container(Container::Mp4),
                }],
                preservation: vec![PreservationClaim::AllStreamsCopied],
                warnings: Vec::new(),
            })
        }
        TransformId::TranscodeVideo => {
            let input_video = artifact.first_video()?;
            let width = input_video.width?;
            let height = input_video.height?;
            let duration_ms = artifact.duration_ms?;
            let codec_fail = report.failing_or_unknown(Field::MediaVideoCodec);
            let bytes_fail = report.failing_or_unknown(Field::FileBytes);
            if codec_fail.is_none() && bytes_fail.is_none() {
                return None;
            }
            if !matches!(
                artifact.container,
                Some(Container::Mp4) | Some(Container::Mov)
            ) {
                return None;
            }
            if codec_fail.is_some() && input_video.codec != Some(VideoCodec::Hevc) {
                return None;
            }
            if codec_fail.is_none()
                && input_video.codec != Some(VideoCodec::H264)
                && input_video.codec != Some(VideoCodec::Hevc)
            {
                return None;
            }
            let max_bytes = file_bytes_from_report(report);
            match max_bytes {
                Some(limit) if artifact.byte_length > limit && !can_fit_media(artifact, limit) => {
                    return None;
                }
                _ => {}
            }
            let mut reasons = Vec::new();
            if let Some(video) = codec_fail {
                reasons.push(PlanReason {
                    constraint_id: video.constraint_id.clone(),
                    message: "The target requires H.264 video.".into(),
                });
            }
            if let Some(container) = report.failing_or_unknown(Field::MediaContainer) {
                reasons.push(PlanReason {
                    constraint_id: container.constraint_id.clone(),
                    message: "The target requires an MP4 container.".into(),
                });
            }
            if let Some(bytes) = bytes_fail {
                reasons.push(PlanReason {
                    constraint_id: bytes.constraint_id.clone(),
                    message: "The target requires a smaller file.".into(),
                });
            }
            if reasons.is_empty() {
                return None;
            }
            let mut preservation = vec![
                PreservationClaim::VideoDimensions,
                PreservationClaim::VideoPixelFormat,
                PreservationClaim::VideoColorMetadata,
            ];
            if artifact.first_audio().is_some() {
                preservation.push(PreservationClaim::AudioStreamCopied);
            }
            Some(PlanStep {
                id: format!("step-{}", index + 1),
                operation,
                target: StepTarget {
                    container: Some(Container::Mp4),
                    video_codec: Some(VideoCodec::H264),
                    max_bytes,
                    duration_ms: max_bytes.map(|_| duration_ms),
                    width: Some(width),
                    height: Some(height),
                    ..StepTarget::default()
                },
                reasons,
                expected: transcode_expected_facts(width, height),
                preservation,
                warnings: Vec::new(),
            })
        }
        TransformId::EncodeJpeg => {
            let format = report.failing_or_unknown(Field::ImageFormat)?;
            let image = artifact.image.as_ref()?;
            let width = image.width?;
            let height = image.height?;
            if image.format != Some(ImageFormat::Png) || image.animated != Some(false) {
                return None;
            }
            Some(PlanStep {
                id: format!("step-{}", index + 1),
                operation,
                target: StepTarget {
                    image_format: Some(ImageFormat::Jpeg),
                    ..StepTarget::default()
                },
                reasons: vec![PlanReason {
                    constraint_id: format.constraint_id.clone(),
                    message: "The target requires a JPEG image.".into(),
                }],
                expected: crate::image::jpeg_expected(width, height),
                preservation: vec![PreservationClaim::ImageDimensions],
                warnings: Vec::new(),
            })
        }
        TransformId::ImageAdapt => None,
    }
}

pub(crate) fn apply(artifact: &Artifact, step: &PlanStep) -> Artifact {
    let mut next = artifact.clone();
    if let Some(container) = &step.target.container {
        next.container = Some(container.clone());
    }
    if let (Some(codec), Some(video)) = (&step.target.video_codec, next.first_video_mut()) {
        video.codec = Some(codec.clone());
    }
    if let (Some(format), Some(image)) = (&step.target.image_format, next.image.as_mut()) {
        image.format = Some(format.clone());
        image.alpha = Some(false);
        image.animated = Some(false);
    }
    if let Some(max_bytes) = step.target.max_bytes {
        next.byte_length = max_bytes;
    }
    next
}

fn file_bytes_from_report(report: &CompatibilityReport) -> Option<u64> {
    report
        .checks
        .iter()
        .find(|check| check.field == Field::FileBytes)
        .and_then(|check| check.required.parse().ok())
}

fn transcode_expected_facts(width: u32, height: u32) -> Vec<ExpectedFact> {
    vec![
        ExpectedFact {
            field: Field::MediaVideoCodec,
            value: ExpectedValue::VideoCodec(VideoCodec::H264),
        },
        ExpectedFact {
            field: Field::MediaContainer,
            value: ExpectedValue::Container(Container::Mp4),
        },
        ExpectedFact {
            field: Field::MediaVideoWidth,
            value: ExpectedValue::Integer(u64::from(width)),
        },
        ExpectedFact {
            field: Field::MediaVideoHeight,
            value: ExpectedValue::Integer(u64::from(height)),
        },
        ExpectedFact {
            field: Field::MediaVideoPixelFormat,
            value: ExpectedValue::Text("yuv420p".into()),
        },
        ExpectedFact {
            field: Field::MediaVideoBitDepth,
            value: ExpectedValue::Integer(8),
        },
        ExpectedFact {
            field: Field::MediaVideoColorRange,
            value: ExpectedValue::Text("tv".into()),
        },
        ExpectedFact {
            field: Field::MediaVideoColorSpace,
            value: ExpectedValue::Text("bt709".into()),
        },
        ExpectedFact {
            field: Field::MediaVideoColorTransfer,
            value: ExpectedValue::Text("bt709".into()),
        },
        ExpectedFact {
            field: Field::MediaVideoColorPrimaries,
            value: ExpectedValue::Text("bt709".into()),
        },
        ExpectedFact {
            field: Field::MediaVideoHdr,
            value: ExpectedValue::Text("sdr".into()),
        },
    ]
}

fn ids_for(report: &CompatibilityReport, field: Field) -> Vec<String> {
    report
        .checks
        .iter()
        .filter(|check| check.field == field)
        .map(|check| check.constraint_id.clone())
        .collect()
}

fn blocking(
    code: BlockingCode,
    constraint_ids: Vec<String>,
    message: impl Into<String>,
) -> BlockingReason {
    BlockingReason {
        code,
        constraint_ids,
        message: message.into(),
    }
}

fn cannot(blocking: Vec<BlockingReason>) -> PlanOutcome {
    PlanOutcome::CannotSatisfy {
        schema: PlanSchema,
        planner_version: PLANNER_VERSION.into(),
        blocking,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{AudioCodec, Container, VideoCodec};
    use crate::capability::default_catalog;
    use crate::constraints::media_h264_mp4_aac;

    #[test]
    fn bounded_search_prefers_lossless_remux() {
        let artifact = Artifact::media(
            Container::Mov,
            VideoCodec::H264,
            Some(AudioCodec::Aac),
            1000,
        );
        let outcome = plan(&artifact, &media_h264_mp4_aac(), &default_catalog());
        assert_eq!(outcome.steps()[0].operation, TransformId::Remux);
    }
}
