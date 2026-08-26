use fitifact::artifact::{Artifact, ImageFormat};
use fitifact::capability::default_catalog;
use fitifact::constraints::image_jpeg;
use fitifact::image::{
    animated_webp_header, artifact_from_bytes, encode_jpeg_bytes, sample_animated_gif,
    sample_bmp_rgb, sample_gif_rgb, sample_jpeg_rgb, sample_png_rgb, sample_tiff_rgb,
    sample_webp_rgb,
};
use fitifact::plan::{BlockingCode, plan};

#[test]
fn jpeg_already_fits_is_noop() {
    let artifact = artifact_from_bytes(None, &sample_jpeg_rgb(8, 8)).unwrap();
    assert_eq!(
        artifact.image.as_ref().unwrap().format,
        Some(ImageFormat::Jpeg)
    );
    assert!(plan(&artifact, &image_jpeg(), &default_catalog()).is_compatible());
}

#[test]
fn png_plans_jpeg_encode_without_ffmpeg_capability() {
    let artifact = artifact_from_bytes(None, &sample_png_rgb(8, 8)).unwrap();
    let outcome = plan(&artifact, &image_jpeg(), &default_catalog());
    assert_eq!(
        outcome.steps()[0].operation,
        fitifact::capability::TransformId::EncodeJpeg
    );
}

#[test]
fn still_webp_plans_jpeg_without_ffmpeg() {
    let artifact = artifact_from_bytes(None, &sample_webp_rgb(8, 8)).unwrap();
    assert_eq!(
        artifact.image.as_ref().unwrap().format,
        Some(ImageFormat::Webp)
    );
    assert_eq!(artifact.image.as_ref().unwrap().width, Some(8));
    assert_eq!(artifact.image.as_ref().unwrap().animated, Some(false));
    let outcome = plan(&artifact, &image_jpeg(), &default_catalog());
    assert!(
        !outcome
            .blocking_codes()
            .contains(&BlockingCode::UnsupportedImageFormat)
    );
    let adapted = fitifact::plan_image_adaptation(&artifact, &image_jpeg()).unwrap();
    assert!(!adapted.noop);
    assert_eq!(adapted.source_format, ImageFormat::Webp);
    assert_eq!(adapted.target.format, ImageFormat::Jpeg);
}

#[test]
fn animated_webp_is_refused() {
    let artifact = artifact_from_bytes(None, &animated_webp_header()).unwrap();
    assert_eq!(
        artifact.image.as_ref().unwrap().format,
        Some(ImageFormat::Webp)
    );
    assert_eq!(artifact.image.as_ref().unwrap().animated, Some(true));
    assert!(fitifact::plan_image_adaptation(&artifact, &image_jpeg()).is_err());
}

#[test]
fn tiff_bmp_and_still_gif_inspect_and_plan_to_jpeg() {
    for (bytes, format) in [
        (sample_tiff_rgb(8, 8), ImageFormat::Tiff),
        (sample_bmp_rgb(8, 8), ImageFormat::Bmp),
        (sample_gif_rgb(8, 8), ImageFormat::Gif),
    ] {
        let artifact = artifact_from_bytes(None, &bytes).unwrap();
        assert_eq!(artifact.image.as_ref().unwrap().format, Some(format));
        let adapted = fitifact::plan_image_adaptation(&artifact, &image_jpeg()).unwrap();
        assert_eq!(adapted.target.format, ImageFormat::Jpeg);
        assert!(!adapted.target.first_frame.required);
    }
}

#[test]
fn animated_gif_plans_first_frame_instead_of_silent_drop() {
    let artifact = artifact_from_bytes(None, &sample_animated_gif(8, 8)).unwrap();
    assert_eq!(artifact.image.as_ref().unwrap().animated, Some(true));
    let adapted = fitifact::plan_image_adaptation(&artifact, &image_jpeg()).unwrap();
    assert!(adapted.target.first_frame.required);
}

#[test]
fn image_constructor_noops_jpeg_target() {
    let artifact = Artifact::image(ImageFormat::Jpeg, 8, 8, 100, false, false);
    assert!(plan(&artifact, &image_jpeg(), &default_catalog()).is_compatible());
}

#[test]
fn png_encode_produces_jpeg_magic() {
    let encoded = encode_jpeg_bytes(&sample_png_rgb(8, 8)).unwrap();
    let out = artifact_from_bytes(None, &encoded).unwrap();
    assert_eq!(out.image.as_ref().unwrap().format, Some(ImageFormat::Jpeg));
    assert_eq!(out.image.as_ref().unwrap().width, Some(8));
}

