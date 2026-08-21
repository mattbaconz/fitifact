use serde::{Deserialize, Serialize};

use crate::artifact::ImageFormat;
use crate::constraints::{
    Constraint, ConstraintSet, ConstraintValue, Field, Operator, Preferences, parse_size_bytes,
    validate_and_normalize,
};
pub use crate::contract::{REQUIREMENTS_SCHEMA, RequirementsSchema};
use crate::error::{Error, ErrorCode, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementSourceSpan {
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub constraint_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementAmbiguity {
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnresolvedRequirement {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementParse {
    pub schema: RequirementsSchema,
    pub constraints: Option<ConstraintSet>,
    pub source_spans: Vec<RequirementSourceSpan>,
    pub ambiguities: Vec<RequirementAmbiguity>,
    pub unresolved: Vec<UnresolvedRequirement>,
}

#[derive(Debug, Clone)]
struct Token {
    start: usize,
    end: usize,
    value: String,
}

/// Parse the deliberately small, deterministic consumer-image requirement language.
/// Anything outside the supported grammar is returned as unresolved text.
pub fn parse_image_requirements(text: &str) -> Result<RequirementParse> {
    let tokens = tokenize(text);
    reject_malformed_numeric_targets(text, &tokens)?;
    let mut covered = vec![false; text.len()];
    let mut constraints = Vec::new();
    let mut spans = Vec::new();
    let mut ambiguities = Vec::new();

    parse_formats(
        text,
        &tokens,
        &mut covered,
        &mut constraints,
        &mut spans,
        &mut ambiguities,
    );
    parse_dimensions(text, &tokens, &mut covered, &mut constraints, &mut spans);
    parse_sizes(text, &tokens, &mut covered, &mut constraints, &mut spans)?;

    let constraints = if constraints.is_empty() {
        None
    } else {
        Some(validate_and_normalize(ConstraintSet {
            schema: crate::contract::ConstraintsSchema,
            hard: constraints,
            preferences: Preferences::default(),
        })?)
    };
    spans.sort_by_key(|span| (span.start, span.end));
    ambiguities.sort_by_key(|span| (span.start, span.end));

    Ok(RequirementParse {
        schema: RequirementsSchema,
        constraints,
        source_spans: spans,
        ambiguities,
        unresolved: unresolved(text, &covered),
    })
}

fn reject_malformed_numeric_targets(text: &str, tokens: &[Token]) -> Result<()> {
    for index in 0..tokens.len() {
        if matches!(tokens[index].value.as_str(), "x" | "×") {
            let left = index.checked_sub(1).and_then(|value| tokens.get(value));
            let right = tokens.get(index + 1);
            if left.is_some_and(looks_numeric) || right.is_some_and(looks_numeric) {
                let valid = left.is_some_and(|token| {
                    integer(token).is_some() && !has_attached_sign_or_decimal(text, token)
                }) && right.is_some_and(|token| {
                    integer(token).is_some() && !has_attached_sign_or_decimal(text, token)
                });
                if !valid {
                    return Err(invalid_numeric(
                        "exact image dimensions must contain two positive whole integers",
                    ));
                }
            }
        }

        if axis_field(&tokens[index].value).is_some() {
            if qualifier_before(tokens, index).is_some() {
                let value_index = if tokens
                    .get(index + 1)
                    .is_some_and(|token| token.value == "of")
                {
                    index + 2
                } else {
                    index + 1
                };
                validate_dimension_number(text, tokens.get(value_index))?;
            }
            if let Some((_, qualifier_end)) = qualifier_after(tokens, index + 1) {
                validate_dimension_number(text, tokens.get(qualifier_end + 1))?;
            }
            let value_index = if index > 1 && pixel_word(&tokens[index - 1].value) {
                Some(index - 2)
            } else {
                index.checked_sub(1)
            };
            if let Some(value_index) = value_index
                && qualifier_before(tokens, value_index).is_some()
            {
                validate_dimension_number(text, tokens.get(value_index))?;
            }
        }
    }

    for unit_index in 0..tokens.len() {
        if !matches!(
            tokens[unit_index].value.as_str(),
            "mb" | "mib" | "byte" | "bytes"
        ) || unit_index == 0
        {
            continue;
        }
        let number_index = unit_index - 1;
        let prefix = size_qualifier_before(tokens, number_index);
        let suffix = tokens
            .get(unit_index + 1)
            .is_some_and(|token| matches!(token.value.as_str(), "max" | "maximum" | "limit"));
        if prefix.is_none() && !suffix {
            continue;
        }
        let start = prefix.unwrap_or(number_index);
        let numeric_tokens: Vec<_> = tokens[start..unit_index]
            .iter()
            .filter(|token| looks_numeric(token))
            .collect();
        let number = &tokens[number_index];
        if numeric_tokens.len() != 1
            || decimal(number).is_none()
            || has_attached_sign_or_decimal(text, number)
        {
            return Err(invalid_numeric(
                "byte limits require one valid decimal MB/MiB or whole-byte value",
            ));
        }
        parse_size_bytes(&format!("{} {}", number.value, tokens[unit_index].value))?;
    }
    Ok(())
}

fn validate_dimension_number(text: &str, token: Option<&Token>) -> Result<()> {
    if token
        .is_none_or(|token| integer(token).is_none() || has_attached_sign_or_decimal(text, token))
    {
        return Err(invalid_numeric(
            "image dimensions require positive whole integers",
        ));
    }
    Ok(())
}

fn looks_numeric(token: &Token) -> bool {
    token.value.bytes().any(|byte| byte.is_ascii_digit())
}

fn has_attached_sign_or_decimal(text: &str, token: &Token) -> bool {
    let prefix = &text[..token.start];
    prefix.ends_with('.')
        || prefix
            .chars()
            .rev()
            .find(|character| !character.is_whitespace())
            .is_some_and(|character| matches!(character, '+' | '-'))
}

fn invalid_numeric(message: &str) -> Error {
    Error::new(
        ErrorCode::InputInvalid,
        format!("requirements.invalid_numeric: {message}"),
    )
}

fn parse_formats(
    text: &str,
    tokens: &[Token],
    covered: &mut [bool],
    constraints: &mut Vec<Constraint>,
    spans: &mut Vec<RequirementSourceSpan>,
    ambiguities: &mut Vec<RequirementAmbiguity>,
) {
    let matches: Vec<_> = tokens
        .iter()
        .filter(|token| matches!(token.value.as_str(), "jpeg" | "jpg" | "jpe" | "png"))
        .collect();
    if matches.is_empty() {
        return;
    }
    let mut formats: Vec<String> = matches
        .iter()
        .filter_map(|token| ImageFormat::parse_constraint(&token.value))
        .map(|format| format.as_str().to_string())
        .collect();
    formats.sort();
    formats.dedup();

    let mut ambiguous_pairs = Vec::new();
    for pair in matches.windows(2) {
        let connectors: Vec<_> = tokens
            .iter()
            .filter(|token| token.start >= pair[0].end && token.end <= pair[1].start)
            .filter(|token| matches!(token.value.as_str(), "or" | "/"))
            .collect();
        for conjunction in tokens
            .iter()
            .filter(|token| token.start >= pair[0].end && token.end <= pair[1].start)
            .filter(|token| token.value == "and")
        {
            mark(covered, conjunction.start, conjunction.end);
        }
        if connectors.is_empty() && formats.len() > 1 {
            ambiguous_pairs.push((pair[0], pair[1]));
        }
        for connector in connectors {
            mark(covered, connector.start, connector.end);
        }
    }
    for item in &matches {
        mark(covered, item.start, item.end);
    }
    if !ambiguous_pairs.is_empty() {
        for (first, second) in ambiguous_pairs {
            let start = first.start;
            let end = second.end;
            ambiguities.push(RequirementAmbiguity {
                start,
                end,
                text: text[start..end].to_string(),
                message: "multiple image formats need an explicit 'or' to mean alternatives".into(),
            });
        }
        return;
    }

    constraints.push(Constraint {
        id: "image-format".into(),
        field: Field::ImageFormat,
        op: Operator::In,
        value: ConstraintValue::List(formats),
    });
    for item in matches {
        spans.push(source_span(text, item.start, item.end, &["image-format"]));
    }
}

fn parse_dimensions(
    text: &str,
    tokens: &[Token],
    covered: &mut [bool],
    constraints: &mut Vec<Constraint>,
    spans: &mut Vec<RequirementSourceSpan>,
) {
    let mut ordinal = 0_usize;
    for index in 0..tokens.len().saturating_sub(2) {
        let Some(width) = integer(&tokens[index]) else {
            continue;
        };
        if !matches!(tokens[index + 1].value.as_str(), "x" | "×") {
            continue;
        }
        let Some(height) = integer(&tokens[index + 2]) else {
            continue;
        };
        let (op, qualifier_start) =
            qualifier_before(tokens, index).unwrap_or((Operator::Eq, index));
        let mut end_index = index + 2;
        if tokens
            .get(end_index + 1)
            .is_some_and(|token| pixel_word(&token.value))
        {
            end_index += 1;
        }
        let start = tokens[qualifier_start].start;
        let end = tokens[end_index].end;
        if overlaps(covered, start, end) {
            continue;
        }
        let width_id = unique_id("image-width", ordinal);
        let height_id = unique_id("image-height", ordinal);
        constraints.push(numeric_constraint(&width_id, Field::ImageWidth, op, width));
        constraints.push(numeric_constraint(
            &height_id,
            Field::ImageHeight,
            op,
            height,
        ));
        mark(covered, start, end);
        spans.push(source_span(text, start, end, &[&width_id, &height_id]));
        ordinal += 1;
    }

    for index in 0..tokens.len() {
        let Some(field) = axis_field(&tokens[index].value) else {
            continue;
        };
        if covered[tokens[index].start..tokens[index].end]
            .iter()
            .any(|value| *value)
        {
            continue;
        }
        let Some((op, value, start_index, end_index)) = dimension_around_axis(tokens, index) else {
            continue;
        };
        let start = tokens[start_index].start;
        let end = tokens[end_index].end;
        if overlaps(covered, start, end) {
            continue;
        }
        let base = if field == Field::ImageWidth {
            "image-width"
        } else {
            "image-height"
        };
        let id = unique_id(base, ordinal);
        constraints.push(numeric_constraint(&id, field, op, value));
        mark(covered, start, end);
        spans.push(source_span(text, start, end, &[&id]));
        ordinal += 1;
    }
}

fn dimension_around_axis(tokens: &[Token], axis: usize) -> Option<(Operator, u64, usize, usize)> {
    if let Some((op, start)) = qualifier_before(tokens, axis) {
        let value_index = next_numeric(tokens, axis + 1)?;
        if value_index <= axis + 2 {
            let end = pixel_suffix(tokens, value_index);
            return Some((op, integer(&tokens[value_index])?, start, end));
        }
    }
    if let Some((op, qualifier_end)) = qualifier_after(tokens, axis + 1) {
        let value_index = next_numeric(tokens, qualifier_end + 1)?;
        if value_index == qualifier_end + 1 {
            let end = pixel_suffix(tokens, value_index);
            return Some((op, integer(&tokens[value_index])?, axis, end));
        }
    }
    if axis > 0 {
        let value_index = axis - 1;
        if let Some(value) = integer(&tokens[value_index])
            && let Some((op, start)) = qualifier_before(tokens, value_index)
        {
            return Some((op, value, start, axis));
        }
        if axis > 1 && pixel_word(&tokens[axis - 1].value) {
            let value_index = axis - 2;
            if let Some(value) = integer(&tokens[value_index])
                && let Some((op, start)) = qualifier_before(tokens, value_index)
            {
                return Some((op, value, start, axis));
            }
        }
    }
    None
}

fn parse_sizes(
    text: &str,
    tokens: &[Token],
    covered: &mut [bool],
    constraints: &mut Vec<Constraint>,
    spans: &mut Vec<RequirementSourceSpan>,
) -> Result<()> {
    let mut ordinal = 0_usize;
    for unit_index in 0..tokens.len() {
        if !matches!(
            tokens[unit_index].value.as_str(),
            "mb" | "mib" | "byte" | "bytes"
        ) || unit_index == 0
        {
            continue;
        }
        let number_index = unit_index - 1;
        if decimal(&tokens[number_index]).is_none() {
            continue;
        }
        let prefix = size_qualifier_before(tokens, number_index);
        let suffix = tokens.get(unit_index + 1).and_then(|token| {
            matches!(token.value.as_str(), "max" | "maximum" | "limit").then_some(unit_index + 1)
        });
        let Some(start_index) = prefix.or(Some(number_index).filter(|_| suffix.is_some())) else {
            continue;
        };
        let end_index = suffix.unwrap_or(unit_index);
        let start = tokens[start_index].start;
        let end = tokens[end_index].end;
        if overlaps(covered, start, end) {
            continue;
        }
        let amount = format!(
            "{} {}",
            tokens[number_index].value, tokens[unit_index].value
        );
        let value = parse_size_bytes(&amount)?;
        let id = unique_id("max-bytes", ordinal);
        constraints.push(numeric_constraint(
            &id,
            Field::FileBytes,
            Operator::Lte,
            value,
        ));
        mark(covered, start, end);
        spans.push(source_span(text, start, end, &[&id]));
        ordinal += 1;
    }
    Ok(())
}

fn qualifier_before(tokens: &[Token], index: usize) -> Option<(Operator, usize)> {
    if index == 0 {
        return None;
    }
    let mut cursor = index - 1;
    if matches!(
        tokens[cursor].value.as_str(),
        "dimension" | "dimensions" | "image"
    ) {
        if cursor == 0 {
            return None;
        }
        cursor -= 1;
    }
    match tokens[cursor].value.as_str() {
        "min" | "minimum" => Some((Operator::Gte, cursor)),
        "max" | "maximum" => Some((Operator::Lte, cursor)),
        "exact" | "exactly" => Some((Operator::Eq, cursor)),
        "least" if cursor > 0 && tokens[cursor - 1].value == "at" => {
            Some((Operator::Gte, cursor - 1))
        }
        "most" if cursor > 0 && tokens[cursor - 1].value == "at" => {
            Some((Operator::Lte, cursor - 1))
        }
        _ => None,
    }
}

fn qualifier_after(tokens: &[Token], index: usize) -> Option<(Operator, usize)> {
    let token = tokens.get(index)?;
    match token.value.as_str() {
        "minimum" | "min" | ">=" => Some((Operator::Gte, index)),
        "maximum" | "max" | "<=" => Some((Operator::Lte, index)),
        "exact" | "exactly" | "=" => Some((Operator::Eq, index)),
        "at" => match tokens.get(index + 1)?.value.as_str() {
            "least" => Some((Operator::Gte, index + 1)),
            "most" => Some((Operator::Lte, index + 1)),
            _ => None,
        },
        "no" if tokens.get(index + 1)?.value == "more"
            && tokens.get(index + 2)?.value == "than" =>
        {
            Some((Operator::Lte, index + 2))
        }
        _ => None,
    }
}

fn size_qualifier_before(tokens: &[Token], number: usize) -> Option<usize> {
    let floor = number.saturating_sub(5);
    for index in (floor..number).rev() {
        if matches!(
            tokens[index].value.as_str(),
            "under" | "max" | "maximum" | "limit" | "<="
        ) {
            return Some(index);
        }
        if tokens[index].value == "at"
            && tokens
                .get(index + 1)
                .is_some_and(|token| token.value == "most")
        {
            return Some(index);
        }
        if tokens[index].value == "no"
            && tokens
                .get(index + 1)
                .is_some_and(|token| token.value == "more")
            && tokens
                .get(index + 2)
                .is_some_and(|token| token.value == "than")
        {
            return Some(index);
        }
    }
    None
}

fn tokenize(text: &str) -> Vec<Token> {
    let mut result = Vec::new();
    let mut iterator = text.char_indices().peekable();
    while let Some((start, character)) = iterator.next() {
        if character.is_whitespace() || matches!(character, ',' | ';' | ':' | '(' | ')') {
            continue;
        }
        let class = token_class(character);
        if class == 0 {
            continue;
        }
        let mut end = start + character.len_utf8();
        if class <= 2 {
            while let Some(&(next_start, next)) = iterator.peek() {
                if token_class(next) != class && !(class == 1 && next == '.') {
                    break;
                }
                iterator.next();
                end = next_start + next.len_utf8();
            }
        } else if matches!(character, '<' | '>')
            && iterator.peek().is_some_and(|(_, next)| *next == '=')
        {
            let (next_start, next) = iterator.next().expect("peeked symbol");
            end = next_start + next.len_utf8();
        }
        result.push(Token {
            start,
            end,
            value: text[start..end].to_ascii_lowercase(),
        });
    }
    result
}

fn token_class(character: char) -> u8 {
    if character.is_ascii_digit() {
        1
    } else if character.is_alphabetic() {
        2
    } else if matches!(character, '×' | '/' | '=' | '<' | '>') {
        3
    } else {
        0
    }
}

fn integer(token: &Token) -> Option<u64> {
    token
        .value
        .bytes()
        .all(|byte| byte.is_ascii_digit())
        .then(|| token.value.parse().ok())
        .flatten()
}

fn decimal(token: &Token) -> Option<&str> {
    let mut dots = 0;
    token
        .value
        .bytes()
        .all(|byte| {
            if byte == b'.' {
                dots += 1;
                dots == 1
            } else {
                byte.is_ascii_digit()
            }
        })
        .then_some(token.value.as_str())
}

fn axis_field(value: &str) -> Option<Field> {
    match value {
        "width" | "wide" => Some(Field::ImageWidth),
        "height" | "high" => Some(Field::ImageHeight),
        _ => None,
    }
}

fn pixel_word(value: &str) -> bool {
    matches!(value, "px" | "pixel" | "pixels")
}

fn next_numeric(tokens: &[Token], start: usize) -> Option<usize> {
    (start..tokens.len().min(start + 2)).find(|index| integer(&tokens[*index]).is_some())
}

fn pixel_suffix(tokens: &[Token], value_index: usize) -> usize {
    if tokens
        .get(value_index + 1)
        .is_some_and(|token| pixel_word(&token.value))
    {
        value_index + 1
    } else {
        value_index
    }
}

fn numeric_constraint(id: &str, field: Field, op: Operator, value: u64) -> Constraint {
    Constraint {
        id: id.into(),
        field,
        op,
        value: ConstraintValue::Integer(value),
    }
}

fn unique_id(base: &str, ordinal: usize) -> String {
    if ordinal == 0 {
        base.into()
    } else {
        format!("{base}-{}", ordinal + 1)
    }
}

fn source_span(text: &str, start: usize, end: usize, ids: &[&str]) -> RequirementSourceSpan {
    RequirementSourceSpan {
        start,
        end,
        text: text[start..end].to_string(),
        constraint_ids: ids.iter().map(|id| (*id).to_string()).collect(),
    }
}

fn unresolved(text: &str, covered: &[bool]) -> Vec<UnresolvedRequirement> {
    let boundaries: Vec<usize> = text
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(text.len()))
        .collect();
    let mut result = Vec::new();
    let mut boundary = 0;
    while boundary + 1 < boundaries.len() {
        while boundary + 1 < boundaries.len() && covered[boundaries[boundary]] {
            boundary += 1;
        }
        let start_boundary = boundary;
        while boundary + 1 < boundaries.len() && !covered[boundaries[boundary]] {
            boundary += 1;
        }
        let start = boundaries[start_boundary];
        let end = boundaries[boundary];
        if start < end {
            let raw = &text[start..end];
            let leading = raw.len() - raw.trim_start_matches(unresolved_separator).len();
            let trailing = raw.len() - raw.trim_end_matches(unresolved_separator).len();
            let trimmed_start = start + leading;
            let trimmed_end = end.saturating_sub(trailing);
            if trimmed_start < trimmed_end
                && text[trimmed_start..trimmed_end]
                    .chars()
                    .any(char::is_alphanumeric)
            {
                result.push(UnresolvedRequirement {
                    start: trimmed_start,
                    end: trimmed_end,
                    text: text[trimmed_start..trimmed_end].to_string(),
                });
            }
        }
    }
    result
}

fn unresolved_separator(character: char) -> bool {
    character.is_whitespace() || matches!(character, ',' | ';' | ':' | '.' | '-' | '(' | ')' | '/')
}

fn mark(covered: &mut [bool], start: usize, end: usize) {
    covered[start..end].fill(true);
}

fn overlaps(covered: &[bool], start: usize, end: usize) -> bool {
    covered[start..end].iter().any(|covered| *covered)
}
