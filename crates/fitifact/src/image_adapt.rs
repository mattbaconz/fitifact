use std::io::Cursor;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{ColorType, DynamicImage, GenericImageView, ImageFormat as EncoderFormat};
use serde::{Deserialize, Serialize};

use crate::adapt::AdaptationStatus;
use crate::artifact::{Artifact, Family, ImageFormat};
use crate::capability::TransformId;
use crate::check::{CompatibilityReport, check};
use crate::constraints::{ConstraintSet, ConstraintValue, Field, Operator};
use crate::contract::ImageAdaptPlanSchema;
use crate::error::{Error, ErrorCode, Result};
use crate::image::{
    artifact_from_bytes, contains_image_metadata, decode_oriented, enforce_decoded_limit,
    jpeg_is_multi_image,
};
use crate::plan::{
    ExpectedFact, ExpectedValue, PLANNER_VERSION, Plan, PlanReason, PlanStep, PreservationClaim,
    StepTarget,
};

const MAX_JPEG_ENCODINGS: u8 = 7;
const MAX_DIMENSION_REDUCTIONS: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageMetadataBehavior {
    PreserveUnchanged,
    NormalizeOrientationAndStrip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageAdaptOperation {
    #[serde(rename = "image.adapt")]
    ImageAdapt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImagePreservationClaim {
    SourceFormat,
    Dimensions,
    AspectRatio,
    Alpha,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageCropRequirement {
    pub required: bool,
    pub explicit_consent_required: bool,
    pub target_aspect_width: u32,
    pub target_aspect_height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageAdaptTarget {
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
    pub max_bytes: Option<u64>,
    pub preservation: Vec<ImagePreservationClaim>,
    pub metadata: ImageMetadataBehavior,
    pub quality_warnings: Vec<String>,
    pub upscale_warnings: Vec<String>,
    pub crop: ImageCropRequirement,
    pub proportional_reduction_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageAdaptStepTarget {
    pub noop: bool,
    pub source_format: ImageFormat,
    pub source_width: u32,
    pub source_height: u32,
    pub output: ImageAdaptTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageAdaptPlan {
    pub schema: ImageAdaptPlanSchema,
    pub plan: Plan,
    pub operation: ImageAdaptOperation,
    pub noop: bool,
    pub source_format: ImageFormat,
    pub source_width: u32,
    pub source_height: u32,
    pub target: ImageAdaptTarget,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NormalizedCropRectangle {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl NormalizedCropRectangle {
    pub fn validate(self) -> Result<Self> {
        let values = [self.x, self.y, self.width, self.height];
        if values.iter().any(|value| !value.is_finite())
            || self.x < 0.0
            || self.y < 0.0
            || self.width <= 0.0
            || self.height <= 0.0
            || self.x + self.width > 1.0 + f64::EPSILON
            || self.y + self.height > 1.0 + f64::EPSILON
        {
            return Err(Error::new(
                ErrorCode::InputInvalid,
                "image.crop_invalid: crop coordinates must be normalized inside the image",
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ImageAdaptOptions {
    pub crop: Option<NormalizedCropRectangle>,
    pub crop_consent: bool,
}

pub trait CancellationSignal: Send + Sync {
    fn is_cancelled(&self) -> bool;
}

#[derive(Debug, Default)]
pub struct NeverCancelled;

impl CancellationSignal for NeverCancelled {
    fn is_cancelled(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Default)]
pub struct AtomicCancellation(Arc<AtomicBool>);

impl AtomicCancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

impl CancellationSignal for AtomicCancellation {
    fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ImageExecutionStats {
    pub jpeg_encodes: u8,
    pub dimension_reductions: u8,
    pub jpeg_quality: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct ImageProviderOutput {
    pub bytes: Vec<u8>,
    pub stats: ImageExecutionStats,
}

pub trait ImageAdaptProvider {
    fn render(
        &self,
        input: &[u8],
        plan: &ImageAdaptPlan,
        options: &ImageAdaptOptions,
        cancellation: &dyn CancellationSignal,
    ) -> Result<ImageProviderOutput>;
}

#[derive(Debug, Default)]
pub struct BuiltinImageProvider;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImageAdaptExecution {
    pub status: AdaptationStatus,
    pub source: Artifact,
    pub output_artifact: Artifact,
    pub report: CompatibilityReport,
    pub plan: ImageAdaptPlan,
    pub stats: ImageExecutionStats,
    pub disclosures: Vec<String>,
    #[serde(skip)]
    pub output: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy)]
struct DimensionRange {
    exact: Option<u32>,
    min: u32,
    max: u32,
}

impl Default for DimensionRange {
    fn default() -> Self {
        Self {
            exact: None,
            min: 1,
            max: u32::MAX,
        }
    }
}

pub fn plan_image_adaptation(
    artifact: &Artifact,
    constraints: &ConstraintSet,
) -> Result<ImageAdaptPlan> {
    if artifact.family != Family::Image {
        return Err(no_plan("image.source_required: the source is not an image"));
    }
    if constraints.hard.iter().any(|constraint| {
        matches!(
            constraint.field,
            Field::MediaContainer
                | Field::MediaVideoCodec
                | Field::MediaAudioCodec
                | Field::MediaVideoWidth
                | Field::MediaVideoHeight
                | Field::MediaVideoPixelFormat
                | Field::MediaVideoBitDepth
                | Field::MediaVideoColorRange
                | Field::MediaVideoColorSpace
                | Field::MediaVideoColorTransfer
                | Field::MediaVideoColorPrimaries
                | Field::MediaVideoHdr
        ) || (constraint.field == Field::FileFamily
            && constraint.value != ConstraintValue::Text("image".into()))
    }) {
        return Err(no_plan(
            "image.target_invalid: image adaptation cannot satisfy media requirements",
        ));
    }
    let facts = artifact
        .image
        .as_ref()
        .ok_or_else(|| no_plan("image.facts_missing: image facts are unavailable"))?;
    if facts.animated == Some(true) {
        return Err(no_plan(
            "image.animation_unsupported: animated image inputs are unsupported",
        ));
    }
    let source_format = facts
        .format
        .clone()
        .ok_or_else(|| no_plan("image.format_unknown: source format is unknown"))?;
    if !matches!(source_format, ImageFormat::Jpeg | ImageFormat::Png) {
        return Err(no_plan(
            "image.format_unsupported: only JPEG and PNG inputs can be adapted",
        ));
    }
    let source_width = facts
        .width
        .ok_or_else(|| no_plan("image.width_unknown: source width is unknown"))?;
    let source_height = facts
        .height
        .ok_or_else(|| no_plan("image.height_unknown: source height is unknown"))?;
    let source_alpha = facts.alpha.unwrap_or(false);
    let allowed_formats = allowed_formats(constraints)?;
    let target_format = choose_format(&source_format, source_alpha, &allowed_formats)?;
    let width_range = dimension_range(constraints, Field::ImageWidth)?;
    let height_range = dimension_range(constraints, Field::ImageHeight)?;
    let (target_width, target_height) =
        choose_dimensions(source_width, source_height, width_range, height_range);
    enforce_decoded_limit(target_width, target_height)?;
    let crop_required = !same_aspect(source_width, source_height, target_width, target_height);
    let max_bytes = constraints
        .hard
        .iter()
        .filter(|constraint| constraint.field == Field::FileBytes)
        .filter_map(|constraint| integer_value(&constraint.value))
        .min();
    let noop = check(artifact, constraints).compatible;
    let proportional_reduction_allowed = max_bytes.is_some()
        && width_range.exact.is_none()
        && height_range.exact.is_none()
        && width_range.min <= 1
        && height_range.min <= 1;
    let reduction_may_be_needed = proportional_reduction_allowed
        && max_bytes.is_some_and(|limit| artifact.byte_length > limit);

    let mut preservation = Vec::new();
    if target_format == source_format {
        preservation.push(ImagePreservationClaim::SourceFormat);
    }
    if target_width == source_width && target_height == source_height && !reduction_may_be_needed {
        preservation.push(ImagePreservationClaim::Dimensions);
    }
    if !crop_required {
        preservation.push(ImagePreservationClaim::AspectRatio);
    }
    if source_alpha && target_format == ImageFormat::Png {
        preservation.push(ImagePreservationClaim::Alpha);
    }

    let mut quality_warnings = Vec::new();
    if !noop && target_format == ImageFormat::Jpeg {
        quality_warnings.push(
            "JPEG output is lossy; fitting may lower quality from 95 to no less than 50.".into(),
        );
    }
    let mut upscale_warnings = Vec::new();
    if target_width > source_width || target_height > source_height {
        upscale_warnings.push(
            "The required dimensions upscale the image; this cannot add source detail.".into(),
        );
    }
    let metadata = if noop {
        ImageMetadataBehavior::PreserveUnchanged
    } else {
        ImageMetadataBehavior::NormalizeOrientationAndStrip
    };
    let mut warnings = quality_warnings.clone();
    warnings.extend(upscale_warnings.clone());
    if !noop {
        warnings.push(
            "EXIF orientation will be applied and remaining metadata will be stripped.".into(),
        );
    }
    if crop_required {
        warnings.push("Cropping requires an explicit crop rectangle and consent.".into());
    }

    let target = ImageAdaptTarget {
        format: target_format.clone(),
        width: target_width,
        height: target_height,
        max_bytes,
        preservation,
        metadata,
        quality_warnings,
        upscale_warnings,
        crop: ImageCropRequirement {
            required: crop_required,
            explicit_consent_required: crop_required,
            target_aspect_width: target_width,
            target_aspect_height: target_height,
        },
        proportional_reduction_allowed,
    };
    let canonical_preserved = target
        .preservation
        .contains(&ImagePreservationClaim::Dimensions)
        .then_some(PreservationClaim::ImageDimensions)
        .into_iter()
        .collect::<Vec<_>>();
    let canonical_plan = Plan {
        schema: crate::contract::PlanSchema,
        planner_version: PLANNER_VERSION.into(),
        steps: vec![PlanStep {
            id: "step-1".into(),
            operation: TransformId::ImageAdapt,
            target: StepTarget {
                image: Some(ImageAdaptStepTarget {
                    noop,
                    source_format: source_format.clone(),
                    source_width,
                    source_height,
                    output: target.clone(),
                }),
                ..StepTarget::default()
            },
            reasons: constraints
                .hard
                .iter()
                .map(|constraint| PlanReason {
                    constraint_id: constraint.id.clone(),
                    message: format!(
                        "The image output must satisfy {}.",
                        constraint.field.as_str()
                    ),
                })
                .collect(),
            expected: vec![
                ExpectedFact {
                    field: Field::ImageFormat,
                    value: ExpectedValue::Text(target_format.as_str().into()),
                },
                ExpectedFact {
                    field: Field::ImageWidth,
                    value: ExpectedValue::Integer(u64::from(target_width)),
                },
                ExpectedFact {
                    field: Field::ImageHeight,
                    value: ExpectedValue::Integer(u64::from(target_height)),
                },
            ],
            preservation: canonical_preserved.clone(),
            warnings: warnings.clone(),
        }],
        preserved: canonical_preserved,
        warnings: warnings.clone(),
    };

    Ok(ImageAdaptPlan {
        schema: ImageAdaptPlanSchema,
        plan: canonical_plan,
        operation: ImageAdaptOperation::ImageAdapt,
        noop,
        source_format,
        source_width,
        source_height,
        target,
        warnings,
    })
}

pub fn execute_image_adaptation(
    input: &[u8],
    constraints: &ConstraintSet,
    plan: &ImageAdaptPlan,
    options: &ImageAdaptOptions,
    cancellation: &dyn CancellationSignal,
) -> Result<ImageAdaptExecution> {
    execute_image_adaptation_with_provider(
        input,
        constraints,
        plan,
        options,
        cancellation,
        &BuiltinImageProvider,
    )
}

pub fn execute_image_adaptation_with_provider(
    input: &[u8],
    constraints: &ConstraintSet,
    plan: &ImageAdaptPlan,
    options: &ImageAdaptOptions,
    cancellation: &dyn CancellationSignal,
    provider: &dyn ImageAdaptProvider,
) -> Result<ImageAdaptExecution> {
    cancelled(cancellation)?;
    if jpeg_is_multi_image(input) {
        return Err(Error::new(
            ErrorCode::InspectionUnsupported,
            "image.multi_image_unsupported: multi-image inputs are unsupported",
        ));
    }
    let source = artifact_from_bytes(None, input)?;
    let expected = plan_image_adaptation(&source, constraints)?;
    if &expected != plan {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            "image.plan_mismatch: plan does not match the inspected source and constraints",
        ));
    }
    if plan.noop {
        let report = check(&source, constraints);
        if !report.compatible {
            return Err(validation_failed());
        }
        return Ok(ImageAdaptExecution {
            status: AdaptationStatus::Compatible,
            source: source.clone(),
            output_artifact: source,
            report,
            plan: plan.clone(),
            stats: ImageExecutionStats::default(),
            disclosures: Vec::new(),
            output: None,
        });
    }
    validate_crop(plan, options)?;
    let rendered = provider.render(input, plan, options, cancellation)?;
    cancelled(cancellation)?;
    let output_artifact = artifact_from_bytes(None, &rendered.bytes)?;
    let report = check(&output_artifact, constraints);
    let output_facts = output_artifact
        .image
        .as_ref()
        .ok_or_else(validation_failed)?;
    let output_dimensions = output_facts.width.zip(output_facts.height);
    let dimensions_match_target = plan.target.proportional_reduction_allowed
        || output_dimensions == Some((plan.target.width, plan.target.height));
    let preservation_holds = plan.target.preservation.iter().all(|claim| match claim {
        ImagePreservationClaim::SourceFormat => {
            output_facts.format.as_ref() == Some(&plan.source_format)
        }
        ImagePreservationClaim::Dimensions => {
            output_dimensions == Some((plan.source_width, plan.source_height))
        }
        ImagePreservationClaim::AspectRatio => output_dimensions.is_some_and(|(width, height)| {
            same_aspect(plan.source_width, plan.source_height, width, height)
        }),
        ImagePreservationClaim::Alpha => output_facts.alpha == Some(true),
    });
    if !report.compatible
        || output_facts.format.as_ref() != Some(&plan.target.format)
        || !dimensions_match_target
        || !preservation_holds
        || output_facts.alpha == Some(true) && plan.target.format != ImageFormat::Png
        || contains_image_metadata(&rendered.bytes, &plan.target.format)
        || rendered.stats.jpeg_encodes > MAX_JPEG_ENCODINGS
        || rendered.stats.dimension_reductions > MAX_DIMENSION_REDUCTIONS
    {
        return Err(validation_failed());
    }
    Ok(ImageAdaptExecution {
        status: AdaptationStatus::Adapted,
        source,
        output_artifact,
        report,
        plan: plan.clone(),
        stats: rendered.stats,
        disclosures: vec![
            "EXIF orientation was normalized before rendering.".into(),
            "Remaining source metadata was stripped from the output.".into(),
        ],
        output: Some(rendered.bytes),
    })
}

impl ImageAdaptProvider for BuiltinImageProvider {
    fn render(
        &self,
        input: &[u8],
        plan: &ImageAdaptPlan,
        options: &ImageAdaptOptions,
        cancellation: &dyn CancellationSignal,
    ) -> Result<ImageProviderOutput> {
        cancelled(cancellation)?;
        enforce_decoded_limit(plan.target.width, plan.target.height)?;
        let mut image = decode_oriented(input)?;
        cancelled(cancellation)?;
        if plan.target.crop.required {
            let crop = options.crop.ok_or_else(|| {
                Error::new(
                    ErrorCode::SecurityBlocked,
                    "image.crop_consent_required: required crop rectangle is missing",
                )
            })?;
            image = crop_image(image, crop, &plan.target.crop)?;
        }
        if image.dimensions() != (plan.target.width, plan.target.height) {
            image = if plan.target.crop.required {
                image.resize_exact(plan.target.width, plan.target.height, FilterType::Lanczos3)
            } else {
                image.resize(plan.target.width, plan.target.height, FilterType::Lanczos3)
            };
        }
        cancelled(cancellation)?;
        match plan.target.format {
            ImageFormat::Jpeg => encode_fitted_jpeg(image, plan, cancellation),
            ImageFormat::Png => encode_fitted_png(image, plan, cancellation),
            _ => Err(no_plan(
                "image.output_format_unsupported: only JPEG and PNG can be produced",
            )),
        }
    }
}

fn encode_fitted_jpeg(
    mut image: DynamicImage,
    plan: &ImageAdaptPlan,
    cancellation: &dyn CancellationSignal,
) -> Result<ImageProviderOutput> {
    let mut stats = ImageExecutionStats::default();
    let qualities = [95_u8, 80, 65, 50];
    let ceiling = plan.target.max_bytes;
    let mut last_size = 0_u64;
    for quality in qualities {
        cancelled(cancellation)?;
        let bytes = encode_jpeg(&image, quality)?;
        stats.jpeg_encodes += 1;
        stats.jpeg_quality = Some(quality);
        last_size = bytes.len() as u64;
        if ceiling.is_none_or(|limit| bytes.len() as u64 <= limit) {
            return Ok(ImageProviderOutput { bytes, stats });
        }
    }
    while stats.dimension_reductions < MAX_DIMENSION_REDUCTIONS
        && stats.jpeg_encodes < MAX_JPEG_ENCODINGS
        && plan.target.proportional_reduction_allowed
    {
        let limit = ceiling.ok_or_else(|| {
            Error::new(
                ErrorCode::InputInvalid,
                "image.plan_invalid: proportional fitting requires a byte ceiling",
            )
        })?;
        let (width, height) = reduced_dimensions(image.dimensions(), last_size, limit);
        if (width, height) == image.dimensions() {
            break;
        }
        image = image.resize(width, height, FilterType::Lanczos3);
        stats.dimension_reductions += 1;
        cancelled(cancellation)?;
        let bytes = encode_jpeg(&image, 50)?;
        stats.jpeg_encodes += 1;
        stats.jpeg_quality = Some(50);
        last_size = bytes.len() as u64;
        if bytes.len() as u64 <= limit {
            return Ok(ImageProviderOutput { bytes, stats });
        }
    }
    Err(no_plan(
        "image.byte_target_impossible: JPEG cannot meet the byte ceiling at quality 50",
    ))
}

fn encode_fitted_png(
    mut image: DynamicImage,
    plan: &ImageAdaptPlan,
    cancellation: &dyn CancellationSignal,
) -> Result<ImageProviderOutput> {
    let mut stats = ImageExecutionStats::default();
    let mut bytes = encode_png(&image)?;
    let Some(limit) = plan.target.max_bytes else {
        return Ok(ImageProviderOutput { bytes, stats });
    };
    while bytes.len() as u64 > limit
        && stats.dimension_reductions < MAX_DIMENSION_REDUCTIONS
        && plan.target.proportional_reduction_allowed
    {
        cancelled(cancellation)?;
        let (width, height) = reduced_dimensions(image.dimensions(), bytes.len() as u64, limit);
        if (width, height) == image.dimensions() {
            break;
        }
        image = image.resize(width, height, FilterType::Lanczos3);
        stats.dimension_reductions += 1;
        bytes = encode_png(&image)?;
    }
    if bytes.len() as u64 > limit {
        return Err(no_plan(
            "image.byte_target_impossible: lossless PNG cannot meet the byte ceiling",
        ));
    }
    Ok(ImageProviderOutput { bytes, stats })
}

fn encode_jpeg(image: &DynamicImage, quality: u8) -> Result<Vec<u8>> {
    let rgb = image.to_rgb8();
    let mut output = Vec::new();
    JpegEncoder::new_with_quality(&mut output, quality)
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            ColorType::Rgb8.into(),
        )
        .map_err(|_| Error::new(ErrorCode::ExecutionFailed, "image.jpeg_encode_failed"))?;
    Ok(output)
}

fn encode_png(image: &DynamicImage) -> Result<Vec<u8>> {
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, EncoderFormat::Png)
        .map_err(|_| Error::new(ErrorCode::ExecutionFailed, "image.png_encode_failed"))?;
    Ok(output.into_inner())
}

fn crop_image(
    image: DynamicImage,
    crop: NormalizedCropRectangle,
    requirement: &ImageCropRequirement,
) -> Result<DynamicImage> {
    let (source_width, source_height) = image.dimensions();
    let x = (crop.x * f64::from(source_width)).floor() as u32;
    let y = (crop.y * f64::from(source_height)).floor() as u32;
    let width = (crop.width * f64::from(source_width)).round().max(1.0) as u32;
    let height = (crop.height * f64::from(source_height)).round().max(1.0) as u32;
    let width = width.min(source_width.saturating_sub(x));
    let height = height.min(source_height.saturating_sub(y));
    if !crop_aspect_matches(
        width,
        height,
        requirement.target_aspect_width,
        requirement.target_aspect_height,
    ) {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            "image.crop_aspect_mismatch: crop rectangle must match the target aspect ratio",
        ));
    }
    Ok(image.crop_imm(x, y, width, height))
}

fn validate_crop(plan: &ImageAdaptPlan, options: &ImageAdaptOptions) -> Result<()> {
    if let Some(crop) = options.crop {
        crop.validate()?;
    }
    if plan.target.crop.required && (!options.crop_consent || options.crop.is_none()) {
        return Err(Error::new(
            ErrorCode::SecurityBlocked,
            "image.crop_consent_required: required crop was not explicitly approved",
        ));
    }
    Ok(())
}

fn allowed_formats(constraints: &ConstraintSet) -> Result<Option<Vec<ImageFormat>>> {
    let mut result: Option<Vec<ImageFormat>> = None;
    for constraint in &constraints.hard {
        if constraint.field != Field::ImageFormat {
            continue;
        }
        let values = constraint.value.as_text_list();
        let current: Vec<ImageFormat> = values
            .iter()
            .filter_map(|value| ImageFormat::parse_constraint(value))
            .collect();
        result = Some(match result {
            None => current,
            Some(existing) => existing
                .into_iter()
                .filter(|format| current.contains(format))
                .collect(),
        });
    }
    if result.as_ref().is_some_and(Vec::is_empty) {
        return Err(no_plan(
            "image.format_conflict: no output format is allowed",
        ));
    }
    Ok(result)
}

fn choose_format(
    source: &ImageFormat,
    source_alpha: bool,
    allowed: &Option<Vec<ImageFormat>>,
) -> Result<ImageFormat> {
    let allowed = allowed.clone().unwrap_or_else(|| vec![source.clone()]);
    if allowed.contains(source) {
        return Ok(source.clone());
    }
    if source_alpha {
        if allowed.contains(&ImageFormat::Png) {
            return Ok(ImageFormat::Png);
        }
        return Err(no_plan(
            "image.transparency_flattening_refused: alpha can only be preserved through PNG",
        ));
    }
    if allowed.contains(&ImageFormat::Jpeg) {
        return Ok(ImageFormat::Jpeg);
    }
    if allowed.contains(&ImageFormat::Png) {
        return Ok(ImageFormat::Png);
    }
    Err(no_plan(
        "image.output_format_unsupported: only JPEG and PNG can be produced",
    ))
}

fn dimension_range(constraints: &ConstraintSet, field: Field) -> Result<DimensionRange> {
    let mut range = DimensionRange::default();
    for constraint in &constraints.hard {
        if constraint.field != field {
            continue;
        }
        let value = u32::try_from(integer_value(&constraint.value).ok_or_else(|| {
            Error::new(ErrorCode::InputInvalid, "image.dimension_target_invalid")
        })?)
        .map_err(|_| Error::new(ErrorCode::InputInvalid, "image.dimension_target_overflow"))?;
        match constraint.op {
            Operator::Eq => range.exact = Some(value),
            Operator::Gte => range.min = range.min.max(value),
            Operator::Lte => range.max = range.max.min(value),
            Operator::In => {
                return Err(Error::new(
                    ErrorCode::InputInvalid,
                    "image.dimension_operator_invalid",
                ));
            }
        }
    }
    if let Some(exact) = range.exact {
        range.min = exact;
        range.max = exact;
    }
    Ok(range)
}

fn choose_dimensions(
    source_width: u32,
    source_height: u32,
    width: DimensionRange,
    height: DimensionRange,
) -> (u32, u32) {
    if let (Some(target_width), Some(target_height)) = (width.exact, height.exact) {
        return (target_width, target_height);
    }
    if let Some(target_width) = width.exact {
        let proportional = scaled(source_height, target_width, source_width);
        return (target_width, proportional.clamp(height.min, height.max));
    }
    if let Some(target_height) = height.exact {
        let proportional = scaled(source_width, target_height, source_height);
        return (proportional.clamp(width.min, width.max), target_height);
    }

    let lower = (f64::from(width.min) / f64::from(source_width))
        .max(f64::from(height.min) / f64::from(source_height));
    let upper = (f64::from(width.max) / f64::from(source_width))
        .min(f64::from(height.max) / f64::from(source_height));
    if lower <= upper {
        let scale = 1.0_f64.clamp(lower, upper);
        return (
            (f64::from(source_width) * scale).round().max(1.0) as u32,
            (f64::from(source_height) * scale).round().max(1.0) as u32,
        );
    }
    (
        source_width.clamp(width.min, width.max),
        source_height.clamp(height.min, height.max),
    )
}

fn reduced_dimensions(dimensions: (u32, u32), actual_bytes: u64, limit: u64) -> (u32, u32) {
    let ratio = ((limit as f64 / actual_bytes as f64).sqrt() * 0.95).min(0.9);
    (
        (f64::from(dimensions.0) * ratio).floor().max(1.0) as u32,
        (f64::from(dimensions.1) * ratio).floor().max(1.0) as u32,
    )
}

fn scaled(value: u32, numerator: u32, denominator: u32) -> u32 {
    ((u64::from(value) * u64::from(numerator) + u64::from(denominator) / 2)
        / u64::from(denominator))
    .max(1) as u32
}

fn same_aspect(first_width: u32, first_height: u32, second_width: u32, second_height: u32) -> bool {
    let left = u128::from(first_width) * u128::from(second_height);
    let right = u128::from(second_width) * u128::from(first_height);
    left == right
}

fn crop_aspect_matches(
    crop_width: u32,
    crop_height: u32,
    target_width: u32,
    target_height: u32,
) -> bool {
    let left = u128::from(crop_width) * u128::from(target_height);
    let right = u128::from(target_width) * u128::from(crop_height);
    let difference = left.abs_diff(right);
    difference == 0 || difference * 2 < u128::from(target_width.max(target_height).max(1))
}

fn integer_value(value: &ConstraintValue) -> Option<u64> {
    match value {
        ConstraintValue::Integer(value) => Some(*value),
        _ => None,
    }
}

fn cancelled(signal: &dyn CancellationSignal) -> Result<()> {
    if signal.is_cancelled() {
        return Err(Error::new(
            ErrorCode::ExecutionCancelled,
            "image.execution_cancelled",
        ));
    }
    Ok(())
}

fn no_plan(message: &str) -> Error {
    Error::new(ErrorCode::NoValidPlan, message)
}

fn validation_failed() -> Error {
    Error::new(
        ErrorCode::ValidationFailed,
        "image.post_validation_failed: rendered output did not satisfy the original constraints",
    )
}
