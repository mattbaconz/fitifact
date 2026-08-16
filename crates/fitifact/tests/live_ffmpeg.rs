//! Live FFmpeg/ffprobe tests. Ignored unless tools are on PATH:
//! `cargo test -p fitifact --test live_ffmpeg -- --ignored --nocapture`

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use fitifact::artifact::{AudioCodec, Container, HdrStatus, VideoCodec};
use fitifact::capability::{TransformId, default_catalog};
use fitifact::check::check;
use fitifact::constraints::{ConstraintInput, compile, media_h264_mp4_aac};
use fitifact::error::ErrorCode;
use fitifact::ffmpeg::FfmpegProvider;
use fitifact::inspect::{FfprobeInspector, Inspector};
use fitifact::plan::{BlockingCode, plan as create_plan};
use fitifact::runtime::{ExecutionContext, RecordingSpawner, SystemSpawner};
use fitifact::{AdaptRequest, AdaptationStatus, adapt};

fn tool_ok(name: &str) -> bool {
    Command::new(name)
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn require_tools() {
    assert!(
        tool_ok("ffmpeg") && tool_ok("ffprobe"),
        "ffmpeg/ffprobe not on PATH; install them to run live tests"
    );
}

fn canonical_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/media")
        .join(name)
}

#[test]
#[ignore]
fn canonical_happy_path_fixtures_select_minimal_outcomes() {
    require_tools();
    let inspector = FfprobeInspector::default();
    let target = media_h264_mp4_aac();
    let catalog = default_catalog();

    let compatible = inspector
        .inspect(&canonical_fixture("compatible-h264-aac.mp4"))
        .unwrap();
    assert!(create_plan(&compatible, &target, &catalog).is_compatible());

    let mismatch = inspector
        .inspect(&canonical_fixture("mismatch-hevc-aac.mp4"))
        .unwrap();
    assert_eq!(
        create_plan(&mismatch, &target, &catalog).steps()[0].operation,
        TransformId::TranscodeVideo
    );

    let remux = inspector
        .inspect(&canonical_fixture("remux-h264-aac.mov"))
        .unwrap();
    assert_eq!(
        create_plan(&remux, &target, &catalog).steps()[0].operation,
        TransformId::Remux
    );
}

fn encoder_ok(name: &str) -> bool {
    let output = Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .output()
        .expect("ffmpeg encoders");
    String::from_utf8_lossy(&output.stdout).contains(name)
}

fn generate(dir: &Path, name: &str, extra: &[&str]) -> PathBuf {
    let dest = dir.join(name);
    let mut args = vec![
        "-nostdin",
        "-y",
        "-f",
        "lavfi",
        "-i",
        "testsrc=duration=0.4:size=160x120:rate=10",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:duration=0.4",
    ];
    args.extend_from_slice(extra);
    args.push(dest.to_str().unwrap());
    let status = Command::new("ffmpeg").args(&args).status().unwrap();
    assert!(status.success(), "fixture generation failed for {name}");
    dest
}

#[test]
#[ignore]
fn live_hevc_transcodes_video_only() {
    require_tools();
    assert!(
        encoder_ok("libx265") && encoder_ok("libx264"),
        "need libx265 and libx264"
    );
    let dir = std::env::temp_dir().join(format!("fitifact-live-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = generate(
        &dir,
        "hevc-aac.mp4",
        &[
            "-c:v",
            "libx265",
            "-pix_fmt",
            "yuv420p",
            "-tag:v",
            "hvc1",
            "-c:a",
            "aac",
            "-b:a",
            "64k",
            "-color_range",
            "tv",
            "-colorspace",
            "bt709",
            "-color_trc",
            "bt709",
            "-color_primaries",
            "bt709",
            "-x265-params",
            "colorprim=bt709:transfer=bt709:colormatrix=bt709:range=limited",
        ],
    );
    let original_bytes = std::fs::metadata(&input).unwrap().len();
    let output = dir.join("hevc-aac.adapted.mp4");
    let spawner = RecordingSpawner::new(SystemSpawner);
    let inspector = FfprobeInspector::new(&spawner);
    let provider = FfmpegProvider::new(&spawner);
    let result = adapt(AdaptRequest {
        input: &input,
        constraints: media_h264_mp4_aac(),
        output: Some(output.clone()),
        catalog: None,
        inspector: &inspector,
        provider: Some(&provider),
        execution: ExecutionContext {
            timeout: Duration::from_secs(60),
            temp_dir: Some(dir.clone()),
        },
    })
    .unwrap();
    assert_eq!(result.status, AdaptationStatus::Adapted);
    let plan = result.plan.expect("plan");
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].operation, TransformId::TranscodeVideo);
    let validation = result.validation.expect("validation");
    assert!(validation.provenance.iter().any(|claim| {
        claim.claim == "audio_copied" && claim.status == fitifact::ValidationStatus::Pass
    }));
    assert!(validation.provenance.iter().any(|claim| {
        claim.claim == "video_changed" && claim.status == fitifact::ValidationStatus::Pass
    }));
    let out = validation.artifact;
    assert_eq!(out.first_video().unwrap().codec, Some(VideoCodec::H264));
    assert_eq!(out.first_audio().unwrap().codec, Some(AudioCodec::Aac));
    assert_eq!(std::fs::metadata(&input).unwrap().len(), original_bytes);
    assert!(output.exists());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
