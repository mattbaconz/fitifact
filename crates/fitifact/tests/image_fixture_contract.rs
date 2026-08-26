use std::path::PathBuf;

use fitifact::adapt::AdaptationStatus;
use fitifact::artifact::ImageFormat;
use fitifact::constraints::{ConstraintSet, compile_from_json};
use fitifact::error::ErrorCode;
use fitifact::image::artifact_from_bytes;
use fitifact::image_adapt::{
    ImageAdaptOptions, NeverCancelled, NormalizedCropRectangle, execute_image_adaptation,
    plan_image_adaptation,
};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/image")
        .join(name);
    std::fs::read(path).unwrap()
}

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

fn jpeg_target() -> ConstraintSet {
    constraints(serde_json::json!([
        {"id":"format","field":"image.format","op":"in","value":["jpeg"]}
    ]))
}

#[test]
fn transparent_png_fixture_is_detected_and_refused_for_jpeg() {
    let bytes = fixture("transparent-png.png");
    let artifact = artifact_from_bytes(None, &bytes).unwrap();
    let facts = artifact.image.as_ref().unwrap();
    assert_eq!(facts.format, Some(ImageFormat::Png));
    assert_eq!(facts.alpha, Some(true));

    let error = plan_image_adaptation(&artifact, &jpeg_target()).unwrap_err();
    assert_eq!(error.code, ErrorCode::NoValidPlan);
    assert!(error.message.contains("transparency"));
}

#[test]
fn crop_grid_fixture_requires_consent_and_executes_the_approved_crop() {
    let bytes = fixture("crop-grid.png");
    let artifact = artifact_from_bytes(None, &bytes).unwrap();
    let target = constraints(serde_json::json!([
        {"id":"format","field":"image.format","op":"in","value":["jpeg"]},
        {"id":"width","field":"image.width","op":"eq","value":320},
        {"id":"height","field":"image.height","op":"eq","value":320}
    ]));
    let plan = plan_image_adaptation(&artifact, &target).unwrap();
    assert!(plan.target.crop.required);
    assert!(plan.target.crop.explicit_consent_required);

    let blocked = execute_image_adaptation(
        &bytes,
        &target,
        &plan,
        &ImageAdaptOptions::default(),
        &NeverCancelled,
    )
    .unwrap_err();
    assert_eq!(blocked.code, ErrorCode::SecurityBlocked);

    let result = execute_image_adaptation(
        &bytes,
        &target,
        &plan,
        &ImageAdaptOptions {
            crop: Some(NormalizedCropRectangle {
                x: 0.21875,
                y: 0.0,
                width: 0.5625,
                height: 1.0,
            }),
            crop_consent: true,
            ..Default::default()
        },
        &NeverCancelled,
    )
    .unwrap();
    assert_eq!(result.status, AdaptationStatus::Adapted);
    let output = result.output.expect("approved crop output");
    assert_eq!(&output[..3], b"\xff\xd8\xff");
    let facts = result.output_artifact.image.unwrap();
    assert_eq!(facts.format, Some(ImageFormat::Jpeg));
    assert_eq!(facts.width.zip(facts.height), Some((320, 320)));
}

#[test]
fn malformed_jpeg_fixture_returns_a_structured_decode_error() {
    let error = artifact_from_bytes(None, &fixture("malformed-image.jpg")).unwrap_err();
    assert_eq!(error.code, ErrorCode::InputInvalid);
    assert!(error.message.contains("could not be decoded"));
}

#[test]
fn oversized_pixels_fixture_hits_the_core_decoded_resource_limit() {
    let error = artifact_from_bytes(None, &fixture("oversized-pixels.png")).unwrap_err();
    assert_eq!(error.code, ErrorCode::InspectionLimit);
    assert!(error.message.contains("24-megapixel limit"));
}
