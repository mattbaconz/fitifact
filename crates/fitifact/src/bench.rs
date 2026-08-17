use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::adapt::{AdaptRequest, AdaptationStatus, adapt};
use crate::capability::{TransformId, default_catalog};
use crate::check::check;
use crate::constraints::{image_jpeg, media_h264_mp4_aac};
use crate::contract::BenchSchema;
use crate::doctor::diagnose;
use crate::error::{Error, ErrorCode, Result};
use crate::ffmpeg::FfmpegProvider;
use crate::image::ImageProvider;
use crate::inspect::{DefaultInspector, FfprobeInspector, Inspector};
use crate::plan::{PlanOutcome, plan};
use crate::runtime::{
    ExecutionContext, RecordingSpawner, SystemSpawner, TransformProvider, program_is_ffprobe,
};

const NETWORK_CRATE_NAMES: &[&str] = &[
    "reqwest",
    "hyper",
    "hyper-util",
    "hyper-tls",
    "h2",
    "h3",
    "ureq",
    "attohttpc",
    "isahc",
    "surf",
    "awc",
    "tokio",
];

const CANONICAL: &[(&str, &str)] = &[
    ("compatible-h264-aac.mp4", "no-op"),
    ("remux-h264-aac.mov", "remux"),
    ("mismatch-hevc-aac.mp4", "transcode"),
];

const CANONICAL_IMAGE: &[(&str, &str)] = &[
    ("compatible-jpeg.jpg", "no-op"),
    ("mismatch-png.png", "encode-jpeg"),
];

