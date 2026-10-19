use fitifact_wasm::{adapt_media_probe, compile_profile, inspect_media_probe, plan_media_probe};
use serde_json::{Value, json};

const REMUX_ONLY: &str = r#"["media.remux"]"#;

fn parse(json: &str) -> Value {
    serde_json::from_str(json).expect(json)
}

/// The avcC payload of a tracked fixture, found by its box type.
fn fixture_avcc(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/media")
        .join(name);
    let bytes = std::fs::read(path).unwrap();
    let at = bytes.windows(4).position(|w| w == b"avcC").unwrap();
    let size = u32::from_be_bytes(bytes[at - 4..at].try_into().unwrap()) as usize;
    bytes[at + 4..at - 4 + size].to_vec()
}

/// What the browser worker reports for a 320x240, 1 s, H.264 (or HEVC) plus AAC file.
fn probe(name: &str, container: &str, brand: Option<&str>, codec: &str, extra: &[Value]) -> Value {
    let avcc = (codec == "avc").then(|| fixture_avcc("remux-h264-aac.mov"));
    let mut tracks = vec![
        json!({ "type": "video", "codec": codec, "width": 320, "height": 240, "avcc": avcc,
                "color": { "primaries": "bt709", "transfer": "bt709", "matrix": "bt709", "full_range": false } }),
        json!({ "type": "audio", "codec": "aac", "channels": 2, "sample_rate": 48000 }),
    ];
    tracks.extend_from_slice(extra);
    json!({
        "schema": "fitifact.browser-probe/v1",
        "provider_version": "1.61.1",
        "name": name,
        "byte_length": 13_600,
        "container": container,
        "major_brand": brand,
        "duration_ms": 1000,
        "tracks": tracks,
    })
}

fn generic() -> String {
    compile_profile("generic/video-upload")
}

/// SHA-256 hex digests of each stream's packets; `video` picks the video digest.
fn hashes(video: char) -> Value {
    json!({ "algorithm": "sha256", "video": video.to_string().repeat(64), "audio": "c".repeat(64) })
}

fn geometry(rotation: u32) -> Value {
    json!({ "display_width": 320, "display_height": 240, "rotation": rotation, "flip": false })
}

#[test]
fn compatible_mp4_is_a_no_op() {
    let source = probe("clip.mp4", "isobmff", Some("isom"), "avc", &[]).to_string();
    let plan = parse(&plan_media_probe(&source, &generic(), REMUX_ONLY));
    assert_eq!(plan["schema"], "fitifact.web-media-plan/v1");
    assert_eq!(plan["outcome"]["kind"], "compatible", "{plan}");
    assert_eq!(plan["report"]["compatible"], true);
    assert_eq!(plan["needs_encoder"], false);
    assert_eq!(plan["inspection"]["inspection"]["provider"], "mediabunny");

    let result = parse(&adapt_media_probe(&source, &generic(), REMUX_ONLY, "null"));
    assert_eq!(result["status"], "compatible");
    assert_eq!(result["output"], Value::Null);
}

#[test]
fn mov_and_mkv_with_h264_aac_plan_a_remux() {
    for (name, container, brand) in [
        ("clip.mov", "isobmff", Some("qt  ")),
        ("clip.mkv", "matroska", None),
    ] {
        let source = probe(name, container, brand, "avc", &[]).to_string();
        let plan = parse(&plan_media_probe(&source, &generic(), REMUX_ONLY));
        assert_eq!(plan["outcome"]["kind"], "planned", "{name}: {plan}");
        assert_eq!(
            plan["outcome"]["plan"]["steps"][0]["operation"],
            "media.remux"
        );
        assert_eq!(
            plan["outcome"]["plan"]["preserved"][0],
            "all_streams_copied"
        );
    }
}

#[test]
fn remuxed_output_must_validate_before_it_counts() {
    let source = probe("clip.mov", "isobmff", Some("qt  "), "avc", &[]).to_string();
    let output = |video_hash: char| {
        json!({
            "name": "clip.fitifact.mp4",
            "probe": probe("clip.fitifact.mp4", "isobmff", Some("isom"), "avc", &[]),
            "source_hashes": hashes('a'),
            "output_hashes": hashes(video_hash),
            "source_geometry": geometry(0),
            "output_geometry": geometry(0),
        })
        .to_string()
    };

    let adapted = parse(&adapt_media_probe(
        &source,
        &generic(),
        REMUX_ONLY,
        &output('a'),
    ));
    assert_eq!(adapted["status"], "adapted", "{adapted}");
    assert_eq!(adapted["output"], "clip.fitifact.mp4");
    assert_eq!(adapted["validation"]["status"], "pass");
    assert_eq!(adapted["validation"]["artifact"]["container"], "mp4");

    let altered = parse(&adapt_media_probe(
        &source,
        &generic(),
        REMUX_ONLY,
        &output('b'),
    ));
    assert_eq!(altered["status"], "failed");
    assert_eq!(altered["error"]["code"], "VALIDATION_FAILED");
    assert_eq!(altered["output"], Value::Null);

    let missing = parse(&adapt_media_probe(&source, &generic(), REMUX_ONLY, "null"));
    assert_eq!(missing["status"], "failed");
    assert_eq!(missing["error"]["code"], "PROVIDER_MISSING");

    let unreadable = json!({
        "name": "clip.fitifact.mp4",
        "probe": { "schema": "fitifact.browser-probe/v1" },
        "source_hashes": hashes('a'),
        "output_hashes": hashes('a'),
        "source_geometry": geometry(0),
        "output_geometry": geometry(0),
    });
    let refused = parse(&adapt_media_probe(
        &source,
        &generic(),
        REMUX_ONLY,
        &unreadable.to_string(),
    ));
    assert_eq!(refused["schema"], "fitifact.error/v1");
    assert_eq!(refused["code"], "VALIDATION_FAILED");
}

