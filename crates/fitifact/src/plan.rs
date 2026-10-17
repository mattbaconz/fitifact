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
use crate::media_fit::{
    can_fit_media_with_limits, dimension_limit, even_dimension, file_bytes_limit, fit_output_size,
};

pub const PLANNER_VERSION: &str = "0.1.0";
const MAX_DEPTH: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StepTarget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<Container>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_codec: Option<VideoCodec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_codec: Option<AudioCodec>,
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

impl PlanStep {
    pub(crate) fn is_audio_only_transcode(&self) -> bool {
        self.operation == TransformId::TranscodeAudio
            && self.target.video_codec.is_none()
            && !self
                .preservation
                .contains(&PreservationClaim::VideoDimensions)
            && self
                .expected
                .iter()
                .all(|fact| fact.field != Field::MediaVideoCodec)
    }
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
    if let Some(reason) = picture_required_blocker(artifact, constraints) {
        return cannot(vec![reason]);
    }
    if let Some(reason) = target_blocker(constraints) {
        return cannot(vec![reason]);
    }

    let initial = check(artifact, constraints);
    if let Some(reason) = check_only_blocker(&initial) {
        return cannot(vec![reason]);
    }
    let max_width = dimension_limit(constraints, Field::MediaVideoWidth);
    let max_height = dimension_limit(constraints, Field::MediaVideoHeight);
    if initial.all_pass() && !needs_playback_normalize(artifact) {
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
        Some(limit)
            if artifact.byte_length > limit
                && !can_fit_media_with_limits(artifact, limit, max_width, max_height) =>
        {
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
            let warnings: Vec<String> = winner
                .steps
                .iter()
                .flat_map(|step| step.warnings.iter().cloned())
                .collect();
            PlanOutcome::Planned {
                schema: PlanSchema,
                planner_version: PLANNER_VERSION.into(),
                plan: Plan {
                    schema: PlanSchema,
                    planner_version: PLANNER_VERSION.into(),
                    steps: winner.steps,
                    preserved,
                    warnings,
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
    if videos == 1 && audios <= 1 && !other {
        return None;
    }
    if videos == 0 && audios == 1 && !other {
        return None;
    }
    Some(blocking(
        BlockingCode::UnsafeStreamTopology,
        Vec::new(),
        "Fitifact can adapt one video (optional sound) or one sound file, and nothing else",
    ))
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

fn picture_required_blocker(
    artifact: &Artifact,
    constraints: &ConstraintSet,
) -> Option<BlockingReason> {
    if artifact.first_video().is_some() {
        return None;
    }
    let ids: Vec<String> = constraints
        .hard
        .iter()
        .filter(|constraint| {
            matches!(
                constraint.field,
                Field::MediaVideoCodec | Field::MediaVideoWidth | Field::MediaVideoHeight
            )
        })
        .map(|constraint| constraint.id.clone())
        .collect();
    if ids.is_empty() {
        return None;
    }
    Some(blocking(
        BlockingCode::UnsupportedVideoTarget,
        ids,
        "this destination is for video",
    ))
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
    for field in [Field::ImageWidth, Field::ImageHeight] {
        if let Some(check) = report.failing_or_unknown(field) {
            return Some(blocking(
                if check.result == CheckResult::Unknown {
                    BlockingCode::UnknownRequiredFact
                } else {
                    BlockingCode::ResizeUnsupported
                },
                vec![check.constraint_id.clone()],
                "the image planner handles still-image sizing; this media planner cannot resize images",
            ));
        }
    }
    for field in [Field::MediaVideoWidth, Field::MediaVideoHeight] {
        if let Some(check) = report
            .failing_or_unknown(field)
            .filter(|check| check.result == CheckResult::Unknown)
        {
            return Some(blocking(
                BlockingCode::UnknownRequiredFact,
                vec![check.constraint_id.clone()],
                "video dimensions are unknown",
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
    if artifact.is_audio_only() {
        return audio_mutation_blocker(artifact, report);
    }
    let video = artifact.first_video().expect("topology checked");
    if video.width.is_none() || video.height.is_none() || artifact.duration_ms.is_none() {
        return Some(blocking(
            BlockingCode::UnknownRequiredFact,
            Vec::new(),
            "cannot execute a transform without known video dimensions and duration",
        ));
    }
    encode_mutation_blocker(artifact, report)
}

fn audio_mutation_blocker(
    artifact: &Artifact,
    report: &CompatibilityReport,
) -> Option<BlockingReason> {
    if artifact.duration_ms.is_none() && report.failing_or_unknown(Field::FileBytes).is_some() {
        return Some(blocking(
            BlockingCode::UnknownRequiredFact,
            ids_for(report, Field::FileBytes),
            "cannot size-fit audio without a known duration",
        ));
    }
    let Some(audio) = artifact.first_audio() else {
        return Some(blocking(
            BlockingCode::UnsafeStreamTopology,
            Vec::new(),
            "Fitifact can adapt one video (optional sound) or one sound file, and nothing else",
        ));
    };
    if !audio
        .codec
        .as_ref()
        .is_some_and(AudioCodec::is_transcodable_source)
    {
        return Some(blocking(
            BlockingCode::UnsupportedAudioTarget,
            ids_for(report, Field::MediaAudioCodec),
            "this sound format cannot be adapted here",
        ));
    }
    if !audio_source_container(artifact.container.as_ref()) {
        return Some(blocking(
            BlockingCode::UnsupportedContainer,
            ids_for(report, Field::MediaContainer),
            "this planner accepts MP4, MOV, WebM, MKV, MP3, WAV, FLAC, and OGG sound sources",
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
    if !transcodable_codec(video.codec.as_ref()) {
        return Some(blocking(
            BlockingCode::UnsupportedVideoCodec,
            ids_for(report, Field::MediaVideoCodec),
            "this planner can transcode H.264, HEVC, VP8, VP9, and AV1 sources to H.264",
        ));
    }
    if !supported_source_container(artifact.container.as_ref()) {
        return Some(blocking(
            BlockingCode::UnsupportedContainer,
            ids_for(report, Field::MediaContainer),
            "this planner accepts MP4, MOV, WebM, and MKV sources",
        ));
    }
    None
}

fn transcodable_codec(codec: Option<&VideoCodec>) -> bool {
    matches!(
        codec,
        Some(VideoCodec::H264)
            | Some(VideoCodec::Hevc)
            | Some(VideoCodec::Vp9)
            | Some(VideoCodec::Vp8)
            | Some(VideoCodec::Av1)
    )
}

fn supported_source_container(container: Option<&Container>) -> bool {
    matches!(
        container,
        Some(Container::Mp4) | Some(Container::Mov) | Some(Container::Webm) | Some(Container::Mkv)
    )
}

fn audio_source_container(container: Option<&Container>) -> bool {
    match container {
        Some(
            Container::Mp4
            | Container::Mov
            | Container::Webm
            | Container::Mkv
            | Container::Mp3
            | Container::Wav
            | Container::Flac
            | Container::Ogg,
        ) => true,
        Some(Container::Unknown(name)) => {
            let name = name.to_ascii_lowercase();
            name == "aac"
                || name == "adts"
                || name == "m4a"
                || name.contains("mp3")
                || name.contains("wav")
                || name.contains("flac")
                || name.contains("ogg")
        }
        None => false,
    }
}

fn remux_source_container(container: Option<&Container>) -> bool {
    matches!(
        container,
        Some(Container::Mov) | Some(Container::Mkv) | Some(Container::Webm)
    )
}

fn copyable_audio(artifact: &Artifact) -> bool {
    artifact
        .first_audio()
        .is_none_or(|audio| audio.codec == Some(AudioCodec::Aac))
}

fn approved_pixel_format(video: &crate::artifact::VideoStream) -> bool {
    video.pixel_format.as_deref() == Some("yuv420p")
}

fn approved_bit_depth(video: &crate::artifact::VideoStream) -> bool {
    video.bit_depth == Some(8)
}

fn known_unplayable_picture(video: &crate::artifact::VideoStream) -> bool {
    video.hdr == HdrStatus::Hdr
        || video.bit_depth.is_some_and(|depth| depth != 8)
        || video
            .pixel_format
            .as_deref()
            .is_some_and(|format| format != "yuv420p")
}

fn needs_playback_normalize(artifact: &Artifact) -> bool {
    artifact.first_video().is_some_and(|video| {
        transcodable_codec(video.codec.as_ref())
            && supported_source_container(artifact.container.as_ref())
            && known_unplayable_picture(video)
    })
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
        TransformId::Remux => instantiate_remux(artifact, report, index),
        TransformId::TranscodeVideo => instantiate_transcode_video(artifact, report, index),
        TransformId::TranscodeAudio => instantiate_transcode_audio(artifact, report, index),
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

fn instantiate_remux(
    artifact: &Artifact,
    report: &CompatibilityReport,
    index: usize,
) -> Option<PlanStep> {
    let container = report.failing_or_unknown(Field::MediaContainer)?;
    let video = artifact.first_video()?;
    if !remux_source_container(artifact.container.as_ref())
        || video.codec != Some(VideoCodec::H264)
        || !copyable_audio(artifact)
        || !approved_pixel_format(video)
        || !approved_bit_depth(video)
        || video.hdr == HdrStatus::Hdr
        || video.width.is_none()
        || video.height.is_none()
        || artifact.duration_ms.is_none()
        || report.failing_or_unknown(Field::FileBytes).is_some()
        || report.failing_or_unknown(Field::MediaVideoCodec).is_some()
        || report.failing_or_unknown(Field::MediaVideoWidth).is_some()
        || report.failing_or_unknown(Field::MediaVideoHeight).is_some()
        || report.failing_or_unknown(Field::MediaAudioCodec).is_some()
        || needs_playback_normalize(artifact)
    {
        return None;
    }
    Some(PlanStep {
        id: format!("step-{}", index + 1),
        operation: TransformId::Remux,
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

fn instantiate_transcode_audio_only(
    artifact: &Artifact,
    report: &CompatibilityReport,
    index: usize,
) -> Option<PlanStep> {
    let audio = artifact.first_audio()?;
    if !audio
        .codec
        .as_ref()
        .is_some_and(AudioCodec::is_transcodable_source)
        || !audio_source_container(artifact.container.as_ref())
    {
        return None;
    }
    let audio_fail = report.failing_or_unknown(Field::MediaAudioCodec);
    let container_fail = report.failing_or_unknown(Field::MediaContainer);
    let bytes_fail = report.failing_or_unknown(Field::FileBytes);
    if audio_fail.is_none() && container_fail.is_none() && bytes_fail.is_none() {
        return None;
    }
    let max_bytes = bytes_fail.and_then(|_| file_bytes_from_report(report));
    if bytes_fail.is_some() {
        let duration = artifact.duration_ms?;
        let limit = max_bytes?;
        if !crate::media_fit::can_fit_audio(duration, limit) {
            return None;
        }
    }
    let mut reasons = Vec::new();
    if let Some(check) = bytes_fail {
        reasons.push(PlanReason {
            constraint_id: check.constraint_id.clone(),
            message: "The file is over the size limit.".into(),
        });
    }
    if let Some(check) = audio_fail {
        reasons.push(PlanReason {
            constraint_id: check.constraint_id.clone(),
            message: "The target requires AAC audio.".into(),
        });
    }
    if let Some(check) = container_fail {
        reasons.push(PlanReason {
            constraint_id: check.constraint_id.clone(),
            message: "The target requires an MP4 container.".into(),
        });
    }
    if reasons.is_empty() {
        return None;
    }
    Some(PlanStep {
        id: format!("step-{}", index + 1),
        operation: TransformId::TranscodeAudio,
        target: StepTarget {
            container: Some(Container::Mp4),
            audio_codec: Some(AudioCodec::Aac),
            max_bytes,
            duration_ms: max_bytes.and(artifact.duration_ms),
            ..StepTarget::default()
        },
        reasons,
        expected: vec![
            ExpectedFact {
                field: Field::MediaContainer,
                value: ExpectedValue::Container(Container::Mp4),
            },
            ExpectedFact {
                field: Field::MediaAudioCodec,
                value: ExpectedValue::AudioCodec(AudioCodec::Aac),
            },
        ],
        preservation: Vec::new(),
        warnings: Vec::new(),
    })
}

fn instantiate_transcode_audio(
    artifact: &Artifact,
    report: &CompatibilityReport,
    index: usize,
) -> Option<PlanStep> {
    if artifact.is_audio_only() {
        return instantiate_transcode_audio_only(artifact, report, index);
    }
    let video = artifact.first_video()?;
    let audio_fail = report.failing_or_unknown(Field::MediaAudioCodec)?;
    if artifact.first_audio().is_none()
        || video.codec != Some(VideoCodec::H264)
        || !supported_source_container(artifact.container.as_ref())
        || video.width.is_none()
        || video.height.is_none()
        || artifact.duration_ms.is_none()
        || report.failing_or_unknown(Field::MediaVideoCodec).is_some()
        || report.failing_or_unknown(Field::FileBytes).is_some()
        || report.failing_or_unknown(Field::MediaVideoWidth).is_some()
        || report.failing_or_unknown(Field::MediaVideoHeight).is_some()
        || needs_playback_normalize(artifact)
        || !matches!(
            artifact.container,
            Some(Container::Mp4)
                | Some(Container::Mov)
                | Some(Container::Mkv)
                | Some(Container::Webm)
        )
    {
        return None;
    }
    let mut reasons = vec![PlanReason {
        constraint_id: audio_fail.constraint_id.clone(),
        message: "The target requires AAC audio.".into(),
    }];
    if let Some(container) = report.failing_or_unknown(Field::MediaContainer) {
        reasons.push(PlanReason {
            constraint_id: container.constraint_id.clone(),
            message: "The target requires an MP4 container.".into(),
        });
    }
    Some(PlanStep {
        id: format!("step-{}", index + 1),
        operation: TransformId::TranscodeAudio,
        target: StepTarget {
            container: Some(Container::Mp4),
            audio_codec: Some(AudioCodec::Aac),
            width: video.width,
            height: video.height,
            ..StepTarget::default()
        },
        reasons,
        expected: vec![
            ExpectedFact {
                field: Field::MediaContainer,
                value: ExpectedValue::Container(Container::Mp4),
            },
            ExpectedFact {
                field: Field::MediaAudioCodec,
                value: ExpectedValue::AudioCodec(AudioCodec::Aac),
            },
            ExpectedFact {
                field: Field::MediaVideoCodec,
                value: ExpectedValue::VideoCodec(VideoCodec::H264),
            },
        ],
        preservation: vec![
            PreservationClaim::VideoDimensions,
            PreservationClaim::VideoPixelFormat,
            PreservationClaim::VideoColorMetadata,
        ],
        warnings: Vec::new(),
    })
}

fn instantiate_transcode_video(
    artifact: &Artifact,
    report: &CompatibilityReport,
    index: usize,
) -> Option<PlanStep> {
    let input_video = artifact.first_video()?;
    let source_width = input_video.width?;
    let source_height = input_video.height?;
    let duration_ms = artifact.duration_ms?;
    if !supported_source_container(artifact.container.as_ref())
        || !transcodable_codec(input_video.codec.as_ref())
    {
        return None;
    }

    let codec_fail = report.failing_or_unknown(Field::MediaVideoCodec);
    let bytes_fail = report.failing_or_unknown(Field::FileBytes);
    let width_fail = report.failing_or_unknown(Field::MediaVideoWidth);
    let height_fail = report.failing_or_unknown(Field::MediaVideoHeight);
    let audio_fail = report.failing_or_unknown(Field::MediaAudioCodec);
    let container_fail = report.failing_or_unknown(Field::MediaContainer);
    let max_bytes = file_bytes_from_report(report);
    let max_width = report
        .checks
        .iter()
        .filter(|check| check.field == Field::MediaVideoWidth)
        .filter_map(|check| check.required.parse().ok())
        .min();
    let max_height = report
        .checks
        .iter()
        .filter(|check| check.field == Field::MediaVideoHeight)
        .filter_map(|check| check.required.parse().ok())
        .min();

    let need_video_encode = codec_fail.is_some()
        || bytes_fail.is_some()
        || width_fail.is_some()
        || height_fail.is_some()
        || needs_playback_normalize(artifact)
        || (container_fail.is_some() && !can_remux_instead(artifact, report));
    if !need_video_encode {
        return None;
    }

    let fitted = fit_output_size(
        source_width,
        source_height,
        max_width,
        max_height,
        max_bytes,
        Some(duration_ms),
        artifact.first_audio().is_some(),
    )?;
    let width = even_dimension(fitted.0);
    let height = even_dimension(fitted.1);
    if max_bytes.is_some_and(|limit| {
        artifact.byte_length > limit
            && !crate::media_fit::can_fit_at(
                width,
                height,
                limit,
                duration_ms,
                artifact.first_audio().is_some(),
            )
    }) {
        return None;
    }

    let mut reasons = Vec::new();
    if let Some(video) = codec_fail {
        reasons.push(PlanReason {
            constraint_id: video.constraint_id.clone(),
            message: "The target requires H.264 video.".into(),
        });
    }
    if let Some(container) = container_fail {
        reasons.push(PlanReason {
            constraint_id: container.constraint_id.clone(),
            message: "The target requires an MP4 container.".into(),
        });
    }
    if let Some(audio) = audio_fail {
        reasons.push(PlanReason {
            constraint_id: audio.constraint_id.clone(),
            message: "The target requires AAC audio.".into(),
        });
    }
    if let Some(bytes) = bytes_fail {
        reasons.push(PlanReason {
            constraint_id: bytes.constraint_id.clone(),
            message: "The target requires a smaller file.".into(),
        });
    }
    if let Some(width_check) = width_fail {
        reasons.push(PlanReason {
            constraint_id: width_check.constraint_id.clone(),
            message: "The target has a maximum width.".into(),
        });
    }
    if let Some(height_check) = height_fail {
        reasons.push(PlanReason {
            constraint_id: height_check.constraint_id.clone(),
            message: "The target has a maximum height.".into(),
        });
    }
    if reasons.is_empty() && needs_playback_normalize(artifact) {
        reasons.push(PlanReason {
            constraint_id: "playback-normalize".into(),
            message: "This file needs a compatible 8-bit H.264 picture.".into(),
        });
    }
    if reasons.is_empty() {
        return None;
    }

    let resized = width != source_width || height != source_height;
    let copy_audio = copyable_audio(artifact);
    let mut preservation = Vec::new();
    if !resized {
        preservation.push(PreservationClaim::VideoDimensions);
    }
    if approved_pixel_format(input_video) && approved_bit_depth(input_video) {
        preservation.push(PreservationClaim::VideoPixelFormat);
    }
    if approved_sdr_color(input_video) && input_video.hdr == HdrStatus::Sdr {
        preservation.push(PreservationClaim::VideoColorMetadata);
    }
    if copy_audio && artifact.first_audio().is_some() {
        preservation.push(PreservationClaim::AudioStreamCopied);
    }

    let mut warnings = Vec::new();
    if input_video.hdr != HdrStatus::Sdr {
        warnings.push("HDR was converted to SDR. Colors may look different.".into());
    }
    if !approved_bit_depth(input_video) {
        warnings.push("Bit depth was converted to 8-bit.".into());
    }
    if !approved_pixel_format(input_video) {
        warnings.push("The picture format was converted to yuv420p.".into());
    }
    if !approved_sdr_color(input_video) && input_video.hdr == HdrStatus::Sdr {
        warnings.push("Color metadata was converted to BT.709.".into());
    }
    if resized {
        warnings.push(format!("Resized to {width}x{height} so the file can fit."));
    }

    Some(PlanStep {
        id: format!("step-{}", index + 1),
        operation: TransformId::TranscodeVideo,
        target: StepTarget {
            container: Some(Container::Mp4),
            video_codec: Some(VideoCodec::H264),
            audio_codec: if copy_audio {
                None
            } else {
                Some(AudioCodec::Aac)
            },
            max_bytes,
            duration_ms: max_bytes.map(|_| duration_ms),
            width: Some(width),
            height: Some(height),
            ..StepTarget::default()
        },
        reasons,
        expected: transcode_expected_facts(width, height, artifact.first_audio().is_some()),
        preservation,
        warnings,
    })
}

fn can_remux_instead(artifact: &Artifact, report: &CompatibilityReport) -> bool {
    remux_source_container(artifact.container.as_ref())
        && artifact
            .first_video()
            .is_some_and(|video| video.codec == Some(VideoCodec::H264))
        && copyable_audio(artifact)
        && !needs_playback_normalize(artifact)
        && report.failing_or_unknown(Field::MediaVideoCodec).is_none()
        && report.failing_or_unknown(Field::FileBytes).is_none()
        && report.failing_or_unknown(Field::MediaVideoWidth).is_none()
        && report.failing_or_unknown(Field::MediaVideoHeight).is_none()
        && report.failing_or_unknown(Field::MediaAudioCodec).is_none()
}

pub(crate) fn apply(artifact: &Artifact, step: &PlanStep) -> Artifact {
    let mut next = artifact.clone();
    if let Some(container) = &step.target.container {
        next.container = Some(container.clone());
    }
    if let Some(max_bytes) = step.target.max_bytes {
        next.byte_length = max_bytes;
    }
    match step.operation {
        TransformId::TranscodeVideo => {
            if let Some(video) = next.first_video_mut() {
                if let Some(codec) = &step.target.video_codec {
                    video.codec = Some(codec.clone());
                }
                if let Some(width) = step.target.width {
                    video.width = Some(width);
                }
                if let Some(height) = step.target.height {
                    video.height = Some(height);
                }
                video.pixel_format = Some("yuv420p".into());
                video.bit_depth = Some(8);
                video.color_range = Some("tv".into());
                video.color_space = Some("bt709".into());
                video.color_transfer = Some("bt709".into());
                video.color_primaries = Some("bt709".into());
                video.hdr = HdrStatus::Sdr;
            }
            if let (Some(codec), Some(audio)) = (&step.target.audio_codec, next.first_audio_mut()) {
                audio.codec = Some(codec.clone());
            }
        }
        TransformId::TranscodeAudio => {
            if let (Some(codec), Some(audio)) = (&step.target.audio_codec, next.first_audio_mut()) {
                audio.codec = Some(codec.clone());
            }
        }
        TransformId::EncodeJpeg => {
            if let (Some(format), Some(image)) = (&step.target.image_format, next.image.as_mut()) {
                image.format = Some(format.clone());
                image.alpha = Some(false);
                image.animated = Some(false);
            }
        }
        TransformId::Remux | TransformId::ImageAdapt => {}
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

fn transcode_expected_facts(width: u32, height: u32, has_audio: bool) -> Vec<ExpectedFact> {
    let mut expected = vec![
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
    ];
    if has_audio {
        expected.push(ExpectedFact {
            field: Field::MediaAudioCodec,
            value: ExpectedValue::AudioCodec(AudioCodec::Aac),
        });
    }
    expected
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
