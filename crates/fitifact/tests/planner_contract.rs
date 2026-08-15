use fitifact::artifact::{
    Artifact, AudioCodec, Container, HdrStatus, OtherStream, Stream, StreamDetails, VideoCodec,
};
use fitifact::capability::{TransformId, default_catalog};
use fitifact::constraints::{ConstraintInput, compile, media_h264_mp4_aac};
use fitifact::plan::{BlockingCode, PLAN_SCHEMA, PLANNER_VERSION, PreservationClaim, plan};

fn blocking(artifact: &Artifact, constraints: &fitifact::ConstraintSet) -> Vec<BlockingCode> {
    plan(artifact, constraints, &default_catalog()).blocking_codes()
}

#[test]
fn preserves_the_three_canonical_outcomes() {
    let target = media_h264_mp4_aac();
    let compatible = Artifact::media(
        Container::Mp4,
        VideoCodec::H264,
        Some(AudioCodec::Aac),
        1000,
    );
    assert!(plan(&compatible, &target, &default_catalog()).is_compatible());

    let mov = Artifact::media(
        Container::Mov,
        VideoCodec::H264,
        Some(AudioCodec::Aac),
        1000,
    );
    let remux = plan(&mov, &target, &default_catalog());
    assert_eq!(remux.steps()[0].operation, TransformId::Remux);

    let hevc = Artifact::media(
        Container::Mp4,
        VideoCodec::Hevc,
        Some(AudioCodec::Aac),
        1000,
    );
    let transcode = plan(&hevc, &target, &default_catalog());
    let step = &transcode.steps()[0];
    assert_eq!(step.operation, TransformId::TranscodeVideo);
    assert_eq!(step.target.container, Some(Container::Mp4));
    assert_eq!(step.target.video_codec, Some(VideoCodec::H264));
    assert!(
        step.preservation
            .contains(&PreservationClaim::AudioStreamCopied)
    );
    assert!(
        step.preservation
            .contains(&PreservationClaim::VideoDimensions)
    );
}

#[test]
fn plan_json_is_versioned_typed_and_provider_neutral() {
    let artifact = Artifact::media(
        Container::Mp4,
        VideoCodec::Hevc,
        Some(AudioCodec::Aac),
        1000,
    );
    let outcome = plan(&artifact, &media_h264_mp4_aac(), &default_catalog());
    let value = serde_json::to_value(&outcome).unwrap();
    assert_eq!(value["schema"], PLAN_SCHEMA);
    assert_eq!(value["planner_version"], PLANNER_VERSION);
    assert_eq!(value["kind"], "planned");
    assert_eq!(
        value["plan"]["steps"][0]["operation"],
        "media.transcode_video"
    );
    assert!(value["plan"]["steps"][0]["target"].is_object());
    assert!(value["plan"]["steps"][0]["reasons"].is_array());
    assert!(value["plan"]["steps"][0]["expected"].is_array());
    assert!(value["plan"]["steps"][0]["preservation"].is_array());
    assert!(value["plan"]["warnings"].is_array());
    let encoded = serde_json::to_string(&value).unwrap().to_ascii_lowercase();
    for forbidden in ["ffmpeg", "libx264", "argv", "shell", "command"] {
        assert!(!encoded.contains(forbidden), "plan contains {forbidden}");
    }
}

#[test]
fn refuses_additional_or_non_av_stream_topologies() {
    let target = media_h264_mp4_aac();
    for details in [
        StreamDetails::Video {
            facts: fitifact::VideoStream {
                codec: Some(VideoCodec::H264),
                width: Some(16),
                height: Some(16),
                frame_rate: None,
                pixel_format: Some("yuv420p".into()),
                bit_depth: Some(8),
                color_range: None,
                color_space: None,
                color_transfer: Some("bt709".into()),
                color_primaries: None,
                hdr: HdrStatus::Sdr,
            },
        },
        StreamDetails::Audio {
            facts: fitifact::AudioStream {
                codec: Some(AudioCodec::Aac),
                channels: Some(2),
                sample_rate: Some(48_000),
            },
        },
        StreamDetails::Subtitle {
            facts: OtherStream { codec: None },
        },
        StreamDetails::Data {
            facts: OtherStream { codec: None },
        },
        StreamDetails::Attachment {
            facts: OtherStream { codec: None },
        },
        StreamDetails::Unknown {
            original_type: Some("mystery".into()),
            facts: OtherStream { codec: None },
        },
    ] {
        let mut artifact = Artifact::media(
            Container::Mp4,
            VideoCodec::H264,
            Some(AudioCodec::Aac),
            1000,
        );
        artifact.streams.push(Stream {
            index: Some(99),
            details,
        });
        assert_eq!(
            blocking(&artifact, &target),
            vec![BlockingCode::UnsafeStreamTopology]
        );
    }
}