#[test]
fn an_encode_the_browser_lacks_is_flagged_not_planned() {
    let source = probe("clip.mp4", "isobmff", Some("isom"), "hevc", &[]).to_string();
    let plan = parse(&plan_media_probe(&source, &generic(), REMUX_ONLY));
    assert_eq!(plan["outcome"]["kind"], "cannot_satisfy", "{plan}");
    assert_eq!(plan["needs_encoder"], true);
}

#[test]
fn tracks_the_browser_cannot_copy_are_refused_like_native() {
    let source = probe(
        "clip.mov",
        "isobmff",
        Some("qt  "),
        "avc",
        &[json!({ "type": "data" })],
    )
    .to_string();
    let plan = parse(&plan_media_probe(&source, &generic(), REMUX_ONLY));
    assert_eq!(plan["outcome"]["kind"], "cannot_satisfy");
    assert_eq!(
        plan["outcome"]["blocking"][0]["code"],
        "unsafe_stream_topology"
    );
    assert_eq!(plan["needs_encoder"], false);
    let inspected = parse(&inspect_media_probe(&source));
    assert_eq!(inspected["streams"].as_array().unwrap().len(), 3);
}

#[test]
fn malformed_inputs_return_structured_errors() {
    let source = probe("clip.mov", "isobmff", Some("qt  "), "avc", &[]).to_string();
    let bad_transforms = parse(&plan_media_probe(
        &source,
        &generic(),
        r#"["media.teleport"]"#,
    ));
    assert_eq!(bad_transforms["code"], "INPUT_INVALID");
    let bad_probe = parse(&inspect_media_probe("{}"));
    assert_eq!(bad_probe["code"], "INPUT_INVALID");
}

#[test]
fn a_copy_shown_differently_or_without_real_digests_is_not_adapted() {
    let source = probe("clip.mov", "isobmff", Some("qt  "), "avc", &[]).to_string();
    let output = |source_hashes: Value, rotation: u32| {
        json!({
            "name": "clip.fitifact.mp4",
            "probe": probe("clip.fitifact.mp4", "isobmff", Some("isom"), "avc", &[]),
            "source_hashes": source_hashes,
            "output_hashes": hashes('a'),
            "source_geometry": geometry(0),
            "output_geometry": geometry(rotation),
        })
        .to_string()
    };

    let turned = parse(&adapt_media_probe(
        &source,
        &generic(),
        REMUX_ONLY,
        &output(hashes('a'), 90),
    ));
    assert_eq!(turned["status"], "failed", "{turned}");
    assert_eq!(turned["error"]["code"], "VALIDATION_FAILED");
    let rows = turned["validation"]["integrity"].as_array().unwrap();
    let row = rows
        .iter()
        .find(|row| row["id"] == "display_geometry")
        .unwrap();
    assert_eq!(row["status"], "fail");
    assert_eq!(row["actual"], "320x240, rotated 90, flipped false");

    // Equal but empty digests would prove nothing.
    let empty = json!({ "algorithm": "sha256", "video": "", "audio": null });
    let refused = parse(&adapt_media_probe(
        &source,
        &generic(),
        REMUX_ONLY,
        &output(empty, 0),
    ));
    assert_eq!(refused["code"], "VALIDATION_FAILED", "{refused}");
}

#[test]
fn the_browser_never_plans_an_encode_even_when_offered_one() {
    let source = probe("clip.mp4", "isobmff", Some("isom"), "hevc", &[]).to_string();
    let every = r#"["media.remux", "media.transcode_video", "media.transcode_audio"]"#;
    let plan = parse(&plan_media_probe(&source, &generic(), every));
    assert_eq!(plan["outcome"]["kind"], "cannot_satisfy", "{plan}");
    assert_eq!(plan["needs_encoder"], true);
}
