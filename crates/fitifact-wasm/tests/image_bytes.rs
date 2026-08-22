use fitifact_wasm::{
    adapt_bytes, adapt_rgba, compile_constraints, compile_requirements, image_limits,
    inspect_bytes, plan_bytes, plan_rgba, sample_jpeg_rgb, sample_png_rgb, validate_bytes,
};

fn parse(json: &str) -> serde_json::Value {
    serde_json::from_str(json).expect(json)
}

fn constraints(format: &str, width: u32, height: u32) -> String {
    serde_json::json!({
        "schema": "fitifact.constraints/v1",
        "hard": [
            {"id":"format", "field":"image.format", "op":"in", "value":[format]},
            {"id":"width", "field":"image.width", "op":"eq", "value":width},
            {"id":"height", "field":"image.height", "op":"eq", "value":height}
        ],
        "preferences": {"preserve_audio":true, "preserve_resolution":true}
    })
    .to_string()
}

#[test]
fn requirements_compile_to_the_core_contract() {
    let report = parse(&compile_requirements("JPEG, exactly 1200 x 630, max 2 MB"));
    assert_eq!(report["schema"], "fitifact.requirements/v1");
    assert_eq!(report["constraints"]["schema"], "fitifact.constraints/v1");
    assert_eq!(report["constraints"]["hard"].as_array().unwrap().len(), 4);
    assert!(report["ambiguities"].as_array().unwrap().is_empty());
}

#[test]
fn complete_consumer_intersection_crosses_the_wasm_contract() {
    let report = parse(&compile_requirements(
        "JPEG or PNG, min 640 x 480, max 1920 x 1080, max 2 MB",
    ));
    let hard = report["constraints"]["hard"].as_array().unwrap();
    assert_eq!(hard.len(), 6);
    assert!(hard.iter().any(|constraint| {
        constraint["field"] == "image.format"
            && constraint["value"] == serde_json::json!(["jpeg", "png"])
    }));
    for (field, op, value) in [
        ("image.width", "gte", 640),
        ("image.height", "gte", 480),
        ("image.width", "lte", 1920),
        ("image.height", "lte", 1080),
        ("file.bytes", "lte", 2_000_000),
    ] {
        assert!(hard.iter().any(|constraint| {
            constraint["field"] == field && constraint["op"] == op && constraint["value"] == value
        }));
    }
}

#[test]
fn browser_resource_limits_are_derived_from_the_core() {
    let limits = parse(&image_limits());
    assert_eq!(limits["schema"], "fitifact.image-limits/v1");
    assert_eq!(limits["max_encoded_bytes"], fitifact::MAX_IMAGE_INPUT_BYTES);
    assert_eq!(limits["max_decoded_pixels"], fitifact::MAX_IMAGE_PIXELS);
}

#[test]
fn malformed_editable_constraints_return_a_structured_error() {
    let report = parse(&compile_constraints(
        r#"{"schema":"fitifact.constraints/v1","hard":[],"preferences":{}}"#,
    ));
    assert_eq!(report["schema"], "fitifact.error/v1");
    assert_eq!(report["code"], "INPUT_INVALID");
}

