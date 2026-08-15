use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::artifact::{AudioCodec, Container, Family, VideoCodec};
pub use crate::contract::{CONSTRAINTS_SCHEMA, ConstraintsSchema};
use crate::error::{Error, ErrorCode, Result};

const MAX_CONSTRAINT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Field {
    #[serde(rename = "file.bytes")]
    FileBytes,
    #[serde(rename = "file.family")]
    FileFamily,
    #[serde(rename = "media.container")]
    MediaContainer,
    #[serde(rename = "media.video.codec")]
    MediaVideoCodec,
    #[serde(rename = "media.audio.codec")]
    MediaAudioCodec,
    #[serde(rename = "media.video.width")]
    MediaVideoWidth,
    #[serde(rename = "media.video.height")]
    MediaVideoHeight,
    #[serde(rename = "media.video.pixel_format")]
    MediaVideoPixelFormat,
    #[serde(rename = "media.video.bit_depth")]
    MediaVideoBitDepth,
    #[serde(rename = "media.video.color_range")]
    MediaVideoColorRange,
    #[serde(rename = "media.video.color_space")]
    MediaVideoColorSpace,
    #[serde(rename = "media.video.color_transfer")]
    MediaVideoColorTransfer,
    #[serde(rename = "media.video.color_primaries")]
    MediaVideoColorPrimaries,
    #[serde(rename = "media.video.hdr")]
    MediaVideoHdr,
}

impl Field {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FileBytes => "file.bytes",
            Self::FileFamily => "file.family",
            Self::MediaContainer => "media.container",
            Self::MediaVideoCodec => "media.video.codec",
            Self::MediaAudioCodec => "media.audio.codec",
            Self::MediaVideoWidth => "media.video.width",
            Self::MediaVideoHeight => "media.video.height",
            Self::MediaVideoPixelFormat => "media.video.pixel_format",
            Self::MediaVideoBitDepth => "media.video.bit_depth",
            Self::MediaVideoColorRange => "media.video.color_range",
            Self::MediaVideoColorSpace => "media.video.color_space",
            Self::MediaVideoColorTransfer => "media.video.color_transfer",
            Self::MediaVideoColorPrimaries => "media.video.color_primaries",
            Self::MediaVideoHdr => "media.video.hdr",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    Eq,
    In,
    Lte,
    Gte,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConstraintValue {
    Integer(u64),
    Text(String),
    List(Vec<String>),
}

impl ConstraintValue {
    pub fn display(&self) -> String {
        match self {
            Self::Integer(n) => n.to_string(),
            Self::Text(s) => s.clone(),
            Self::List(items) => items.join(", "),
        }
    }