#[ignore]
fn live_h264_mp4_is_noop_without_ffmpeg() {
    require_tools();
    assert!(encoder_ok("libx264"), "need libx264");
    let dir = std::env::temp_dir().join(format!("fitifact-live-noop-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = generate(
        &dir,
        "h264-aac.mp4",
        &[
            "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "64k",
        ],
    );
    let spawner = RecordingSpawner::new(SystemSpawner);
    let inspector = FfprobeInspector::new(&spawner);
    let provider = FfmpegProvider::new(&spawner);
    let result = adapt(AdaptRequest {
        input: &input,
        constraints: media_h264_mp4_aac(),
        output: None,
        catalog: None,
        inspector: &inspector,
        provider: Some(&provider),
        execution: ExecutionContext::default(),
    })
    .unwrap();
    assert_eq!(result.status, AdaptationStatus::Compatible);
    assert_eq!(spawner.ffmpeg_spawn_count(), 0);
    assert!(spawner.ffprobe_spawn_count() >= 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
#[ignore]
fn live_check_and_plan_spawn_only_ffprobe() {
    require_tools();
    let spawner = RecordingSpawner::new(SystemSpawner);
    let inspector = FfprobeInspector::new(&spawner);
    let artifact = inspector
        .inspect(&canonical_fixture("compatible-h264-aac.mp4"))
        .unwrap();
    let target = media_h264_mp4_aac();
    assert!(check(&artifact, &target).compatible);
    assert!(create_plan(&artifact, &target, &default_catalog()).is_compatible());
    assert_eq!(spawner.ffmpeg_spawn_count(), 0);
    assert!(spawner.ffprobe_spawn_count() >= 1);
    assert!(
        spawner
            .programs()
            .iter()
            .all(|program| fitifact::runtime::program_is_ffprobe(program))
    );
}

#[test]
#[ignore]
fn live_mov_remuxes_without_transcode() {
    require_tools();
    assert!(encoder_ok("libx264"), "need libx264");
    let dir = std::env::temp_dir().join(format!("fitifact-live-mov-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = generate(
        &dir,
        "h264-aac.mov",
        &[
            "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "64k",
        ],
    );
    let output = dir.join("h264-aac.adapted.mp4");
    let spawner = RecordingSpawner::new(SystemSpawner);
    let inspector = FfprobeInspector::new(&spawner);
    let provider = FfmpegProvider::new(&spawner);
    let result = adapt(AdaptRequest {
        input: &input,
        constraints: media_h264_mp4_aac(),
        output: Some(output.clone()),
        catalog: None,
        inspector: &inspector,
        provider: Some(&provider),
        execution: ExecutionContext {
            timeout: Duration::from_secs(60),
            temp_dir: Some(dir.clone()),
        },
    })
    .unwrap();
    assert_eq!(result.status, AdaptationStatus::Adapted);
    assert_eq!(result.plan.unwrap().steps[0].operation, TransformId::Remux);
    let validation = result.validation.unwrap();
    assert!(validation.provenance.iter().any(|claim| {
        claim.claim == "video_copied" && claim.status == fitifact::ValidationStatus::Pass
    }));
    assert!(validation.provenance.iter().any(|claim| {
        claim.claim == "audio_copied" && claim.status == fitifact::ValidationStatus::Pass
    }));
    let out = validation.artifact;
    assert_eq!(out.container.as_ref().unwrap().as_str(), "mp4");
    assert_eq!(out.first_video().unwrap().codec, Some(VideoCodec::H264));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
#[ignore]
fn canonical_corrupt_fixture_is_rejected() {
    require_tools();
    let inspector = FfprobeInspector::default();
    let error = inspector
        .inspect(&canonical_fixture("corrupt-truncated.mp4"))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InputInvalid);
}

#[test]
#[ignore]
fn canonical_extra_stream_fixture_is_refused() {
    require_tools();
    let inspector = FfprobeInspector::default();
    let artifact = inspector
        .inspect(&canonical_fixture("unsupported-extra-video.mp4"))
        .unwrap();
    assert_eq!(artifact.video_streams().count(), 2);
    let outcome = create_plan(&artifact, &media_h264_mp4_aac(), &default_catalog());
    assert_eq!(
        outcome.blocking_codes(),
        vec![BlockingCode::UnsafeStreamTopology]
    );
}

#[test]
#[ignore]
fn canonical_hdr10_fixture_is_refused_without_conversion() {
    require_tools();
    let inspector = FfprobeInspector::default();
    let artifact = inspector
        .inspect(&canonical_fixture("refusal-hdr10-hevc-aac.mp4"))
        .unwrap();
    let video = artifact.first_video().unwrap();
    assert_eq!(video.bit_depth, Some(10));
    assert_eq!(video.hdr, HdrStatus::Hdr);
    let outcome = create_plan(&artifact, &media_h264_mp4_aac(), &default_catalog());
    assert_eq!(
        outcome.blocking_codes(),
        vec![BlockingCode::HdrConversionUnsupported]
    );
}

#[test]
#[ignore]
fn live_matroska_is_not_webm_and_cannot_remux() {
    require_tools();
    assert!(encoder_ok("libx264"), "need libx264");
    let dir = std::env::temp_dir().join(format!("fitifact-live-mkv-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = generate(
        &dir,
        "h264-aac.mkv",
        &[
            "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "64k",
        ],
    );
    let inspector = FfprobeInspector::default();
    let artifact = inspector.inspect(&input).unwrap();
    assert_ne!(artifact.container, Some(Container::Webm));
    let webm_target = compile(ConstraintInput {
        container: Some(vec!["webm".into()]),
        ..ConstraintInput::default()
    })
    .unwrap();
    assert!(!check(&artifact, &webm_target).compatible);
    let outcome = create_plan(&artifact, &media_h264_mp4_aac(), &default_catalog());
    assert_eq!(
        outcome.blocking_codes(),
        vec![BlockingCode::UnsupportedContainer]
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
#[ignore]
fn live_webm_is_fail_closed_and_cannot_remux() {
    require_tools();
    assert!(
        encoder_ok("libvpx-vp9") && encoder_ok("libopus"),
        "need libvpx-vp9 and libopus to generate a live WebM"
    );
    let dir = std::env::temp_dir().join(format!("fitifact-live-webm-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = generate(
        &dir,
        "vp9-opus.webm",
        &[
            "-c:v",
            "libvpx-vp9",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "libopus",
            "-b:a",
            "64k",
            "-f",
            "webm",
        ],
    );
    let inspector = FfprobeInspector::default();
    let artifact = inspector.inspect(&input).unwrap();
    assert!(
        !matches!(
            artifact.container,
            Some(Container::Mp4) | Some(Container::Mov)
        ),
        "live WebM must not be classified as MP4/MOV, got {:?}",
        artifact.container
    );
    assert!(!check(&artifact, &media_h264_mp4_aac()).compatible);
    let outcome = create_plan(&artifact, &media_h264_mp4_aac(), &default_catalog());
    assert!(
        !outcome.is_compatible() && outcome.plan().is_none(),
        "live WebM must be refused rather than remuxed or transcoded, got {:?}",
        outcome.blocking_codes()
    );
    assert!(
        !outcome.blocking_codes().is_empty(),
        "refusal must carry a blocking code"
    );
    if let Some(Container::Unknown(label)) = &artifact.container {
        assert!(
            label.contains("matroska") || label.contains("webm"),
            "unexpected unknown label {label}"
        );
        let webm_target = compile(ConstraintInput {
            container: Some(vec!["webm".into()]),
            ..ConstraintInput::default()
        })
        .unwrap();
        assert!(
            !check(&artifact, &webm_target).compatible,
            "ambiguous probe {label} must not check as WebM"
        );
        assert_eq!(
            artifact.container.as_ref().map(Container::display_label),
            Some(format!("unknown ({label})"))
        );
    }
    let _ = std::fs::remove_dir_all(dir);
}
