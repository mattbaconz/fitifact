use fitifact::artifact::{Artifact, ImageFormat};
use fitifact::capability::default_catalog;
use fitifact::constraints::image_jpeg;
use fitifact::image::{artifact_from_bytes, encode_jpeg_bytes, sample_jpeg_rgb, sample_png_rgb};
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
fn webp_magic_is_refused() {
    let mut bytes = b"RIFF....WEBPVP8 ".to_vec();
    bytes[4..8].copy_from_slice(&[8, 0, 0, 0]);
    let artifact = artifact_from_bytes(None, &bytes).unwrap();
    assert_eq!(
        artifact.image.as_ref().unwrap().format,
        Some(ImageFormat::Webp)
    );
    assert_eq!(
        plan(&artifact, &image_jpeg(), &default_catalog()).blocking_codes(),
        vec![BlockingCode::UnsupportedImageFormat]
    );
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