#[derive(Debug, Clone)]
pub struct BenchOptions {
    pub fixtures: PathBuf,
    pub keep: bool,
    pub work_dir: PathBuf,
    pub cli_exe: Option<PathBuf>,
    pub lockfile: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchReport {
    pub schema: BenchSchema,
    pub version: String,
    pub doctor_healthy: bool,
    pub fixtures: PathBuf,
    pub cold_start_ms: Option<f64>,
    pub scenarios: Vec<BenchScenario>,
    pub proofs: BenchProofs,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchScenario {
    pub file: String,
    pub expected: String,
    pub actual: String,
    pub matched: bool,
    pub summary: String,
    pub inspect_ms: f64,
    pub check_ms: f64,
    pub plan_ms: f64,
    pub adapt_ms: f64,
    pub ffprobe_spawns: usize,
    pub ffmpeg_spawns: usize,
    pub transform_provider_loaded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchProofs {
    pub noop_ffmpeg_spawns_zero: bool,
    pub check_plan_ffprobe_only: bool,
    pub no_network_crates: bool,
    pub all_outcomes_matched: bool,
    pub image_adapt_ffmpeg_spawns_zero: bool,
}

pub fn network_crates_in_lockfile(lockfile: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in lockfile.lines() {
        let Some(name) = line.strip_prefix("name = \"") else {
            continue;
        };
        let Some(name) = name.strip_suffix('"') else {
            continue;
        };
        if NETWORK_CRATE_NAMES.contains(&name) && !found.iter().any(|item| item == name) {
            found.push(name.to_string());
        }
    }
    found
}

pub fn resolve_fixtures(explicit: Option<PathBuf>) -> PathBuf {
    if let Some(path) = explicit {
        return path;
    }
    if let Ok(path) = std::env::var("FITIFACT_FIXTURES") {
        return PathBuf::from(path);
    }
    PathBuf::from("fixtures/media")
}

pub fn find_lockfile(start: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        let candidate = dir.join("Cargo.lock");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

pub fn run_bench(options: BenchOptions) -> Result<BenchReport> {
    if !options.fixtures.is_dir() {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            format!(
                "fixture directory not found: {}",
                options.fixtures.display()
            ),
        ));
    }
    std::fs::create_dir_all(&options.work_dir).map_err(|err| {
        Error::new(
            ErrorCode::ExecutionFailed,
            format!("cannot create bench workspace: {err}"),
        )
    })?;
    let doctor = diagnose(
        &SystemSpawner,
        &options.work_dir,
        &options.work_dir,
        Duration::from_secs(30),
    );
    let lockfile_text = options
        .lockfile
        .as_ref()
        .and_then(|path| std::fs::read_to_string(path).ok());
    let network_crates = lockfile_text
        .as_deref()
        .map(network_crates_in_lockfile)
        .unwrap_or_default();
    let no_network_crates = lockfile_text.is_some() && network_crates.is_empty();

    let cold_start_ms = options
        .cli_exe
        .as_ref()
        .and_then(|exe| measure_cold_start(exe, &options.fixtures.join(CANONICAL[0].0)));

    let mut scenarios = Vec::new();
    for &(file, expected) in CANONICAL {
        let needs_provider = expected != "no-op";
        if needs_provider && !doctor.healthy {
            scenarios.push(BenchScenario {
                file: file.into(),
                expected: expected.into(),
                actual: "tools-missing".into(),
                matched: false,
                summary: "System FFmpeg is not healthy enough to run this scenario.".into(),
                inspect_ms: 0.0,
                check_ms: 0.0,
                plan_ms: 0.0,
                adapt_ms: 0.0,
                ffprobe_spawns: 0,
                ffmpeg_spawns: 0,
                transform_provider_loaded: false,
            });
            continue;
        }
        scenarios.push(run_scenario(
            &options.fixtures.join(file),
            expected,
            &options.work_dir,
        )?);
    }

    let image_dir = image_fixture_dir(&options.fixtures);
    let mut image_ffmpeg_spawns = Vec::new();
    if image_dir.is_dir() {
        for &(file, expected) in CANONICAL_IMAGE {
            let scenario = run_image_scenario(&image_dir.join(file), expected, &options.work_dir)?;
            image_ffmpeg_spawns.push(scenario.ffmpeg_spawns);
            scenarios.push(scenario);
        }
    }

    let check_plan_ffprobe_only =
        prove_check_plan_ffprobe_only(&options.fixtures.join(CANONICAL[0].0))?;
    let media_noop = scenarios
        .iter()
        .find(|row| row.file == CANONICAL[0].0 && row.expected == "no-op");
    let proofs = BenchProofs {
        noop_ffmpeg_spawns_zero: media_noop
            .is_some_and(|row| row.ffmpeg_spawns == 0 && !row.transform_provider_loaded),
        check_plan_ffprobe_only,
        no_network_crates,
        all_outcomes_matched: scenarios.iter().all(|row| row.matched),
        image_adapt_ffmpeg_spawns_zero: image_dir.is_dir()
            && !image_ffmpeg_spawns.is_empty()
            && image_ffmpeg_spawns.iter().all(|count| *count == 0),
    };

    if !options.keep {
        let _ = std::fs::remove_dir_all(&options.work_dir);
    }

    Ok(BenchReport {
        schema: BenchSchema,
        version: env!("CARGO_PKG_VERSION").into(),
        doctor_healthy: doctor.healthy,
        fixtures: options.fixtures,
        cold_start_ms,
        scenarios,
        proofs,
    })
}

fn measure_cold_start(exe: &Path, fixture: &Path) -> Option<f64> {
    let start = Instant::now();
    let output = Command::new(exe)
        .args(["inspect", "--json"])
        .arg(fixture)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(millis(start.elapsed()))
}

fn prove_check_plan_ffprobe_only(path: &Path) -> Result<bool> {
    let spawner = RecordingSpawner::new(SystemSpawner);
    let inspector = FfprobeInspector::new(&spawner);
    let artifact = inspector.inspect(path)?;
    let target = media_h264_mp4_aac();
    let _ = check(&artifact, &target);
    let _ = plan(&artifact, &target, &default_catalog());
    Ok(spawner.ffmpeg_spawn_count() == 0
        && !spawner.programs().is_empty()
        && spawner
            .programs()
            .iter()
            .all(|program| program_is_ffprobe(program)))
}

fn run_scenario(path: &Path, expected: &str, work_dir: &Path) -> Result<BenchScenario> {
    let file = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file")
        .to_string();
    let spawner = RecordingSpawner::new(SystemSpawner);
    let inspector = FfprobeInspector::new(&spawner);
    let target = media_h264_mp4_aac();

    let start = Instant::now();
    let artifact = inspector.inspect(path)?;
    let inspect_ms = millis(start.elapsed());

    let start = Instant::now();
    let _report = check(&artifact, &target);
    let check_ms = millis(start.elapsed());

    let start = Instant::now();
    let outcome = plan(&artifact, &target, &default_catalog());
    let plan_ms = millis(start.elapsed());

    let transform_provider_loaded = matches!(outcome, PlanOutcome::Planned { .. });
    let provider = transform_provider_loaded.then(|| FfmpegProvider::new(&spawner));
    let output = (expected != "no-op").then(|| {
        work_dir.join(format!(
            "{}.adapted.mp4",
            path.file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("out")
        ))
    });

    let start = Instant::now();
    let result = adapt(AdaptRequest {
        input: path,
        constraints: target,
        output,
        catalog: None,
        inspector: &inspector,
        provider: provider.as_ref().map(|item| item as &dyn TransformProvider),
        execution: ExecutionContext {
            timeout: Duration::from_secs(60),
            temp_dir: Some(work_dir.to_path_buf()),
        },
    })?;
    let adapt_ms = millis(start.elapsed());

    let actual = match (
        &result.status,
        result.plan.as_ref().and_then(|plan| plan.steps.first()),
    ) {
        (AdaptationStatus::Compatible, _) => "no-op",
        (AdaptationStatus::Adapted, Some(step)) if step.operation == TransformId::Remux => "remux",
        (AdaptationStatus::Adapted, Some(step))
            if step.operation == TransformId::TranscodeVideo =>
        {
            "transcode"
        }
        (AdaptationStatus::Adapted, Some(step)) if step.operation == TransformId::EncodeJpeg => {
            "encode-jpeg"
        }
        (AdaptationStatus::Adapted, _) => "adapted",
        (AdaptationStatus::CannotSatisfy, _) => "refuse",
        (AdaptationStatus::Failed, _) => "failed",
    };

    Ok(BenchScenario {
        file,
        expected: expected.into(),
        actual: actual.into(),
        matched: actual == expected,
        summary: result.explanation.summary,
        inspect_ms,
        check_ms,
        plan_ms,
        adapt_ms,
        ffprobe_spawns: spawner.ffprobe_spawn_count(),
        ffmpeg_spawns: spawner.ffmpeg_spawn_count(),
        transform_provider_loaded,
    })
}

fn image_fixture_dir(media_dir: &Path) -> PathBuf {
    media_dir
        .parent()
        .map(|parent| parent.join("image"))
        .unwrap_or_else(|| PathBuf::from("fixtures/image"))
}

fn run_image_scenario(path: &Path, expected: &str, work_dir: &Path) -> Result<BenchScenario> {
    let file = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file")
        .to_string();
    let spawner = RecordingSpawner::new(SystemSpawner);
    let inspector = DefaultInspector::new(&spawner);
    let target = image_jpeg();

    let start = Instant::now();
    let artifact = inspector.inspect(path)?;
    let inspect_ms = millis(start.elapsed());

    let start = Instant::now();
    let _report = check(&artifact, &target);
    let check_ms = millis(start.elapsed());

    let start = Instant::now();
    let outcome = plan(&artifact, &target, &default_catalog());
    let plan_ms = millis(start.elapsed());

    let transform_provider_loaded = matches!(outcome, PlanOutcome::Planned { .. });
    let provider = transform_provider_loaded.then(ImageProvider::default);
    let output = (expected != "no-op").then(|| {
        work_dir.join(format!(
            "{}.adapted.jpg",
            path.file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("out")
        ))
    });

    let start = Instant::now();
    let result = adapt(AdaptRequest {
        input: path,
        constraints: target,
        output,
        catalog: None,
        inspector: &inspector,
        provider: provider.as_ref().map(|item| item as &dyn TransformProvider),
        execution: ExecutionContext {
            timeout: Duration::from_secs(60),
            temp_dir: Some(work_dir.to_path_buf()),
        },
    })?;
    let adapt_ms = millis(start.elapsed());

    let actual = match (
        &result.status,
        result.plan.as_ref().and_then(|plan| plan.steps.first()),
    ) {
        (AdaptationStatus::Compatible, _) => "no-op",
        (AdaptationStatus::Adapted, Some(step)) if step.operation == TransformId::EncodeJpeg => {
            "encode-jpeg"
        }
        (AdaptationStatus::Adapted, _) => "adapted",
        (AdaptationStatus::CannotSatisfy, _) => "refuse",
        (AdaptationStatus::Failed, _) => "failed",
    };

    Ok(BenchScenario {
        file,
        expected: expected.into(),
        actual: actual.into(),
        matched: actual == expected,
        summary: result.explanation.summary,
        inspect_ms,
        check_ms,
        plan_ms,
        adapt_ms,
        ffprobe_spawns: spawner.ffprobe_spawn_count(),
        ffmpeg_spawns: spawner.ffmpeg_spawn_count(),
        transform_provider_loaded,
    })
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockfile_scan_flags_http_clients_and_ignores_fitifact() {
        let lock = r#"
[[package]]
name = "fitifact"
version = "0.1.0-rc.1"

[[package]]
name = "reqwest"
version = "0.12.0"

[[package]]
name = "serde"
version = "1.0.0"
"#;
        assert_eq!(
            network_crates_in_lockfile(lock),
            vec!["reqwest".to_string()]
        );
        assert!(network_crates_in_lockfile("name = \"fitifact\"\n").is_empty());
    }
}
