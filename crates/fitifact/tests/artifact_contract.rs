use std::path::Path;

use fitifact::artifact::{ARTIFACT_SCHEMA, HdrStatus, Rational, StreamType, VideoCodec};
use fitifact::inspect::artifact_from_ffprobe_json;

#[test]
fn normalizes_every_ffprobe_stream_and_provider_version() {
    let json = r#"{
        "program_version": {"version": "7.1.2"},
        "streams": [
            {
                "index": 0,
                "codec_type": "video",
                "codec_name": "hevc",
                "width": 3840,
                "height": 2160,
                "avg_frame_rate": "30000/1001",
                "pix_fmt": "yuv420p10le",
                "bits_per_raw_sample": "10",
                "color_range": "tv",
                "color_space": "bt2020nc",
                "color_transfer": "smpte2084",
                "color_primaries": "bt2020"
            },
            {"index": 1, "codec_type": "audio", "codec_name": "aac", "channels": 2, "sample_rate": "48000"},
            {"index": 2, "codec_type": "subtitle", "codec_name": "mov_text"},
            {"index": 3, "codec_type": "data", "codec_name": "bin_data"},
            {"index": 4, "codec_type": "attachment", "codec_name": "ttf"},
            {"codec_type": "mystery", "codec_name": "opaque"}
        ],
        "format": {
            "format_name": "mov,mp4,m4a,3gp,3g2,mj2",
            "duration": "1.0",
            "tags": {"major_brand": "isom"}
        }
    }"#;

    let artifact = artifact_from_ffprobe_json(Path::new("video.mp4"), 1000, json).unwrap();
    assert_eq!(artifact.streams.len(), 6);
    assert_eq!(
        artifact.inspection.provider_version.as_deref(),
        Some("7.1.2")
    );
    assert_eq!(artifact.streams[0].stream_type(), StreamType::Video);
    assert_eq!(artifact.streams[1].stream_type(), StreamType::Audio);
    assert_eq!(artifact.streams[2].stream_type(), StreamType::Subtitle);
    assert_eq!(artifact.streams[3].stream_type(), StreamType::Data);
    assert_eq!(artifact.streams[4].stream_type(), StreamType::Attachment);
    assert!(matches!(
        artifact.streams[5].stream_type(),
        StreamType::Unknown(_)
    ));
    assert_eq!(artifact.streams[5].index, None);

    let video = artifact.video_streams().next().unwrap();
    assert_eq!(video.codec, Some(VideoCodec::Hevc));
    assert_eq!(video.width, Some(3840));
    assert_eq!(video.height, Some(2160));
    assert_eq!(
        video.frame_rate,
        Some(Rational::new(30_000, 1_001).unwrap())
    );
    assert_eq!(video.pixel_format.as_deref(), Some("yuv420p10le"));
    assert_eq!(video.bit_depth, Some(10));
    assert_eq!(video.color_range.as_deref(), Some("tv"));
    assert_eq!(video.color_space.as_deref(), Some("bt2020nc"));
    assert_eq!(video.color_transfer.as_deref(), Some("smpte2084"));
    assert_eq!(video.color_primaries.as_deref(), Some("bt2020"));
    assert_eq!(video.hdr, HdrStatus::Hdr);

    let encoded = serde_json::to_value(&artifact).unwrap();
    assert_eq!(encoded["schema"], ARTIFACT_SCHEMA);
    assert_eq!(encoded["streams"][0]["type"], "video");
    assert_eq!(encoded["streams"][5]["type"], "unknown");
}

#[test]
fn keeps_missing_probe_facts_unknown() {
    let json = r#"{
        "streams": [{"index": 0, "codec_type": "video", "codec_name": "h264"}],
        "format": {"format_name": "mov,mp4", "tags": {"major_brand": "isom"}}
    }"#;
    let artifact = artifact_from_ffprobe_json(Path::new("video.mp4"), 1, json).unwrap();
    let video = artifact.video_streams().next().unwrap();
    assert_eq!(video.width, None);
    assert_eq!(video.height, None);
    assert_eq!(video.frame_rate, None);
    assert_eq!(video.pixel_format, None);
    assert_eq!(video.bit_depth, None);
    assert_eq!(video.hdr, HdrStatus::Unknown);
}

#[test]
fn derives_bit_depth_from_common_pixel_format_when_probe_omits_it() {
    let json = r#"{
        "streams": [{
            "index": 0,
            "codec_type": "video",
            "codec_name": "hevc",
            "pix_fmt": "yuv420p10le",
            "bits_per_raw_sample": "0"
        }],
        "format": {"format_name": "mov,mp4", "tags": {"major_brand": "isom"}}
    }"#;
    let artifact = artifact_from_ffprobe_json(Path::new("video.mp4"), 1, json).unwrap();
    assert_eq!(artifact.first_video().unwrap().bit_depth, Some(10));
}
