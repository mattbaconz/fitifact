use fitifact_wasm::{adapt_media_probe, compile_profile, inspect_media_probe, plan_media_probe};
use serde_json::{Value, json};

const NOT_MEASURED: &str = "null";

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

/// A record whose 64x64 baseline SPS tags BT.709 limited range in its VUI, as Chromium's H.264
/// encoder writes for BT.709 frames.
const BT709_AVCC: [u8; 25] = [
    0x01, 0x42, 0x00, 0x28, 0xff, 0xe1, 0x00, 0x0c, 0x67, 0x42, 0x00, 0x28, 0xf4, 0x21, 0x34, 0xd4,
    0x04, 0x04, 0x04, 0x08, 0x01, 0x00, 0x02, 0x68, 0xce,
];

const BT709: &str =
    r#"{ "primaries": "bt709", "transfer": "bt709", "matrix": "bt709", "full_range": false }"#;

/// What the browser worker reports for a 320x240, 1 s, H.264 (or HEVC) plus AAC file.
fn probe(name: &str, container: &str, brand: Option<&str>, codec: &str, extra: &[Value]) -> Value {
    let avcc = (codec == "avc").then(|| fixture_avcc("remux-h264-aac.mov"));
    let mut tracks = vec![
        json!({ "type": "video", "codec": codec, "width": 320, "height": 240, "avcc": avcc,
                "color": parse(BT709) }),
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

/// A 1 s WebM with VP9 video and Opus audio, which needs both streams re-encoded.
fn vp9_opus() -> String {
    json!({
        "schema": "fitifact.browser-probe/v1",
        "provider_version": "1.61.1",
        "name": "clip.webm",
        "byte_length": 13_600,
        "container": "matroska",
        "major_brand": null,
        "duration_ms": 1000,
        "tracks": [
            { "type": "video", "codec": "vp9", "width": 320, "height": 240, "avcc": null, "color": parse(BT709) },
            { "type": "audio", "codec": "opus", "channels": 2, "sample_rate": 48000 },
        ],
    })
    .to_string()
}

fn generic() -> String {
    compile_profile("generic/video-upload")
}

/// The generic target with a different byte ceiling and any extra hard constraints.
fn generic_with(max_bytes: u64, extra: &[Value]) -> String {
    let mut target = parse(&generic());
    let hard = target["hard"].as_array_mut().unwrap();
    for constraint in hard.iter_mut() {
        if constraint["field"] == "file.bytes" {
            constraint["value"] = json!(max_bytes);
        }
    }
    hard.extend_from_slice(extra);
    target.to_string()
}

/// SHA-256 hex digests of each stream's packets; `video` picks the video digest.
fn hashes(video: char) -> Value {
    json!({ "algorithm": "sha256", "video": video.to_string().repeat(64), "audio": "c".repeat(64) })
}

fn geometry(rotation: u32) -> Value {
    json!({ "display_width": 320, "display_height": 240, "rotation": rotation, "flip": false })
}

/// What a capable browser measures: it decodes both streams, and its H.264 encoder writes a
/// sound BT.709 record.
fn capable() -> Value {
    json!({
        "geometry": geometry(0),
        "video_decode": true,
        "h264_sample": BT709_AVCC,
        "audio_decode": true,
        "aac_encode": true,
    })
}

fn measured(change: Value) -> String {
    let mut facts = capable();
    for (key, value) in change.as_object().unwrap() {
        facts[key] = value.clone();
    }
    facts.to_string()
}

/// The fixture's record with its SPS behind a doubled NAL header, as Firefox's encoder writes it.
fn doubled_header_sample() -> Vec<u8> {
    let record = fixture_avcc("remux-h264-aac.mov");
    let length = usize::from(u16::from_be_bytes([record[6], record[7]]));
    let mut doubled = record[..6].to_vec();
    doubled.extend_from_slice(&u16::try_from(length + 1).unwrap().to_be_bytes());
    doubled.push(record[8]);
    doubled.extend_from_slice(&record[8..]);
    doubled
}

#[test]
fn compatible_mp4_is_a_no_op() {
    let source = probe("clip.mp4", "isobmff", Some("isom"), "avc", &[]).to_string();
    let plan = parse(&plan_media_probe(&source, &generic(), NOT_MEASURED));
    assert_eq!(plan["schema"], "fitifact.web-media-plan/v2");
    assert_eq!(plan["outcome"]["kind"], "compatible", "{plan}");
    assert_eq!(plan["report"]["compatible"], true);
    assert_eq!(plan["browser"], Value::Null);
    assert_eq!(plan["inspection"]["inspection"]["provider"], "mediabunny");

    let result = parse(&adapt_media_probe(&source, &generic(), "null"));
    assert_eq!(result["status"], "compatible");
    assert_eq!(result["output"], Value::Null);
}

#[test]
fn mov_and_mkv_with_h264_aac_plan_a_remux_without_measuring() {
    for (name, container, brand) in [
        ("clip.mov", "isobmff", Some("qt  ")),
        ("clip.mkv", "matroska", None),
    ] {
        let source = probe(name, container, brand, "avc", &[]).to_string();
        let plan = parse(&plan_media_probe(&source, &generic(), NOT_MEASURED));
        assert_eq!(plan["outcome"]["kind"], "planned", "{name}: {plan}");
        assert_eq!(
            plan["outcome"]["plan"]["steps"][0]["operation"],
            "media.remux"
        );
        assert_eq!(
            plan["outcome"]["plan"]["preserved"][0],
            "all_streams_copied"
        );
        assert_eq!(plan["browser"]["status"], "ready");
        assert_eq!(plan["browser"]["execution"]["operation"], "media.remux");
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

    let adapted = parse(&adapt_media_probe(&source, &generic(), &output('a')));
    assert_eq!(adapted["status"], "adapted", "{adapted}");
    assert_eq!(adapted["output"], "clip.fitifact.mp4");
    assert_eq!(adapted["validation"]["status"], "pass");
    assert_eq!(adapted["validation"]["artifact"]["container"], "mp4");

    let altered = parse(&adapt_media_probe(&source, &generic(), &output('b')));
    assert_eq!(altered["status"], "failed");
    assert_eq!(altered["error"]["code"], "VALIDATION_FAILED");
    assert_eq!(altered["output"], Value::Null);

    let missing = parse(&adapt_media_probe(&source, &generic(), "null"));
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
        &unreadable.to_string(),
    ));
    assert_eq!(refused["schema"], "fitifact.error/v1");
    assert_eq!(refused["code"], "VALIDATION_FAILED");
}

#[test]
fn an_hevc_source_is_measured_then_encoded_with_the_quality_ladder() {
    let source = probe("clip.mp4", "isobmff", Some("isom"), "hevc", &[]).to_string();
    let plan = parse(&plan_media_probe(&source, &generic(), NOT_MEASURED));
    assert_eq!(plan["outcome"]["kind"], "planned", "{plan}");
    assert_eq!(
        plan["outcome"]["plan"]["steps"][0]["operation"],
        "media.transcode_video"
    );
    assert_eq!(plan["browser"]["status"], "measure");
    assert_eq!(
        plan["browser"]["measure"],
        json!({ "video": { "codec": "avc", "width": 320, "height": 240, "bitrate": 230_400 }, "audio": null })
    );

    let plan = parse(&plan_media_probe(
        &source,
        &generic(),
        &capable().to_string(),
    ));
    assert_eq!(plan["browser"]["status"], "ready", "{plan}");
    assert_eq!(
        plan["browser"]["execution"],
        json!({
            "operation": "media.transcode_video",
            "width": 320,
            "height": 240,
            "video_bitrates": [230_400, 172_800, 129_600, 97_200, 80_000],
            "audio": "copy",
            "max_bytes": 25_000_000,
        })
    );
}

#[test]
fn an_oversized_h264_file_runs_the_size_ladder_and_scales_to_fit_limits() {
    let mut source = probe("clip.mp4", "isobmff", Some("isom"), "avc", &[]);
    source["byte_length"] = json!(400_000);
    let source = source.to_string();

    // 40 KB over 1 s leaves 288 kb/s minus the assumed soundtrack, under the quality bitrate.
    let plan = parse(&plan_media_probe(
        &source,
        &generic_with(40_000, &[]),
        &capable().to_string(),
    ));
    let execution = &plan["browser"]["execution"];
    assert_eq!(
        execution["video_bitrates"],
        json!([128_000, 96_000, 80_000]),
        "{plan}"
    );
    assert_eq!(execution["max_bytes"], 40_000);
    assert_eq!(
        (execution["width"].clone(), execution["height"].clone()),
        (json!(320), json!(240))
    );

    let narrow = json!({ "id": "width", "field": "media.video.width", "op": "lte", "value": 160 });
    let plan = parse(&plan_media_probe(
        &source,
        &generic_with(25_000_000, &[narrow]),
        NOT_MEASURED,
    ));
    assert_eq!(
        plan["browser"]["measure"]["video"],
        json!({ "codec": "avc", "width": 160, "height": 120, "bitrate": 80_000 }),
        "{plan}"
    );
}

#[test]
fn what_the_browser_lacks_is_refused_with_plain_codes() {
    let hevc = probe("clip.mp4", "isobmff", Some("isom"), "hevc", &[]).to_string();
    let missing = |source: &str, change: Value| {
        let plan = parse(&plan_media_probe(source, &generic(), &measured(change)));
        assert_eq!(plan["browser"]["status"], "refused", "{plan}");
        plan["browser"]["missing"].clone()
    };
    assert_eq!(
        missing(&hevc, json!({ "video_decode": false })),
        json!(["video_decoder"])
    );
    assert_eq!(
        missing(&hevc, json!({ "h264_sample": null })),
        json!(["h264_encoder"])
    );
    assert_eq!(
        missing(&hevc, json!({ "h264_sample": doubled_header_sample() })),
        json!(["h264_encoder_output"])
    );
    assert_eq!(
        missing(&hevc, json!({ "geometry": geometry(90) })),
        json!(["display_transform"])
    );
    let stretched =
        json!({ "display_width": 427, "display_height": 240, "rotation": 0, "flip": false });
    assert_eq!(
        missing(&hevc, json!({ "geometry": stretched })),
        json!(["display_transform"])
    );

    let webm = vp9_opus();
    let plan = parse(&plan_media_probe(&webm, &generic(), NOT_MEASURED));
    assert_eq!(
        plan["browser"]["measure"]["audio"],
        json!({ "codec": "aac", "channels": 2, "sample_rate": 48000, "bitrate": 160_000 }),
        "{plan}"
    );
    assert_eq!(
        missing(&webm, json!({ "aac_encode": false, "audio_decode": false })),
        json!(["audio_decoder", "aac_encoder"])
    );
    let plan = parse(&plan_media_probe(&webm, &generic(), &capable().to_string()));
    assert_eq!(
        plan["browser"]["execution"]["audio"],
        json!({ "codec": "aac", "channels": 2, "bitrate": 160_000 })
    );
}

#[test]
fn hdr_and_audio_only_changes_are_refused_before_anything_is_measured() {
    let mut hdr = probe("clip.mp4", "isobmff", Some("isom"), "hevc", &[]);
    hdr["tracks"][0]["color"] = json!({ "primaries": "bt2020", "transfer": "pq", "matrix": "bt2020-ncl", "full_range": false });
    let plan = parse(&plan_media_probe(
        &hdr.to_string(),
        &generic(),
        NOT_MEASURED,
    ));
    assert_eq!(plan["outcome"]["kind"], "planned", "{plan}");
    assert_eq!(
        plan["browser"],
        json!({ "status": "refused", "missing": ["hdr"] })
    );

    let mut opus = probe("clip.mkv", "matroska", None, "avc", &[]);
    opus["tracks"][1]["codec"] = json!("opus");
    let opus = opus.to_string();
    let plan = parse(&plan_media_probe(&opus, &generic(), NOT_MEASURED));
    assert_eq!(
        plan["outcome"]["plan"]["steps"][0]["operation"], "media.transcode_audio",
        "{plan}"
    );
    assert_eq!(
        plan["browser"],
        json!({ "status": "refused", "missing": ["audio_only_encode"] })
    );
    let output = json!({
        "name": "clip.fitifact.mp4",
        "probe": probe("clip.fitifact.mp4", "isobmff", Some("isom"), "avc", &[]),
        "source_hashes": hashes('a'),
        "output_hashes": hashes('a'),
        "source_geometry": geometry(0),
        "output_geometry": geometry(0),
    });
    let refused = parse(&adapt_media_probe(&opus, &generic(), &output.to_string()));
    assert_eq!(refused["code"], "PROVIDER_MISSING", "{refused}");
}

#[test]
fn an_encoded_output_must_be_upright_at_the_planned_size_and_validate() {
    let source = probe("clip.mp4", "isobmff", Some("isom"), "hevc", &[]).to_string();
    let mut encoded = probe("clip.fitifact.mp4", "isobmff", Some("isom"), "avc", &[]);
    encoded["tracks"][0]["avcc"] = json!(BT709_AVCC);
    let output = |shown: Value, produced: Value| {
        json!({
            "name": "clip.fitifact.mp4",
            "probe": encoded,
            "source_hashes": hashes('a'),
            "output_hashes": hashes('b'),
            "source_geometry": shown,
            "output_geometry": produced,
        })
        .to_string()
    };

    let adapted = parse(&adapt_media_probe(
        &source,
        &generic(),
        &output(geometry(0), geometry(0)),
    ));
    assert_eq!(adapted["status"], "adapted", "{adapted}");
    assert_eq!(adapted["validation"]["status"], "pass");

    let turned = parse(&adapt_media_probe(
        &source,
        &generic(),
        &output(geometry(0), geometry(90)),
    ));
    assert_eq!(turned["status"], "failed", "{turned}");
    let rows = turned["validation"]["integrity"].as_array().unwrap();
    let row = rows
        .iter()
        .find(|row| row["id"] == "display_geometry")
        .unwrap();
    assert_eq!(row["expected"], "320x240, rotated 0, flipped false");

    let from_rotated = parse(&adapt_media_probe(
        &source,
        &generic(),
        &output(geometry(90), geometry(0)),
    ));
    assert_eq!(from_rotated["code"], "VALIDATION_FAILED", "{from_rotated}");
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
    let plan = parse(&plan_media_probe(&source, &generic(), NOT_MEASURED));
    assert_eq!(plan["outcome"]["kind"], "cannot_satisfy");
    assert_eq!(
        plan["outcome"]["blocking"][0]["code"],
        "unsafe_stream_topology"
    );
    assert_eq!(plan["browser"], Value::Null);
    let inspected = parse(&inspect_media_probe(&source));
    assert_eq!(inspected["streams"].as_array().unwrap().len(), 3);
}

#[test]
fn malformed_inputs_return_structured_errors() {
    let source = probe("clip.mov", "isobmff", Some("qt  "), "avc", &[]).to_string();
    let bad_measurement = parse(&plan_media_probe(
        &source,
        &generic(),
        r#"{ "video_decode": "yes" }"#,
    ));
    assert_eq!(bad_measurement["code"], "INPUT_INVALID");
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
    let refused = parse(&adapt_media_probe(&source, &generic(), &output(empty, 0)));
    assert_eq!(refused["code"], "VALIDATION_FAILED", "{refused}");
}