#[test]
fn png_adapt_does_not_spawn_ffmpeg() {
    use fitifact::adapt::{AdaptRequest, AdaptationStatus, adapt};
    use fitifact::image::ImageProvider;
    use fitifact::inspect::DefaultInspector;
    use fitifact::runtime::{ExecutionContext, RecordingSpawner, SystemSpawner};
    use std::time::Duration;

    let dir = std::env::temp_dir().join(format!("fitifact-image-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("mismatch-png.png");
    let output = dir.join("out.jpg");
    std::fs::write(&input, sample_png_rgb(8, 8)).unwrap();
    let spawner = RecordingSpawner::new(SystemSpawner);
    let inspector = DefaultInspector::new(&spawner);
    let provider = ImageProvider;
    let result = adapt(AdaptRequest {
        input: &input,
        constraints: image_jpeg(),
        output: Some(output.clone()),
        catalog: None,
        inspector: &inspector,
        provider: Some(&provider),
        execution: ExecutionContext {
            timeout: Duration::from_secs(30),
            temp_dir: Some(dir.clone()),
        },
    })
    .unwrap();
    assert_eq!(
        result.status,
        AdaptationStatus::Adapted,
        "adapt failed: {:?}",
        result.error
    );
    assert_eq!(spawner.ffmpeg_spawn_count(), 0);
    assert_eq!(spawner.ffprobe_spawn_count(), 0);
    let written = std::fs::read(&output).unwrap();
    assert_eq!(
        artifact_from_bytes(None, &written)
            .unwrap()
            .image
            .unwrap()
            .format,
        Some(ImageFormat::Jpeg)
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn materialize_tracked_image_fixtures() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/image");
    std::fs::create_dir_all(&dir).unwrap();
    let jpeg = dir.join("compatible-jpeg.jpg");
    let png = dir.join("mismatch-png.png");
    if !jpeg.is_file() {
        std::fs::write(&jpeg, sample_jpeg_rgb(8, 8)).unwrap();
    }
    if !png.is_file() {
        std::fs::write(&png, sample_png_rgb(8, 8)).unwrap();
    }
    let jpeg_art = artifact_from_bytes(Some(&jpeg), &std::fs::read(&jpeg).unwrap()).unwrap();
    let png_art = artifact_from_bytes(Some(&png), &std::fs::read(&png).unwrap()).unwrap();
    assert_eq!(jpeg_art.image.unwrap().format, Some(ImageFormat::Jpeg));
    assert_eq!(png_art.image.unwrap().format, Some(ImageFormat::Png));
}

#[test]
fn image_resource_limits_use_exact_boundaries() {
    use fitifact::image::{
        MAX_IMAGE_INPUT_BYTES, MAX_IMAGE_PIXELS, enforce_decoded_limit, enforce_encoded_limit,
    };

    enforce_encoded_limit(MAX_IMAGE_INPUT_BYTES).unwrap();
    assert_eq!(
        enforce_encoded_limit(MAX_IMAGE_INPUT_BYTES + 1)
            .unwrap_err()
            .code,
        fitifact::ErrorCode::InspectionLimit
    );
    enforce_decoded_limit(6_000, 4_000).unwrap();
    assert_eq!(
        u64::from(6_000_u32) * u64::from(4_000_u32),
        MAX_IMAGE_PIXELS
    );
    assert_eq!(
        enforce_decoded_limit(6_000, 4_001).unwrap_err().code,
        fitifact::ErrorCode::InspectionLimit
    );
}

#[test]
fn public_jpeg_encoder_enforces_input_and_decoded_limits_before_pixel_allocation() {
    use fitifact::image::MAX_IMAGE_INPUT_BYTES;

    let mut exact = sample_png_rgb(8, 8);
    exact.resize(MAX_IMAGE_INPUT_BYTES, 0);
    assert!(encode_jpeg_bytes(&exact).is_ok());
    exact.push(0);
    assert_eq!(
        encode_jpeg_bytes(&exact).unwrap_err().code,
        fitifact::ErrorCode::InspectionLimit
    );

    let oversized_dimensions = png_with_dimensions(sample_png_rgb(8, 8), 6_001, 4_000);
    assert_eq!(
        encode_jpeg_bytes(&oversized_dimensions).unwrap_err().code,
        fitifact::ErrorCode::InspectionLimit
    );
}

#[test]
fn public_png_to_jpeg_encoder_refuses_alpha_instead_of_flattening() {
    use std::io::Cursor;

    let image = image::RgbaImage::from_pixel(8, 8, image::Rgba([20, 40, 60, 100]));
    let mut output = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut output, image::ImageFormat::Png)
        .unwrap();
    let error = encode_jpeg_bytes(&output.into_inner()).unwrap_err();
    assert_eq!(error.code, fitifact::ErrorCode::NoValidPlan);
    assert!(error.message.contains("transparency"));
}

#[test]
fn legacy_image_provider_enforces_encoded_and_decoded_limits() {
    use fitifact::image::{ImageProvider, MAX_IMAGE_INPUT_BYTES};
    use fitifact::runtime::{ExecutionContext, TransformProvider};

    let source = sample_png_rgb(8, 8);
    let artifact = artifact_from_bytes(None, &source).unwrap();
    let outcome = plan(&artifact, &image_jpeg(), &default_catalog());
    let dir = std::env::temp_dir().join(format!("fitifact-image-limit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("oversized.png");
    let output = dir.join("output.jpg");
    let mut oversized = source;
    oversized.resize(MAX_IMAGE_INPUT_BYTES + 1, 0);
    std::fs::write(&input, oversized).unwrap();

    let error = ImageProvider
        .execute(
            &input,
            &output,
            outcome.plan().unwrap(),
            &ExecutionContext::default(),
        )
        .unwrap_err();
    assert_eq!(error.code, fitifact::ErrorCode::InspectionLimit);
    assert!(!output.exists());

    std::fs::write(
        &input,
        png_with_dimensions(sample_png_rgb(8, 8), 6_001, 4_000),
    )
    .unwrap();
    let error = ImageProvider
        .execute(
            &input,
            &output,
            outcome.plan().unwrap(),
            &ExecutionContext::default(),
        )
        .unwrap_err();
    assert_eq!(error.code, fitifact::ErrorCode::InspectionLimit);
    assert!(!output.exists());
    let _ = std::fs::remove_dir_all(dir);
}

fn png_with_dimensions(mut png: Vec<u8>, width: u32, height: u32) -> Vec<u8> {
    png[16..20].copy_from_slice(&width.to_be_bytes());
    png[20..24].copy_from_slice(&height.to_be_bytes());
    let crc = png_crc32(&png[12..29]);
    png[29..33].copy_from_slice(&crc.to_be_bytes());
    png
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