    pub fn as_text_list(&self) -> Vec<String> {
        match self {
            Self::List(items) => items.clone(),
            Self::Text(s) => vec![s.clone()],
            Self::Integer(n) => vec![n.to_string()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Constraint {
    pub id: String,
    pub field: Field,
    pub op: Operator,
    pub value: ConstraintValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    #[serde(default = "default_true")]
    pub preserve_audio: bool,
    #[serde(default = "default_true")]
    pub preserve_resolution: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            preserve_audio: true,
            preserve_resolution: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstraintSet {
    pub schema: ConstraintsSchema,
    pub hard: Vec<Constraint>,
    #[serde(default)]
    pub preferences: Preferences,
}

/// Structured CLI input. Natural-language parsing is intentionally out of scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstraintInput {
    pub container: Option<Vec<String>>,
    pub video_codec: Option<Vec<String>>,
    pub audio_codec: Option<Vec<String>>,
    pub family: Option<String>,
    pub max_bytes: Option<u64>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    #[serde(default = "default_true")]
    pub preserve_audio: bool,
    #[serde(default = "default_true")]
    pub preserve_resolution: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ConstraintInput {
    fn default() -> Self {
        Self {
            container: None,
            video_codec: None,
            audio_codec: None,
            family: None,
            max_bytes: None,
            max_width: None,
            max_height: None,
            preserve_audio: true,
            preserve_resolution: true,
        }
    }
}

pub fn compile(input: ConstraintInput) -> Result<ConstraintSet> {
    let mut hard = Vec::new();

    if let Some(family) = input.family {
        hard.push(Constraint {
            id: "family".into(),
            field: Field::FileFamily,
            op: Operator::Eq,
            value: ConstraintValue::Text(family),
        });
    }
    if let Some(values) = input.container {
        hard.push(Constraint {
            id: "container".into(),
            field: Field::MediaContainer,
            op: Operator::In,
            value: ConstraintValue::List(values),
        });
    }
    if let Some(values) = input.video_codec {
        hard.push(Constraint {
            id: "video-codec".into(),
            field: Field::MediaVideoCodec,
            op: Operator::In,
            value: ConstraintValue::List(values),
        });
    }
    if let Some(values) = input.audio_codec {
        hard.push(Constraint {
            id: "audio-codec".into(),
            field: Field::MediaAudioCodec,
            op: Operator::In,
            value: ConstraintValue::List(values),
        });
    }
    if let Some(max_bytes) = input.max_bytes {
        hard.push(Constraint {
            id: "max-bytes".into(),
            field: Field::FileBytes,
            op: Operator::Lte,
            value: ConstraintValue::Integer(max_bytes),
        });
    }
    if let Some(max_width) = input.max_width {
        hard.push(Constraint {
            id: "max-width".into(),
            field: Field::MediaVideoWidth,
            op: Operator::Lte,
            value: ConstraintValue::Integer(u64::from(max_width)),
        });
    }
    if let Some(max_height) = input.max_height {
        hard.push(Constraint {
            id: "max-height".into(),
            field: Field::MediaVideoHeight,
            op: Operator::Lte,
            value: ConstraintValue::Integer(u64::from(max_height)),
        });
    }

    validate_and_normalize(ConstraintSet {
        schema: ConstraintsSchema,
        hard,
        preferences: Preferences {
            preserve_audio: input.preserve_audio,
            preserve_resolution: input.preserve_resolution,
        },
    })
}

pub fn compile_from_yaml(text: &str) -> Result<ConstraintSet> {
    enforce_input_limit(text)?;
    let parsed: ConstraintSet = yaml_serde::from_str(text).map_err(|error| {
        invalid(
            "constraints.invalid_document",
            format!("constraint YAML is invalid: {error}"),
        )
    })?;
    validate_and_normalize(parsed)
}

/// Parse and semantically validate the public JSON constraint contract.
pub fn compile_from_json(text: &str) -> Result<ConstraintSet> {
    enforce_input_limit(text)?;
    let parsed: ConstraintSet = serde_json::from_str(text).map_err(|error| {
        invalid(
            "constraints.invalid_document",
            format!("constraint JSON is invalid: {error}"),
        )
    })?;
    validate_and_normalize(parsed)
}

fn enforce_input_limit(text: &str) -> Result<()> {
    if text.len() > MAX_CONSTRAINT_BYTES {
        return Err(invalid(
            "constraints.input_too_large",
            "constraint input exceeds the 1 MiB limit",
        ));
    }
    Ok(())
}

fn validate_and_normalize(mut constraints: ConstraintSet) -> Result<ConstraintSet> {
    if constraints.hard.is_empty() {
        return Err(invalid(
            "constraints.empty_target",
            "at least one hard constraint is required",
        ));
    }

    let mut ids = HashSet::new();
    for constraint in &mut constraints.hard {
        let normalized_id = constraint.id.trim();
        if normalized_id.is_empty() {
            return Err(invalid(
                "constraints.blank_id",
                "constraint IDs cannot be blank",
            ));
        }
        if !ids.insert(normalized_id.to_string()) {
            return Err(invalid(
                "constraints.duplicate_id",
                format!("duplicate constraint ID: {normalized_id}"),
            ));
        }
        constraint.id = normalized_id.to_string();
        normalize_constraint(constraint)?;
    }
    reject_conflicts(&constraints.hard)?;
    Ok(constraints)
}

fn normalize_constraint(constraint: &mut Constraint) -> Result<()> {
    match (constraint.field, constraint.op, &mut constraint.value) {
        (Field::FileFamily, Operator::Eq, ConstraintValue::Text(value)) => {
            let normalized = Family::parse_constraint(value).ok_or_else(|| {
                invalid(
                    "constraints.unknown_family",
                    format!("unknown family: {value}"),
                )
            })?;
            *value = normalized.as_str().to_string();
        }
        (Field::MediaContainer, Operator::In, ConstraintValue::List(values)) => {
            normalize_list(values, "container", |value| {
                Container::parse_constraint(value).map(|value| value.as_str().to_string())
            })?;
        }
        (Field::MediaVideoCodec, Operator::In, ConstraintValue::List(values)) => {
            normalize_list(values, "video codec", |value| {
                VideoCodec::parse_constraint(value).map(|value| value.as_str().to_string())
            })?;
        }
        (Field::MediaAudioCodec, Operator::In, ConstraintValue::List(values)) => {
            normalize_list(values, "audio codec", |value| {
                AudioCodec::parse_constraint(value).map(|value| value.as_str().to_string())
            })?;
        }
        (
            Field::FileBytes | Field::MediaVideoWidth | Field::MediaVideoHeight,
            Operator::Lte,
            ConstraintValue::Integer(value),
        ) if *value > 0 => {}
        _ => {
            return Err(invalid(
                "constraints.invalid_combination",
                format!(
                    "invalid operator or value for {}",
                    constraint.field.as_str()
                ),
            ));
        }
    }
    Ok(())
}

fn normalize_list(
    values: &mut Vec<String>,
    label: &str,
    normalize: impl Fn(&str) -> Option<String>,
) -> Result<()> {
    if values.is_empty() {
        return Err(invalid(
            "constraints.empty_list",
            format!("{label} list cannot be empty"),
        ));
    }
    for value in values.iter_mut() {
        let normalized = normalize(value).ok_or_else(|| {
            invalid(
                "constraints.unknown_enum_value",
                format!("unknown {label}: {value}"),
            )
        })?;
        *value = normalized;
    }
    values.sort();
    values.dedup();
    Ok(())
}

fn reject_conflicts(constraints: &[Constraint]) -> Result<()> {
    let mut by_field: HashMap<Field, Vec<&Constraint>> = HashMap::new();
    for constraint in constraints {
        by_field
            .entry(constraint.field)
            .or_default()
            .push(constraint);
    }
    for (field, group) in by_field {
        if group.len() < 2 {
            continue;
        }
        let conflict = match group[0].op {
            Operator::Eq => group.windows(2).any(|pair| pair[0].value != pair[1].value),
            Operator::In => {
                let mut allowed: HashSet<String> =
                    group[0].value.as_text_list().into_iter().collect();
                for constraint in &group[1..] {
                    let next: HashSet<String> =
                        constraint.value.as_text_list().into_iter().collect();
                    allowed.retain(|value| next.contains(value));
                }
                allowed.is_empty()
            }
            Operator::Lte | Operator::Gte => false,
        };
        if conflict {
            return Err(Error::new(
                ErrorCode::RequirementsConflict,
                format!(
                    "constraints.conflict: hard constraints conflict for {}",
                    field.as_str()
                ),
            ));
        }
    }
    Ok(())
}

/// Parse exact byte counts plus decimal MB and binary MiB values.
pub fn parse_size_bytes(raw: &str) -> Result<u64> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(invalid("size.invalid", "size cannot be empty"));
    }
    let lower = value.to_ascii_lowercase();
    let (number, multiplier) = if lower.ends_with("mib") {
        (&value[..value.len() - 3], Some(1_048_576_u64))
    } else if lower.ends_with("mb") {
        (&value[..value.len() - 2], Some(1_000_000_u64))
    } else {
        (value, None)
    };
    let number = number.trim();
    match multiplier {
        None => {
            if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(invalid(
                    "size.invalid",
                    "unadorned sizes must be whole bytes",
                ));
            }
            number
                .parse()
                .map_err(|_| invalid("size.overflow", "size exceeds the byte range"))
        }
        Some(multiplier) => parse_scaled_decimal(number, multiplier),
    }
}

fn parse_scaled_decimal(number: &str, multiplier: u64) -> Result<u64> {
    let (whole, fraction) = match number.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (number, None),
    };
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.is_some_and(|fraction| {
            fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        })
    {
        return Err(invalid("size.invalid", "size has an invalid decimal value"));
    }
    let whole: u128 = whole
        .parse()
        .map_err(|_| invalid("size.overflow", "size exceeds the byte range"))?;
    let (numerator, denominator) = if let Some(fraction) = fraction {
        let exponent = u32::try_from(fraction.len()).map_err(|_| {
            invalid(
                "size.overflow",
                "size precision exceeds the supported range",
            )
        })?;
        let denominator = 10_u128.checked_pow(exponent).ok_or_else(|| {
            invalid(
                "size.overflow",
                "size precision exceeds the supported range",
            )
        })?;
        let fraction: u128 = fraction
            .parse()
            .map_err(|_| invalid("size.overflow", "size exceeds the byte range"))?;
        (
            whole
                .checked_mul(denominator)
                .and_then(|value| value.checked_add(fraction))
                .ok_or_else(|| invalid("size.overflow", "size exceeds the byte range"))?,
            denominator,
        )
    } else {
        (whole, 1)
    };
    let scaled = numerator
        .checked_mul(u128::from(multiplier))
        .ok_or_else(|| invalid("size.overflow", "size exceeds the byte range"))?;
    if scaled % denominator != 0 {
        return Err(invalid(
            "size.fractional_byte",
            "size does not resolve to a whole byte",
        ));
    }
    u64::try_from(scaled / denominator)
        .map_err(|_| invalid("size.overflow", "size exceeds the byte range"))
}

fn invalid(reason: &str, message: impl Into<String>) -> Error {
    Error::new(
        ErrorCode::InputInvalid,
        format!("{reason}: {}", message.into()),
    )
}

/// Common v0.1 target: MP4 / H.264 / AAC.
pub fn media_h264_mp4_aac() -> ConstraintSet {
    compile(ConstraintInput {
        container: Some(vec!["mp4".into()]),
        video_codec: Some(vec!["h264".into()]),
        audio_codec: Some(vec!["aac".into()]),
        preserve_audio: true,
        preserve_resolution: true,
        ..ConstraintInput::default()
    })
    .expect("built-in constraints are valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_normalizes_codec_aliases() {
        let set = compile(ConstraintInput {
            video_codec: Some(vec!["avc1".into()]),
            ..ConstraintInput::default()
        })
        .unwrap();
        assert_eq!(set.hard[0].id, "video-codec");
        assert_eq!(
            set.hard[0].value,
            ConstraintValue::List(vec!["h264".into()])
        );
    }
}
