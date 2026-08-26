use fitifact::constraints::{ConstraintValue, Field, Operator};
use fitifact::requirements::{REQUIREMENTS_SCHEMA, parse_image_requirements};

#[test]
fn parses_supported_image_language_with_normalized_constraints_and_spans() {
    let input = "JPEG or PNG, under 1.5 MiB, exactly 1200×630 pixels, minimum height 600; keep faces centered";
    let parsed = parse_image_requirements(input).unwrap();
    assert_eq!(
        serde_json::to_value(&parsed).unwrap()["schema"],
        REQUIREMENTS_SCHEMA
    );
    let constraints = parsed.constraints.unwrap();
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::ImageFormat
            && constraint.value == ConstraintValue::List(vec!["jpeg".into(), "png".into()])
    }));
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::FileBytes
            && constraint.value == ConstraintValue::Integer(1_572_864)
    }));
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::ImageWidth
            && constraint.op == Operator::Eq
            && constraint.value == ConstraintValue::Integer(1200)
    }));
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::ImageHeight
            && constraint.op == Operator::Gte
            && constraint.value == ConstraintValue::Integer(600)
    }));
    assert!(parsed.source_spans.iter().all(|span| {
        input[span.start..span.end] == span.text && !span.constraint_ids.is_empty()
    }));
    assert_eq!(parsed.unresolved.len(), 1);
    assert_eq!(parsed.unresolved[0].text, "keep faces centered");
}

#[test]
fn format_alternatives_do_not_swallow_intervening_dimension_evidence() {
    let input = "JPEG, exactly 1200×630, or PNG";
    let parsed = parse_image_requirements(input).unwrap();
    let constraints = parsed.constraints.unwrap();
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::ImageFormat
            && constraint.value == ConstraintValue::List(vec!["jpeg".into(), "png".into()])
    }));
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::ImageWidth
            && constraint.op == Operator::Eq
            && constraint.value == ConstraintValue::Integer(1200)
    }));
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::ImageHeight
            && constraint.op == Operator::Eq
            && constraint.value == ConstraintValue::Integer(630)
    }));
    assert!(parsed.ambiguities.is_empty());
    assert!(parsed.unresolved.is_empty());
    assert!(parsed.source_spans.iter().all(|span| {
        span.text == "JPEG" || span.text == "PNG" || span.text == "exactly 1200×630"
    }));
}

#[test]
fn words_containing_or_are_not_format_alternative_connectors() {
    let parsed = parse_image_requirements("JPEG for PNG").unwrap();
    assert!(parsed.constraints.is_none());
    assert_eq!(parsed.ambiguities.len(), 1);
    assert_eq!(parsed.unresolved[0].text, "for");
}

#[test]
fn jpg_is_canonicalized_and_decimal_bytes_are_exact() {
    let parsed = parse_image_requirements("JPG; maximum file size 2.25 MB").unwrap();
    let constraints = parsed.constraints.unwrap();
    assert_eq!(
        constraints
            .hard
            .iter()
            .find(|constraint| constraint.field == Field::ImageFormat)
            .unwrap()
            .value,
        ConstraintValue::List(vec!["jpeg".into()])
    );
    assert_eq!(
        constraints
            .hard
            .iter()
            .find(|constraint| constraint.field == Field::FileBytes)
            .unwrap()
            .value,
        ConstraintValue::Integer(2_250_000)
    );
}

#[test]
fn adjacent_short_size_and_dimension_rules_do_not_overlap() {
    let parsed = parse_image_requirements("JPEG, max 2 MB, max 2000 x 2000").unwrap();
    let constraints = parsed.constraints.unwrap();
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::FileBytes
            && constraint.value == ConstraintValue::Integer(2_000_000)
    }));
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::ImageWidth
            && constraint.op == Operator::Lte
            && constraint.value == ConstraintValue::Integer(2000)
    }));
    assert!(parsed.unresolved.is_empty());
}

#[test]
fn min_and_max_dimension_language_compiles_to_ranges() {
    let parsed = parse_image_requirements(
        "width at least 640 px, width no more than 1920 pixels, at most 1080 high",
    )
    .unwrap();
    let constraints = parsed.constraints.unwrap();
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::ImageWidth && constraint.op == Operator::Gte
    }));
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::ImageWidth && constraint.op == Operator::Lte
    }));
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::ImageHeight && constraint.op == Operator::Lte
    }));
}

#[test]
fn symbolic_minimum_and_maximum_dimensions_are_deterministic() {
    let parsed = parse_image_requirements("width >= 640, height <= 1080").unwrap();
    let constraints = parsed.constraints.unwrap();
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::ImageWidth && constraint.op == Operator::Gte
    }));
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::ImageHeight && constraint.op == Operator::Lte
    }));
    assert!(parsed.unresolved.is_empty());
}

