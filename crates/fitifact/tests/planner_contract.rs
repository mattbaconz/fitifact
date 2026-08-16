use fitifact::artifact::{
    Artifact, AudioCodec, Container, HdrStatus, OtherStream, Stream, StreamDetails, VideoCodec,
};
use fitifact::capability::{TransformId, default_catalog};
use fitifact::constraints::{
    ConstraintInput, Field, compile, compile_from_yaml, media_h264_mp4_aac,
};
use fitifact::plan::{BlockingCode, PLAN_SCHEMA, PLANNER_VERSION, PreservationClaim, plan};

fn blocking(artifact: &Artifact, constraints: &fitifact::ConstraintSet) -> Vec<BlockingCode> {
    plan(artifact, constraints, &default_catalog()).blocking_codes()
}

fn overlapping_target(first: &str, second: &str) -> fitifact::ConstraintSet {
    compile_from_yaml(&format!(
        "schema: fitifact.constraints/v1\nhard:\n{first}{second}preferences:\n  preserve_audio: true\n  preserve_resolution: true\n"
    ))
    .unwrap()
}

fn assert_permutations_block(
    artifact: &Artifact,
    first: &str,
    second: &str,
    expected: BlockingCode,
) {
    for constraints in [
        overlapping_target(first, second),
        overlapping_target(second, first),
    ] {
        assert_eq!(blocking(artifact, &constraints), vec![expected]);
    }
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
    for bit_depth in [Some(10), Some(6), None] {
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
fn refuses_ambiguous_mov_family_containers_instead_of_no_op() {
    for evidence in ["missing-brand", "3gp4", "unrecognized-brand"] {
        let artifact = Artifact::media(
            Container::Unknown(format!("mov-family:{evidence}")),
            VideoCodec::H264,
            Some(AudioCodec::Aac),
            1000,
        );
        let outcome = plan(&artifact, &media_h264_mp4_aac(), &default_catalog());
        assert!(!outcome.is_compatible());
        assert_eq!(
            outcome.blocking_codes(),
            vec![BlockingCode::UnsupportedContainer]
        );
    }
}

#[test]
fn refuses_every_source_outside_the_exact_v01_operation_matrix() {
    for container in [
        Container::Webm,
        Container::Mkv,
        Container::Unknown("ambiguous".into()),
    ] {
        for codec in [VideoCodec::H264, VideoCodec::Hevc] {
            let artifact = Artifact::media(
                container.clone(),
                codec.clone(),
                Some(AudioCodec::Aac),
                1000,
            );
            assert_eq!(
                blocking(&artifact, &media_h264_mp4_aac()),
                vec![BlockingCode::UnsupportedContainer],
                "source {container:?} with {codec:?} must refuse"
            );
        }
    }

    let mov_hevc = Artifact::media(
        Container::Mov,
        VideoCodec::Hevc,
        Some(AudioCodec::Aac),
        1000,
    );
    assert_eq!(
        blocking(&mov_hevc, &media_h264_mp4_aac()),
        vec![BlockingCode::UnsupportedContainer]
    );
}

#[test]
fn refuses_missing_dimensions_or_duration_instead_of_an_unexecutable_plan() {
    for (container, codec) in [
        (Container::Mov, VideoCodec::H264),
        (Container::Mp4, VideoCodec::Hevc),
    ] {
        let mut missing_width = Artifact::media(
            container.clone(),
            codec.clone(),
            Some(AudioCodec::Aac),
            1000,
        );
        missing_width.first_video_mut().unwrap().width = None;
        assert_eq!(
            blocking(&missing_width, &media_h264_mp4_aac()),
            vec![BlockingCode::UnknownRequiredFact],
            "{container:?} {codec:?} without width must refuse"
        );

        let mut missing_height = Artifact::media(
            container.clone(),
            codec.clone(),
            Some(AudioCodec::Aac),
            1000,
        );
        missing_height.first_video_mut().unwrap().height = None;
        assert_eq!(
            blocking(&missing_height, &media_h264_mp4_aac()),
            vec![BlockingCode::UnknownRequiredFact],
            "{container:?} {codec:?} without height must refuse"
        );

        let mut missing_duration = Artifact::media(
            container.clone(),
            codec.clone(),
            Some(AudioCodec::Aac),
            1000,
        );
        missing_duration.duration_ms = None;
        assert_eq!(
            blocking(&missing_duration, &media_h264_mp4_aac()),
            vec![BlockingCode::UnknownRequiredFact],
            "{container:?} {codec:?} without duration must refuse"
        );
    }
}

#[test]
fn intersected_container_target_is_order_independent() {
    let artifact = Artifact::media(
        Container::Mp4,
        VideoCodec::H264,
        Some(AudioCodec::Aac),
        1000,
    );
    let broad =
        "  - id: container-broad\n    field: media.container\n    op: in\n    value: [mp4, mov]\n";
    let mov = "  - id: container-mov\n    field: media.container\n    op: in\n    value: [mov]\n";
    assert_permutations_block(&artifact, broad, mov, BlockingCode::NonMp4Target);
}

#[test]
fn intersected_video_target_is_order_independent() {
    let artifact = Artifact::media(
        Container::Mp4,
        VideoCodec::H264,
        Some(AudioCodec::Aac),
        1000,
    );
    let broad =
        "  - id: video-broad\n    field: media.video.codec\n    op: in\n    value: [h264, vp9]\n";
    let vp9 = "  - id: video-vp9\n    field: media.video.codec\n    op: in\n    value: [vp9]\n";
    assert_permutations_block(&artifact, broad, vp9, BlockingCode::UnsupportedVideoTarget);
}

#[test]
fn intersected_audio_target_is_order_independent() {
    let artifact = Artifact::media(
        Container::Mp4,
        VideoCodec::H264,
        Some(AudioCodec::Aac),
        1000,
    );
    let broad =
        "  - id: audio-broad\n    field: media.audio.codec\n    op: in\n    value: [aac, mp3]\n";
    let mp3 = "  - id: audio-mp3\n    field: media.audio.codec\n    op: in\n    value: [mp3]\n";
    assert_permutations_block(&artifact, broad, mp3, BlockingCode::UnsupportedAudioTarget);
}

#[test]
fn intersected_supported_target_remains_valid_in_both_orders() {
    let artifact = Artifact::media(
        Container::Mov,
        VideoCodec::H264,
        Some(AudioCodec::Aac),
        1000,
    );
    let broad =
        "  - id: container-broad\n    field: media.container\n    op: in\n    value: [mp4, mov]\n";
    let mp4 = "  - id: container-mp4\n    field: media.container\n    op: in\n    value: [mp4]\n";
    for constraints in [
        overlapping_target(broad, mp4),
        overlapping_target(mp4, broad),
    ] {
        assert_eq!(
            plan(&artifact, &constraints, &default_catalog()).steps()[0].operation,
            TransformId::Remux
        );
    }
}

#[test]
fn refuses_unknown_or_non_yuv420p_pixel_format_for_transcode() {
    for pixel_format in [None, Some("yuv444p".into()), Some("yuv422p".into())] {
        let mut artifact = Artifact::media(
            Container::Mp4,
            VideoCodec::Hevc,
            Some(AudioCodec::Aac),
            1000,
        );
        artifact.first_video_mut().unwrap().pixel_format = pixel_format;
        assert_eq!(
            blocking(&artifact, &media_h264_mp4_aac()),
            vec![BlockingCode::PixelFormatConversionUnsupported]
        );
    }
}

#[test]
fn refuses_unknown_or_unapproved_color_facts_for_transcode() {
    let mut variants = Vec::new();
    for field in 0..4 {
        let mut artifact = Artifact::media(
            Container::Mp4,
            VideoCodec::Hevc,
            Some(AudioCodec::Aac),
            1000,
        );
        let video = artifact.first_video_mut().unwrap();
        match field {
            0 => video.color_range = None,
            1 => video.color_space = None,
            2 => video.color_transfer = None,
            _ => video.color_primaries = None,
        }
        variants.push(artifact);
    }
    let mut bt2020 = Artifact::media(
        Container::Mp4,
        VideoCodec::Hevc,
        Some(AudioCodec::Aac),
        1000,
    );
    bt2020.first_video_mut().unwrap().color_space = Some("bt2020nc".into());
    variants.push(bt2020);

    for artifact in variants {
        assert_eq!(
            blocking(&artifact, &media_h264_mp4_aac()),
            vec![BlockingCode::ColorConversionUnsupported]
        );
    }
}

#[test]
fn approved_transcode_claims_only_provable_pixel_and_color_preservation() {
    let artifact = Artifact::media(
        Container::Mp4,
        VideoCodec::Hevc,
        Some(AudioCodec::Aac),
        1000,
    );
    let outcome = plan(&artifact, &media_h264_mp4_aac(), &default_catalog());
    let step = &outcome.steps()[0];
    assert!(
        step.preservation
            .contains(&PreservationClaim::VideoPixelFormat)
    );
    assert!(
        step.preservation
            .contains(&PreservationClaim::VideoColorMetadata)
    );
    for field in [
        Field::MediaVideoPixelFormat,
        Field::MediaVideoBitDepth,
        Field::MediaVideoColorRange,
        Field::MediaVideoColorSpace,
        Field::MediaVideoColorTransfer,
        Field::MediaVideoColorPrimaries,
        Field::MediaVideoHdr,
    ] {
        assert!(
            step.expected.iter().any(|fact| fact.field == field),
            "missing expected {field:?}"
        );
    }
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