#[test]
fn custom_constraints_drive_inspect_plan_adapt_and_validate() {
    let source = sample_png_rgb(16, 8);
    let target = constraints("jpeg", 8, 4);
    let planned = parse(&plan_bytes(&source, &target));
    assert_eq!(planned["schema"], "fitifact.web-plan/v1");
    assert_eq!(planned["plan"]["schema"], "fitifact.image-adapt-plan/v1");
    assert_eq!(planned["plan"]["plan"]["schema"], "fitifact.plan/v1");
    assert_eq!(
        planned["plan"]["plan"]["steps"][0]["operation"],
        "image.adapt"
    );
    assert_eq!(planned["inspection"]["image"]["format"], "png");
    assert_eq!(planned["report"]["compatible"], false);
    assert_eq!(planned["plan"]["target"]["width"], 8);
    assert_eq!(planned["plan"]["target"]["format"], "jpeg");

    let adapted = adapt_bytes(&source, &target, r#"{"crop":null,"crop_consent":false}"#);
    let report = parse(&adapted.report_json);
    assert_eq!(report["status"], "adapted");
    let output = adapted.output.expect("adapted bytes");
    assert_eq!(parse(&inspect_bytes(&output))["image"]["format"], "jpeg");
    assert_eq!(parse(&validate_bytes(&output, &target))["compatible"], true);
}

#[test]
fn no_op_returns_no_duplicate_output_buffer() {
    let source = sample_jpeg_rgb(8, 8);
    let target = constraints("jpeg", 8, 8);
    let adapted = adapt_bytes(&source, &target, r#"{"crop":null,"crop_consent":false}"#);
    assert_eq!(parse(&adapted.report_json)["status"], "compatible");
    assert!(adapted.output.is_none());
}

#[test]
fn changed_aspect_requires_explicit_crop_rectangle_and_consent() {
    let source = sample_png_rgb(12, 8);
    let target = constraints("png", 8, 8);
    let planned = parse(&plan_bytes(&source, &target));
    assert_eq!(planned["plan"]["target"]["crop"]["required"], true);

    let blocked = adapt_bytes(&source, &target, r#"{"crop":null,"crop_consent":false}"#);
    assert_eq!(parse(&blocked.report_json)["code"], "SECURITY_BLOCKED");

    let adapted = adapt_bytes(
        &source,
        &target,
        r#"{"crop":{"x":0.16666666666666666,"y":0.0,"width":0.6666666666666666,"height":1.0},"crop_consent":true}"#,
    );
    assert_eq!(parse(&adapted.report_json)["status"], "adapted");
    assert!(adapted.output.is_some());
}

#[test]
fn decoded_rgba_uses_the_same_plan_execute_and_validation_contract() {
    let rgba = [255_u8, 0, 0, 255, 0, 255, 0, 255];
    let target = constraints("png", 2, 1);
    let planned_rgba = plan_rgba(&rgba, 2, 1, &target);
    let planned = parse(&planned_rgba.report_json);
    assert_eq!(planned["schema"], "fitifact.web-plan/v1");
    assert_eq!(planned["report"]["compatible"], true);
    let preview = planned_rgba.preview.expect("encoded PNG preview");
    assert_eq!(&preview[..8], b"\x89PNG\r\n\x1a\n");
    let adapted = adapt_rgba(
        &rgba,
        2,
        1,
        &target,
        r#"{"crop":null,"crop_consent":false}"#,
    );
    let report = parse(&adapted.report_json);
    assert_eq!(report["status"], "compatible");
    let output = adapted
        .output
        .expect("compatible RGBA still returns encoded bytes");
    assert_eq!(&output[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(parse(&validate_bytes(&output, &target))["compatible"], true);
    assert_eq!(report["output_artifact"]["image"]["format"], "png");
}

#[test]
fn decoded_rgba_rejects_length_mismatch_and_resource_limit() {
    let target = constraints("png", 1, 1);
    let mismatch = adapt_rgba(
        &[0; 3],
        1,
        1,
        &target,
        r#"{"crop":null,"crop_consent":false}"#,
    );
    assert_eq!(parse(&mismatch.report_json)["code"], "INPUT_INVALID");

    let over_limit = adapt_rgba(
        &[],
        6_001,
        4_000,
        &target,
        r#"{"crop":null,"crop_consent":false}"#,
    );
    assert_eq!(parse(&over_limit.report_json)["code"], "INSPECTION_LIMIT");
}

#[test]
fn opaque_decoded_rgba_can_target_jpeg_but_real_alpha_is_preserved_and_refused() {
    let target = constraints("jpeg", 2, 1);
    let opaque = [255_u8, 0, 0, 255, 0, 255, 0, 255];
    let planned = parse(&plan_rgba(&opaque, 2, 1, &target).report_json);
    assert_eq!(planned["plan"]["target"]["format"], "jpeg");
    let adapted = adapt_rgba(
        &opaque,
        2,
        1,
        &target,
        r#"{"crop":null,"crop_consent":false}"#,
    );
    assert_eq!(parse(&adapted.report_json)["status"], "adapted");
    assert_eq!(&adapted.output.expect("JPEG output")[..3], b"\xff\xd8\xff");

    let translucent = [255_u8, 0, 0, 128, 0, 255, 0, 255];
    let blocked = parse(&plan_rgba(&translucent, 2, 1, &target).report_json);
    assert_eq!(blocked["code"], "NO_VALID_PLAN");
}

#[test]
fn video_bytes_stay_explicitly_unsupported() {
    let mut bytes = vec![0_u8; 12];
    bytes[4..8].copy_from_slice(b"ftyp");
    bytes[8..12].copy_from_slice(b"isom");
    let inspected = parse(&inspect_bytes(&bytes));
    assert_eq!(inspected["schema"], "fitifact.error/v1");
    assert_eq!(inspected["code"], "INSPECTION_UNSUPPORTED");
}

#[test]
fn crate_surface_never_constructs_a_media_runtime() {
    let lib = include_str!("../src/lib.rs");
    assert!(!lib.contains("FfmpegProvider"));
    assert!(!lib.to_ascii_lowercase().contains("ffmpeg.wasm"));
}
