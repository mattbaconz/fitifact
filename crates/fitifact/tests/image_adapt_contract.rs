use std::io::Cursor;

use fitifact::adapt::AdaptationStatus;
use fitifact::artifact::ImageFormat;
use fitifact::constraints::{ConstraintSet, compile_from_json};
use fitifact::error::ErrorCode;
use fitifact::image::{artifact_from_bytes, sample_jpeg_rgb, sample_png_rgb};
use fitifact::image_adapt::{
    AtomicCancellation, ImageAdaptOptions, ImageAdaptProvider, ImageProviderOutput, NeverCancelled,
    NormalizedCropRectangle, execute_image_adaptation, execute_image_adaptation_with_provider,
    plan_image_adaptation,
};
use image::codecs::jpeg::JpegEncoder;
use image::{ColorType, DynamicImage, ImageBuffer, ImageFormat as EncoderFormat, Rgba};

fn constraints(hard: serde_json::Value) -> ConstraintSet {
    compile_from_json(
        &serde_json::json!({
            "schema": "fitifact.constraints/v1",
            "hard": hard
        })
        .to_string(),
    )
    .unwrap()
}

fn format_target(format: &str) -> ConstraintSet {
    constraints(serde_json::json!([
        {"id":"format","field":"image.format","op":"in","value":[format]}
    ]))
}

fn plan_for(bytes: &[u8], target: &ConstraintSet) -> fitifact::ImageAdaptPlan {
    let artifact = artifact_from_bytes(None, bytes).unwrap();
    plan_image_adaptation(&artifact, target).unwrap()
}

#[test]
fn compatible_jpeg_is_a_true_noop() {
    let input = sample_jpeg_rgb(32, 24);
    let target = format_target("jpeg");
    let plan = plan_for(&input, &target);
    assert!(plan.noop);
    let result = execute_image_adaptation(
        &input,
        &target,
        &plan,
        &ImageAdaptOptions::default(),
        &NeverCancelled,
    )
    .unwrap();
    assert_eq!(result.status, AdaptationStatus::Compatible);
    assert!(result.output.is_none());
    assert_eq!(result.stats.jpeg_encodes, 0);
}

#[test]
fn png_to_jpeg_is_format_only_and_discloses_metadata_stripping() {
    let input = sample_png_rgb(40, 30);
    let target = format_target("jpeg");
    let plan = plan_for(&input, &target);
    assert!(!plan.noop);
    assert_eq!((plan.target.width, plan.target.height), (40, 30));
    let result = execute_image_adaptation(
        &input,
        &target,
        &plan,
        &ImageAdaptOptions::default(),
        &NeverCancelled,
    )
    .unwrap();
    assert_eq!(
        result.output_artifact.image.unwrap().format,
        Some(ImageFormat::Jpeg)
    );
    assert_eq!(result.stats.jpeg_encodes, 1);
    assert!(
        result
            .disclosures
            .iter()
            .any(|text| text.contains("metadata"))
    );
}

#[test]
fn resize_only_preserves_png_and_aspect_ratio() {
    let input = sample_png_rgb(120, 60);
    let target = constraints(serde_json::json!([
        {"id":"max-width","field":"image.width","op":"lte","value":60},
        {"id":"max-height","field":"image.height","op":"lte","value":60}
    ]));
    let plan = plan_for(&input, &target);
    assert_eq!(plan.target.format, ImageFormat::Png);
    assert_eq!((plan.target.width, plan.target.height), (60, 30));
    let result = execute_image_adaptation(
        &input,
        &target,
        &plan,
        &ImageAdaptOptions::default(),
        &NeverCancelled,
    )
    .unwrap();
    let facts = result.output_artifact.image.unwrap();
    assert_eq!((facts.width, facts.height), (Some(60), Some(30)));
    assert_eq!(facts.format, Some(ImageFormat::Png));
}

#[test]
fn jpeg_byte_fitting_respects_ceiling_quality_floor_and_encode_cap() {
    let input = noisy_jpeg(256, 256, 95);
    let limit = input.len() as u64 * 3 / 4;
    let target = constraints(serde_json::json!([
        {"id":"format","field":"image.format","op":"in","value":["jpeg"]},
        {"id":"bytes","field":"file.bytes","op":"lte","value":limit}
    ]));
    let plan = plan_for(&input, &target);
    let result = execute_image_adaptation(
        &input,
        &target,
        &plan,
        &ImageAdaptOptions::default(),
        &NeverCancelled,
    )
    .unwrap();
    assert!(result.output.as_ref().unwrap().len() as u64 <= limit);
    assert!((1..=7).contains(&result.stats.jpeg_encodes));
    assert!(result.stats.jpeg_quality.unwrap() >= 50);
}