#[test]
fn refuses_hdr_or_unknown_hdr_video_transcoding() {
    for hdr in [HdrStatus::Hdr, HdrStatus::Unknown] {
        let mut artifact = Artifact::media(
            Container::Mp4,
            VideoCodec::Hevc,
            Some(AudioCodec::Aac),
            1000,
        );
        artifact.first_video_mut().unwrap().hdr = hdr;
        assert_eq!(
            blocking(&artifact, &media_h264_mp4_aac()),
            vec![BlockingCode::HdrConversionUnsupported]
        );
    }
}

#[test]
fn refuses_high_or_unknown_bit_depth_video_transcoding() {
    for bit_depth in [Some(10), None] {
        let mut artifact = Artifact::media(
            Container::Mp4,
            VideoCodec::Hevc,
            Some(AudioCodec::Aac),
            1000,
        );
        artifact.first_video_mut().unwrap().bit_depth = bit_depth;
        assert_eq!(
            blocking(&artifact, &media_h264_mp4_aac()),
            vec![BlockingCode::BitDepthConversionUnsupported]
        );
    }
}

#[test]
fn refuses_non_mp4_targets_and_audio_transcoding() {
    let artifact = Artifact::media(
        Container::Mov,
        VideoCodec::H264,
        Some(AudioCodec::Mp3),
        1000,
    );
    let mov_target = compile(ConstraintInput {
        container: Some(vec!["mov".into()]),
        video_codec: Some(vec!["h264".into()]),
        audio_codec: Some(vec!["aac".into()]),
        ..ConstraintInput::default()
    })
    .unwrap();
    assert!(blocking(&artifact, &mov_target).contains(&BlockingCode::NonMp4Target));

    let mp4_target = media_h264_mp4_aac();
    assert!(blocking(&artifact, &mp4_target).contains(&BlockingCode::AudioTranscodeUnsupported));
}

#[test]
fn refuses_resize_and_size_fitting() {
    let artifact = Artifact::media(
        Container::Mp4,
        VideoCodec::H264,
        Some(AudioCodec::Aac),
        1000,
    );
    let dimensions = compile(ConstraintInput {
        max_width: Some(1000),
        ..ConstraintInput::default()
    })
    .unwrap();
    assert_eq!(
        blocking(&artifact, &dimensions),
        vec![BlockingCode::ResizeUnsupported]
    );

    let size = compile(ConstraintInput {
        max_bytes: Some(999),
        ..ConstraintInput::default()
    })
    .unwrap();
    assert_eq!(
        blocking(&artifact, &size),
        vec![BlockingCode::SizeFittingUnsupported]
    );
}

#[test]
fn refuses_unsupported_or_unknown_video_codecs() {
    for codec in [
        VideoCodec::Vp9,
        VideoCodec::Av1,
        VideoCodec::Unknown("x".into()),
    ] {
        let artifact = Artifact::media(Container::Mp4, codec, Some(AudioCodec::Aac), 1000);
        assert_eq!(
            blocking(&artifact, &media_h264_mp4_aac()),
            vec![BlockingCode::UnsupportedVideoCodec]
        );
    }
}

#[test]
fn refuses_a_missing_video_codec_as_an_unknown_required_fact() {
    let mut artifact = Artifact::media(
        Container::Mp4,
        VideoCodec::H264,
        Some(AudioCodec::Aac),
        1000,
    );
    artifact.first_video_mut().unwrap().codec = None;
    assert_eq!(
        blocking(&artifact, &media_h264_mp4_aac()),
        vec![BlockingCode::UnknownRequiredFact]
    );
}

#[test]
fn refuses_when_a_transform_makes_a_passing_size_fact_uncertain() {
    let artifact = Artifact::media(
        Container::Mov,
        VideoCodec::H264,
        Some(AudioCodec::Aac),
        1000,
    );
    let target = compile(ConstraintInput {
        container: Some(vec!["mp4".into()]),
        video_codec: Some(vec!["h264".into()]),
        audio_codec: Some(vec!["aac".into()]),
        max_bytes: Some(2000),
        ..ConstraintInput::default()
    })
    .unwrap();
    assert_eq!(
        blocking(&artifact, &target),
        vec![BlockingCode::UncertainPostTransformSize]
    );
}
