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

#[test]
fn version_is_automatic_and_matches_candidate() {
    let output = run(&["--version"]);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "fitifact 0.1.0-rc.1"
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