#[test]
fn lossless_png_fitting_uses_no_more_than_three_proportional_reductions() {
    let input = noisy_png(256, 256);
    let limit = input.len() as u64 / 3;
    let target = constraints(serde_json::json!([
        {"id":"format","field":"image.format","op":"in","value":["png"]},
        {"id":"bytes","field":"file.bytes","op":"lte","value":limit}
    ]));
    let plan = plan_for(&input, &target);
    assert!(plan.target.proportional_reduction_allowed);
    let result = execute_image_adaptation(
        &input,
        &target,
        &plan,
        &ImageAdaptOptions::default(),
        &NeverCancelled,
    )
    .unwrap();
    assert!(result.output.as_ref().unwrap().len() as u64 <= limit);
    assert!((1..=3).contains(&result.stats.dimension_reductions));
}

#[test]
fn upscale_is_explicitly_warned() {
    let input = sample_png_rgb(40, 20);
    let target = constraints(serde_json::json!([
        {"id":"width","field":"image.width","op":"eq","value":80},
        {"id":"height","field":"image.height","op":"eq","value":40}
    ]));
    let plan = plan_for(&input, &target);
    assert_eq!((plan.target.width, plan.target.height), (80, 40));
    assert_eq!(plan.target.upscale_warnings.len(), 1);
}

#[test]
fn combined_format_resize_and_byte_adaptation_validates_real_output() {
    let input = sample_png_rgb(240, 120);
    let target = constraints(serde_json::json!([
        {"id":"format","field":"image.format","op":"in","value":["jpeg"]},
        {"id":"width","field":"image.width","op":"lte","value":100},
        {"id":"bytes","field":"file.bytes","op":"lte","value":4000}
    ]));
    let plan = plan_for(&input, &target);
    let result = execute_image_adaptation(
        &input,
        &target,
        &plan,
        &ImageAdaptOptions::default(),
        &NeverCancelled,
    )
    .unwrap();
    assert!(result.report.compatible);
    assert_eq!(
        result.output_artifact.image.as_ref().unwrap().width,
        Some(100)
    );
    assert!(result.output.as_ref().unwrap().len() <= 4000);
}

#[test]
fn exact_aspect_change_requires_approved_normalized_crop() {
    let input = sample_png_rgb(100, 50);
    let target = constraints(serde_json::json!([
        {"id":"width","field":"image.width","op":"eq","value":50},
        {"id":"height","field":"image.height","op":"eq","value":50}
    ]));
    let plan = plan_for(&input, &target);
    assert!(plan.target.crop.required);
    let missing = execute_image_adaptation(
        &input,
        &target,
        &plan,
        &ImageAdaptOptions::default(),
        &NeverCancelled,
    )
    .unwrap_err();
    assert_eq!(missing.code, ErrorCode::SecurityBlocked);

    let result = execute_image_adaptation(
        &input,
        &target,
        &plan,
        &ImageAdaptOptions {
            crop: Some(NormalizedCropRectangle {
                x: 0.25,
                y: 0.0,
                width: 0.5,
                height: 1.0,
            }),
            crop_consent: true,
        },
        &NeverCancelled,
    )
    .unwrap();
    let facts = result.output_artifact.image.unwrap();
    assert_eq!((facts.width, facts.height), (Some(50), Some(50)));
}

#[test]
fn impossible_exact_byte_target_is_refused() {
    let input = noisy_jpeg(128, 128, 95);
    let target = constraints(serde_json::json!([
        {"id":"format","field":"image.format","op":"in","value":["jpeg"]},
        {"id":"width","field":"image.width","op":"eq","value":128},
        {"id":"height","field":"image.height","op":"eq","value":128},
        {"id":"bytes","field":"file.bytes","op":"lte","value":100}
    ]));
    let plan = plan_for(&input, &target);
    let error = execute_image_adaptation(
        &input,
        &target,
        &plan,
        &ImageAdaptOptions::default(),
        &NeverCancelled,
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::NoValidPlan);
    assert!(error.message.contains("impossible"));
}

