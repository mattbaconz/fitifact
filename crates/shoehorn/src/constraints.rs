use serde::{Deserialize, Serialize};

use crate::artifact::{AudioCodec, Container, Family, VideoCodec};
use crate::error::{Error, ErrorCode, Result};

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
pub struct Constraint {
    pub id: String,
    pub field: Field,
    pub op: Operator,
    pub value: ConstraintValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Preferences {
    pub preserve_audio: bool,
    pub preserve_resolution: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ConstraintSet {
    pub hard: Vec<Constraint>,
    pub preferences: Preferences,
}

/// Structured flags / YAML input. Not natural-language parsing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

pub fn compile(input: ConstraintInput) -> ConstraintSet {
    let mut hard = Vec::new();

    if let Some(family) = input.family {
        hard.push(Constraint {
            id: "family".into(),
            field: Field::FileFamily,
            op: Operator::Eq,
            value: ConstraintValue::Text(normalize_family(&family)),
        });
    }
    if let Some(values) = input.container.filter(|v| !v.is_empty()) {
        hard.push(Constraint {
            id: "container".into(),
            field: Field::MediaContainer,
            op: Operator::In,
            value: ConstraintValue::List(
                values
                    .iter()
                    .map(|v| Container::parse_loose(v).as_str().to_string())
                    .collect(),
            ),
        });
    }
    if let Some(values) = input.video_codec.filter(|v| !v.is_empty()) {
        hard.push(Constraint {
            id: "video-codec".into(),
            field: Field::MediaVideoCodec,
            op: Operator::In,
            value: ConstraintValue::List(
                values
                    .iter()
                    .map(|v| VideoCodec::parse_loose(v).as_str().to_string())
                    .collect(),
            ),
        });
    }
    if let Some(values) = input.audio_codec.filter(|v| !v.is_empty()) {
        hard.push(Constraint {
            id: "audio-codec".into(),
            field: Field::MediaAudioCodec,
            op: Operator::In,
            value: ConstraintValue::List(
                values
                    .iter()
                    .map(|v| AudioCodec::parse_loose(v).as_str().to_string())
                    .collect(),
            ),
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

    ConstraintSet {
        hard,
        preferences: Preferences {
            preserve_audio: input.preserve_audio,
            preserve_resolution: input.preserve_resolution,
        },
    }
}

fn normalize_family(raw: &str) -> String {
    match Family::from_str_loose(raw) {
        Family::Media => "media".into(),
        Family::Image => "image".into(),
        Family::Unknown => raw.to_ascii_lowercase(),
    }
}

impl Family {
    pub fn from_str_loose(raw: &str) -> Self {
        match raw.to_ascii_lowercase().as_str() {
            "media" | "video" | "audio" => Self::Media,
            "image" | "picture" => Self::Image,
            _ => Self::Unknown,
        }
    }
}

pub fn compile_from_yaml(text: &str) -> Result<ConstraintSet> {
    let yaml: serde_yaml::Value = serde_yaml::from_str(text).map_err(|err| {
        Error::new(
            ErrorCode::RequirementsAmbiguous,
            format!("constraints yaml could not be parsed: {err}"),
        )
    })?;
    if yaml.get("hard").is_some() {
        serde_yaml::from_value(yaml).map_err(|err| {
            Error::new(
                ErrorCode::RequirementsAmbiguous,
                format!("constraint set yaml was invalid: {err}"),
            )
        })
    } else {
        let input: ConstraintInput = serde_yaml::from_value(yaml).map_err(|err| {
            Error::new(
                ErrorCode::RequirementsAmbiguous,
                format!("constraint input yaml was invalid: {err}"),
            )
        })?;
        Ok(compile(input))
    }
}

/// Common demo target: MP4 / H.264 / AAC.
pub fn media_h264_mp4_aac() -> ConstraintSet {
    compile(ConstraintInput {
        container: Some(vec!["mp4".into()]),
        video_codec: Some(vec!["h264".into()]),
        audio_codec: Some(vec!["aac".into()]),
        preserve_audio: true,
        preserve_resolution: true,
        ..ConstraintInput::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_normalizes_codec_aliases() {
        let set = compile(ConstraintInput {
            video_codec: Some(vec!["avc1".into()]),
            ..ConstraintInput::default()
        });
        assert_eq!(set.hard[0].id, "video-codec");
        assert_eq!(
            set.hard[0].value,
            ConstraintValue::List(vec!["h264".into()])
        );
    }

    #[test]
    fn compile_from_yaml_flags_shape() {
        let set = compile_from_yaml("container: [mp4]\nvideo_codec: [h264]\n").unwrap();
        assert_eq!(
            set.hard.iter().find(|c| c.id == "container").unwrap().field,
            Field::MediaContainer
        );
        assert!(set.preferences.preserve_audio);
    }
}
