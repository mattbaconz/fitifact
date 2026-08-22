use std::path::PathBuf;
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fitifact"))
        .args(args)
        .output()
        .unwrap()
}

fn json_stderr(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stderr).unwrap_or_else(|error| {
        panic!(
            "stderr was not JSON ({error}): {}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn constraint_file(label: &str, length: Option<usize>) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "fitifact-{label}-{}-{}.yaml",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut text = "schema: fitifact.constraints/v1\nhard:\n  - id: container\n    field: media.container\n    op: in\n    value: [mp4]\n".to_string();
    if let Some(length) = length {
        assert!(text.len() + 2 <= length);
        text.push('#');
        text.push_str(&"x".repeat(length - text.len()));
    }
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn version_is_automatic_and_matches_candidate() {
    let output = run(&["--version"]);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "fitifact 0.1.0-rc.5"
    );
}

#[test]
fn doctor_json_is_the_v1_envelope() {
    let output = run(&["doctor", "--json"]);
    assert!(matches!(output.status.code(), Some(0 | 5)));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], "fitifact.doctor/v1");
    assert!(value["capabilities"].is_array());
    assert!(value["workspaces"].is_array());
}

#[test]
fn engine_errors_are_structured_when_json_is_requested() {
    let output = run(&["inspect", "definitely-missing.mp4", "--json"]);
    assert_eq!(output.status.code(), Some(4));
    let value = json_stderr(&output);
    assert_eq!(value["schema"], "fitifact.error/v1");
    assert_eq!(value["code"], "INPUT_INVALID");
}

#[test]
fn max_size_uses_strict_mb_mib_parser() {
    let output = run(&[
        "check",
        "definitely-missing.mp4",
        "--max-size",
        "1.5 MiB",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(json_stderr(&output)["code"], "INPUT_INVALID");
}

#[test]
fn transform_timeout_is_bounded_and_usage_errors_are_json() {
    let output = run(&[
        "adapt",
        "definitely-missing.mp4",
        "--container",
        "mp4",
        "--timeout-seconds",
        "0",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(64));
    assert_eq!(json_stderr(&output)["schema"], "fitifact.error/v1");
}

#[test]
fn constraints_file_conflicts_with_every_individual_hard_target() {
    let path = constraint_file("constraint-conflicts", None);
    let path = path.to_str().unwrap();
    for flag in [
        ["--container", "mp4"],
        ["--video-codec", "h264"],
        ["--audio-codec", "aac"],
        ["--image-format", "jpeg"],
        ["--max-size", "1"],
        ["--max-width", "1"],
        ["--max-height", "1"],
    ] {
        let output = run(&[
            "check",
            "definitely-missing.mp4",
            "--constraints",
            path,
            flag[0],
            flag[1],
            "--json",
        ]);
        assert_eq!(output.status.code(), Some(64), "flag {flag:?}");
        let error = json_stderr(&output);
        assert_eq!(error["schema"], "fitifact.error/v1");
        assert_eq!(error["code"], "INPUT_INVALID");
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn constraint_file_read_accepts_exact_limit_and_rejects_one_byte_over() {
    const LIMIT: usize = fitifact::constraints::MAX_CONSTRAINT_BYTES;
    let exact = constraint_file("constraint-exact-limit", Some(LIMIT));
    let output = run(&[
        "check",
        "definitely-missing.mp4",
        "--constraints",
        exact.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(4));
    assert!(
        json_stderr(&output)["message"]
            .as_str()
            .unwrap()
            .contains("file not found"),
        "an exact-limit valid document must reach input inspection"
    );

    let over = constraint_file("constraint-over-limit", Some(LIMIT + 1));
    let output = run(&[
        "check",
        "definitely-missing.mp4",
        "--constraints",
        over.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(4));
    assert!(
        json_stderr(&output)["message"]
            .as_str()
            .unwrap()
            .contains("1 MiB safety limit")
    );

    std::fs::remove_file(exact).unwrap();
    std::fs::remove_file(over).unwrap();
}

#[test]
#[ignore]
fn bench_json_reports_canonical_proofs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(env!("CARGO_BIN_EXE_fitifact"))
        .current_dir(&root)
        .args(["bench", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "bench failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], "fitifact.bench/v1");
    assert_eq!(value["proofs"]["noop_ffmpeg_spawns_zero"], true);
    assert_eq!(value["proofs"]["check_plan_ffprobe_only"], true);
    assert_eq!(value["proofs"]["all_outcomes_matched"], true);
    assert_eq!(value["proofs"]["image_adapt_ffmpeg_spawns_zero"], true);
}