#[test]
fn ambiguous_formats_and_unsupported_rules_are_never_silently_inferred() {
    let parsed = parse_image_requirements("JPEG and PNG with a transparent background").unwrap();
    assert_eq!(parsed.ambiguities.len(), 1);
    assert!(parsed.constraints.is_none());
    assert_eq!(parsed.unresolved[0].text, "with a transparent background");
}

#[test]
fn malformed_or_contradictory_numeric_targets_are_rejected() {
    assert!(parse_image_requirements("maximum 0×800").is_err());
    assert!(parse_image_requirements("max 0.0000001 MB").is_err());
    assert!(parse_image_requirements("minimum width 1200, maximum width 1000").is_err());
}

#[test]
fn recognizable_malformed_numeric_language_is_rejected_not_unresolved() {
    for input in [
        "maximum width 12.5",
        "maximum width - 12",
        "height at most nope",
        "exactly 12.5×630",
        "exactly - 1200×630",
        "exactly 1200×630.5",
        "max 1.2.3 MiB",
        "max 1,5 MiB",
        "max .5 MB",
        "max - 1 MB",
        "max 1.5 bytes",
    ] {
        let error = parse_image_requirements(input).unwrap_err();
        assert_eq!(error.code, fitifact::ErrorCode::InputInvalid, "{input}");
    }
}

#[test]
fn unsupported_prose_with_an_x_suffix_remains_unresolved() {
    let parsed = parse_image_requirements("make it 2x faster").unwrap();
    assert!(parsed.constraints.is_none());
    assert!(parsed.source_spans.is_empty());
    assert_eq!(parsed.unresolved[0].text, "make it 2x faster");
}

#[test]
fn dimension_qualified_incomplete_exact_pair_is_rejected() {
    let error = parse_image_requirements("exactly 2x").unwrap_err();
    assert_eq!(error.code, fitifact::ErrorCode::InputInvalid);
}

#[test]
fn no_supported_rule_returns_only_unresolved_text() {
    let parsed = parse_image_requirements("make it beautiful").unwrap();
    assert!(parsed.constraints.is_none());
    assert!(parsed.source_spans.is_empty());
    assert_eq!(parsed.unresolved[0].text, "make it beautiful");
}

#[test]
fn rejection_messages_compile_without_a_review_click() {
    let square =
        parse_image_requirements("Photo must be JPG. Maximum 500KB. 600×600 pixels.").unwrap();
    assert!(square.ambiguities.is_empty());
    let constraints = square.constraints.unwrap();
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::ImageFormat
            && constraint.value == ConstraintValue::List(vec!["jpeg".into()])
    }));
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::FileBytes
            && constraint.value == ConstraintValue::Integer(500_000)
    }));
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::ImageWidth
            && constraint.op == Operator::Eq
            && constraint.value == ConstraintValue::Integer(600)
    }));
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::ImageHeight
            && constraint.op == Operator::Eq
            && constraint.value == ConstraintValue::Integer(600)
    }));

    let alternatives =
        parse_image_requirements("Unsupported file. Image must be JPEG or PNG. Max 2 MB.").unwrap();
    assert!(alternatives.ambiguities.is_empty());
    let constraints = alternatives.constraints.unwrap();
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::ImageFormat
            && constraint.value == ConstraintValue::List(vec!["jpeg".into(), "png".into()])
    }));
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::FileBytes
            && constraint.value == ConstraintValue::Integer(2_000_000)
    }));
}

#[test]
fn webp_gif_tiff_and_bmp_tokens_compile_as_format_alternatives() {
    let parsed = parse_image_requirements("JPG, PNG, or WebP, max 2 MB").unwrap();
    let constraints = parsed.constraints.unwrap();
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::ImageFormat
            && constraint.value
                == ConstraintValue::List(vec!["jpeg".into(), "png".into(), "webp".into()])
    }));
    assert!(constraints.hard.iter().any(|constraint| {
        constraint.field == Field::FileBytes
            && constraint.value == ConstraintValue::Integer(2_000_000)
    }));

    let extra = parse_image_requirements("GIF, TIFF, or BMP").unwrap();
    let formats = extra
        .constraints
        .unwrap()
        .hard
        .into_iter()
        .find(|constraint| constraint.field == Field::ImageFormat)
        .unwrap()
        .value;
    assert_eq!(
        formats,
        ConstraintValue::List(vec!["bmp".into(), "gif".into(), "tiff".into()])
    );
}