#[test]
fn alpha_is_preserved_through_png_and_never_flattened_implicitly() {
    let input = rgba_png(32, 32);
    let jpeg_error = plan_image_adaptation(
        &artifact_from_bytes(None, &input).unwrap(),
        &format_target("jpeg"),
    )
    .unwrap_err();
    assert_eq!(jpeg_error.code, ErrorCode::NoValidPlan);
    assert!(jpeg_error.message.contains("transparency"));

    let target = constraints(serde_json::json!([
        {"id":"format","field":"image.format","op":"in","value":["png"]},
        {"id":"width","field":"image.width","op":"lte","value":16}
    ]));
    let plan = plan_for(&input, &target);
    let result = execute_image_adaptation(
        &input,
        &target,
        &plan,
        &ImageAdaptOptions::default(),
        &NeverCancelled,
    )
    .unwrap();
    assert_eq!(result.output_artifact.image.unwrap().alpha, Some(true));
}

#[test]
fn exif_orientation_is_applied_then_removed_on_adaptation() {
    let input = jpeg_with_orientation_6(20, 10);
    let artifact = artifact_from_bytes(None, &input).unwrap();
    let facts = artifact.image.as_ref().unwrap();
    assert_eq!((facts.width, facts.height), (Some(10), Some(20)));
    let target = format_target("png");
    let plan = plan_image_adaptation(&artifact, &target).unwrap();
    let result = execute_image_adaptation(
        &input,
        &target,
        &plan,
        &ImageAdaptOptions::default(),
        &NeverCancelled,
    )
    .unwrap();
    let output = result.output.unwrap();
    assert!(!output.windows(6).any(|window| window == b"Exif\0\0"));
    let facts = result.output_artifact.image.unwrap();
    assert_eq!((facts.width, facts.height), (Some(10), Some(20)));
}

#[test]
fn exact_byte_boundary_noops_and_one_byte_under_invokes_fitting() {
    let input = noisy_jpeg(64, 64, 90);
    let exact = constraints(serde_json::json!([
        {"id":"format","field":"image.format","op":"in","value":["jpeg"]},
        {"id":"bytes","field":"file.bytes","op":"lte","value":input.len()}
    ]));
    assert!(plan_for(&input, &exact).noop);
    let under = constraints(serde_json::json!([
        {"id":"format","field":"image.format","op":"in","value":["jpeg"]},
        {"id":"bytes","field":"file.bytes","op":"lte","value":input.len() - 1}
    ]));
    assert!(!plan_for(&input, &under).noop);
}

#[test]
fn cancellation_is_visible_at_the_execution_boundary() {
    let input = sample_png_rgb(64, 64);
    let target = format_target("jpeg");
    let plan = plan_for(&input, &target);
    let cancellation = AtomicCancellation::default();
    cancellation.cancel();
    let error = execute_image_adaptation(
        &input,
        &target,
        &plan,
        &ImageAdaptOptions::default(),
        &cancellation,
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ExecutionCancelled);
}

#[test]
fn animation_and_multi_image_inputs_are_identified_as_unsupported() {
    let animated = apng_header(sample_png_rgb(8, 8));
    let artifact = artifact_from_bytes(None, &animated).unwrap();
    assert_eq!(
        plan_image_adaptation(&artifact, &format_target("png"))
            .unwrap_err()
            .code,
        ErrorCode::NoValidPlan
    );

    let jpeg = sample_jpeg_rgb(8, 8);
    let mut multi = Vec::new();
    multi.extend_from_slice(&jpeg[..2]);
    multi.extend_from_slice(&[0xff, 0xe2, 0, 6]);
    multi.extend_from_slice(b"MPF\0");
    multi.extend_from_slice(&jpeg[2..]);
    let error = artifact_from_bytes(None, &multi).unwrap_err();
    assert_eq!(error.code, ErrorCode::InspectionUnsupported);
    assert!(error.message.contains("multi-image"));
}

#[test]
fn provider_success_is_not_accepted_without_post_validation() {
    struct WrongProvider(Vec<u8>);
    impl ImageAdaptProvider for WrongProvider {
        fn render(
            &self,
            _input: &[u8],
            _plan: &fitifact::ImageAdaptPlan,
            _options: &ImageAdaptOptions,
            _cancellation: &dyn fitifact::CancellationSignal,
        ) -> fitifact::error::Result<ImageProviderOutput> {
            Ok(ImageProviderOutput {
                bytes: self.0.clone(),
                stats: Default::default(),
            })
        }
    }

    let input = sample_jpeg_rgb(32, 32);
    let target = format_target("png");
    let plan = plan_for(&input, &target);
    let error = execute_image_adaptation_with_provider(
        &input,
        &target,
        &plan,
        &ImageAdaptOptions::default(),
        &NeverCancelled,
        &WrongProvider(input.clone()),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ValidationFailed);
}

