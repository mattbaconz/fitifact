use fitifact::constraints::{CONSTRAINTS_SCHEMA, compile_from_yaml, parse_size_bytes};
use fitifact::error::ErrorCode;

fn yaml(hard: &str) -> String {
    format!(
        "schema: {CONSTRAINTS_SCHEMA}\nhard:\n{hard}preferences:\n  preserve_audio: true\n  preserve_resolution: true\n"
    )
}

fn input_error(text: &str) {
    let error = compile_from_yaml(text).expect_err("constraints must be rejected");
    assert_eq!(error.code, ErrorCode::InputInvalid, "{error}");
}

#[test]
fn parses_the_versioned_hard_constraint_shape() {
    let constraints = compile_from_yaml(&yaml(
        "  - id: container\n    field: media.container\n    op: in\n    value: [mp4]\n",
    ))
    .unwrap();

    assert_eq!(
        serde_json::to_value(constraints).unwrap()["schema"],
        CONSTRAINTS_SCHEMA
    );
}

#[test]
fn rejects_constraint_input_over_one_mib() {
    let input = "x".repeat(1024 * 1024 + 1);
    input_error(&input);
}

#[test]
fn rejects_missing_or_wrong_schema() {
    input_error("hard: []\n");
    input_error("schema: fitifact.constraints/v2\nhard: []\n");
}

#[test]
fn rejects_empty_target_and_unknown_keys() {
    input_error("schema: fitifact.constraints/v1\nhard: []\n");
    input_error(
        "schema: fitifact.constraints/v1\nhard:\n  - id: family\n    field: file.family\n    op: eq\n    value: media\nunexpected: true\n",
    );
}

#[test]
fn rejects_blank_and_duplicate_constraint_ids() {
    input_error(&yaml(
        "  - id: '  '\n    field: file.family\n    op: eq\n    value: media\n",
    ));
    input_error(&yaml(
        "  - id: same\n    field: file.family\n    op: eq\n    value: media\n  - id: same\n    field: media.container\n    op: in\n    value: [mp4]\n",
    ));
    input_error(&yaml(
        "  - id: same\n    field: file.family\n    op: eq\n    value: media\n  - id: ' same '\n    field: media.container\n    op: in\n    value: [mp4]\n",
    ));
}

#[test]
fn rejects_conflicting_constraints_with_stable_error_code() {
    let error = compile_from_yaml(&yaml(
        "  - id: mp4\n    field: media.container\n    op: in\n    value: [mp4]\n  - id: mov\n    field: media.container\n    op: in\n    value: [mov]\n",
    ))
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::RequirementsConflict);
}

#[test]
fn rejects_invalid_field_operator_and_value_combinations() {
    input_error(&yaml(
        "  - id: bad-op\n    field: media.container\n    op: lte\n    value: 10\n",
    ));
    input_error(&yaml(
        "  - id: bad-value\n    field: file.bytes\n    op: lte\n    value: mp4\n",
    ));
    input_error(&yaml(
        "  - id: bad-list\n    field: media.video.width\n    op: lte\n    value: [1920]\n",
    ));
}

#[test]
fn rejects_empty_lists_and_zero_numeric_limits() {
    input_error(&yaml(
        "  - id: empty\n    field: media.container\n    op: in\n    value: []\n",
    ));
    input_error(&yaml(
        "  - id: zero\n    field: file.bytes\n    op: lte\n    value: 0\n",
    ));
}

#[test]
fn rejects_unknown_family_container_and_codec_values() {
    for constraint in [
        "  - id: family\n    field: file.family\n    op: eq\n    value: archive\n",
        "  - id: container\n    field: media.container\n    op: in\n    value: [avi]\n",
        "  - id: video\n    field: media.video.codec\n    op: in\n    value: [theora]\n",
        "  - id: audio\n    field: media.audio.codec\n    op: in\n    value: [flac]\n",
    ] {
        input_error(&yaml(constraint));
    }
}

#[test]
fn parses_raw_decimal_and_binary_sizes_without_float_ambiguity() {
    assert_eq!(parse_size_bytes(" 42 ").unwrap(), 42);
    assert_eq!(parse_size_bytes("1 MB").unwrap(), 1_000_000);
    assert_eq!(parse_size_bytes("1.5 mb").unwrap(), 1_500_000);
    assert_eq!(parse_size_bytes("1 MiB").unwrap(), 1_048_576);
    assert_eq!(parse_size_bytes("1.5mIb").unwrap(), 1_572_864);
}

#[test]
fn size_parser_rejects_unitless_fractions_bad_units_fractional_bytes_and_overflow() {
    for value in ["1.5", "1 KB", "0.0000001 MB", "18446744073709551616"] {
        assert!(parse_size_bytes(value).is_err(), "accepted {value}");
    }
}
