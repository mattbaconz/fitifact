use fitifact_wasm::{adapt_bytes, inspect_bytes, plan_bytes, sample_jpeg_rgb, sample_png_rgb};

fn parse(json: &str) -> serde_json::Value {
    serde_json::from_str(json).expect(json)
}

#[test]
fn jpeg_noop_does_not_load_a_media_runtime() {
    let report = parse(&adapt_bytes(&sample_jpeg_rgb(8, 8)).report_json);
    assert_eq!(report["status"], "compatible");
    assert_eq!(report["media_runtime_loaded"], false);
    assert!(adapt_bytes(&sample_jpeg_rgb(8, 8)).output.is_none());
}

#[test]
fn png_encodes_jpeg_without_a_media_runtime() {
    let adapted = adapt_bytes(&sample_png_rgb(8, 8));
    let report = parse(&adapted.report_json);
    assert_eq!(report["status"], "adapted");
    assert_eq!(report["media_runtime_loaded"], false);
    let output = adapted.output.expect("jpeg bytes");
    let inspected = parse(&inspect_bytes(&output));
    assert_eq!(inspected["image"]["format"], "jpeg");
}

#[test]
fn video_bytes_tell_the_caller_to_use_the_cli() {
    let mut bytes = vec![0_u8; 12];
    bytes[4..8].copy_from_slice(b"ftyp");
    bytes[8..12].copy_from_slice(b"isom");
    let inspected = parse(&inspect_bytes(&bytes));
    assert_eq!(inspected["schema"], "fitifact.error/v1");
    assert_eq!(inspected["code"], "INSPECTION_UNSUPPORTED");
    assert!(
        inspected["message"]
            .as_str()
            .unwrap()
            .contains("Fitifact CLI")
    );
}

#[test]
fn planned_png_is_jpeg_encode() {
    let outcome = parse(&plan_bytes(&sample_png_rgb(8, 8)));
    assert_eq!(outcome["kind"], "planned");
    assert_eq!(
        outcome["plan"]["steps"][0]["operation"],
        "image.encode_jpeg"
    );
}

#[test]
fn crate_and_web_surface_never_ship_a_media_runtime() {
    let lib = include_str!("../src/lib.rs");
    let html = include_str!("../../../web/index.html");
    let js = include_str!("../../../web/app.js");
    for source in [lib, html, js] {
        let lowered = source.to_ascii_lowercase();
        assert!(!lowered.contains("ffmpeg"));
        assert!(!lowered.contains("ffmpeg.wasm"));
        assert!(!source.contains("FfmpegProvider"));
    }
    assert!(html.contains("Uploads to Fitifact: 0 bytes"));
    assert!(!js.contains("https://"));
    assert!(!html.contains("https://"));
    assert!(!js.contains("http://"));
    assert!(!html.contains("http://"));
    assert!(js.contains("./pkg/fitifact_wasm.js"));
    assert!(!js.contains("fetch("));
}