#[test]
fn post_validation_rejects_provider_output_that_keeps_metadata() {
    struct MetadataProvider(Vec<u8>);
    impl ImageAdaptProvider for MetadataProvider {
        fn render(
            &self,
            _input: &[u8],
            _plan: &fitifact::ImageAdaptPlan,
            _options: &ImageAdaptOptions,
            _cancellation: &dyn fitifact::CancellationSignal,
        ) -> fitifact::error::Result<ImageProviderOutput> {
            Ok(ImageProviderOutput {
                bytes: self.0.clone(),
                stats: Default::default(),
            })
        }
    }

    let input = jpeg_with_orientation_6(20, 10);
    let target = constraints(serde_json::json!([
        {"id":"format","field":"image.format","op":"in","value":["jpeg"]},
        {"id":"width","field":"image.width","op":"eq","value":20},
        {"id":"height","field":"image.height","op":"eq","value":40}
    ]));
    let plan = plan_for(&input, &target);
    let metadata_output = jpeg_with_orientation_6(40, 20);
    let error = execute_image_adaptation_with_provider(
        &input,
        &target,
        &plan,
        &ImageAdaptOptions::default(),
        &NeverCancelled,
        &MetadataProvider(metadata_output),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ValidationFailed);
}

fn noisy_jpeg(width: u32, height: u32, quality: u8) -> Vec<u8> {
    let mut rgb = Vec::with_capacity((width * height * 3) as usize);
    for y in 0..height {
        for x in 0..width {
            rgb.extend_from_slice(&[
                ((x * 37 + y * 17) % 256) as u8,
                ((x * 11 + y * 53) % 256) as u8,
                ((x * 97 + y * 7) % 256) as u8,
            ]);
        }
    }
    let mut output = Vec::new();
    JpegEncoder::new_with_quality(&mut output, quality)
        .encode(&rgb, width, height, ColorType::Rgb8.into())
        .unwrap();
    output
}

fn rgba_png(width: u32, height: u32) -> Vec<u8> {
    let image = ImageBuffer::from_fn(width, height, |x, y| {
        Rgba([x as u8, y as u8, 100, ((x + y) % 255) as u8])
    });
    let mut output = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut output, EncoderFormat::Png)
        .unwrap();
    output.into_inner()
}

fn noisy_png(width: u32, height: u32) -> Vec<u8> {
    let image = ImageBuffer::from_fn(width, height, |x, y| {
        image::Rgb([
            ((x * 37 + y * 17) % 256) as u8,
            ((x * 11 + y * 53) % 256) as u8,
            ((x * 97 + y * 7) % 256) as u8,
        ])
    });
    let mut output = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(image)
        .write_to(&mut output, EncoderFormat::Png)
        .unwrap();
    output.into_inner()
}

fn apng_header(png: Vec<u8>) -> Vec<u8> {
    let insert_at = 8 + 12 + 13;
    let mut chunk = Vec::new();
    chunk.extend_from_slice(&8_u32.to_be_bytes());
    chunk.extend_from_slice(b"acTL");
    chunk.extend_from_slice(&1_u32.to_be_bytes());
    chunk.extend_from_slice(&0_u32.to_be_bytes());
    chunk.extend_from_slice(&png_crc32(&chunk[4..]).to_be_bytes());
    let mut output = Vec::with_capacity(png.len() + chunk.len());
    output.extend_from_slice(&png[..insert_at]);
    output.extend_from_slice(&chunk);
    output.extend_from_slice(&png[insert_at..]);
    output
}

fn png_crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn jpeg_with_orientation_6(width: u32, height: u32) -> Vec<u8> {
    let jpeg = sample_jpeg_rgb(width, height);
    let mut exif = b"Exif\0\0MM\0*\0\0\0\x08".to_vec();
    exif.extend_from_slice(&[0, 1]);
    exif.extend_from_slice(&[0x01, 0x12, 0, 3, 0, 0, 0, 1, 0, 6, 0, 0]);
    exif.extend_from_slice(&[0, 0, 0, 0]);
    let length = u16::try_from(exif.len() + 2).unwrap();
    let mut output = Vec::with_capacity(jpeg.len() + exif.len() + 4);
    output.extend_from_slice(&jpeg[..2]);
    output.extend_from_slice(&[0xff, 0xe1]);
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(&exif);
    output.extend_from_slice(&jpeg[2..]);
    output
}
