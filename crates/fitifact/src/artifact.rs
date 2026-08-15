use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    Media,
    Image,
    Unknown,
}

impl Family {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Media => "media",
            Self::Image => "image",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Container {
    Mp4,
    Mov,
    Webm,
    Mkv,
    Unknown(String),
}

impl Container {
    pub fn parse_loose(raw: &str) -> Self {
        let lower = raw.to_ascii_lowercase();
        if lower.split(',').any(|p| p.trim() == "mp4") || lower.contains("isom") {
            return Self::Mp4;
        }
        if lower.contains("webm") {
            return Self::Webm;
        }
        if lower.contains("matroska") || lower.split(',').any(|p| p.trim() == "mkv") {
            return Self::Mkv;
        }
        if lower.contains("mov") || lower.contains("quicktime") || lower.trim() == "qt" {
            return Self::Mov;
        }
        Self::Unknown(raw.to_string())
    }

    pub fn from_probe(format_name: &str, major_brand: Option<&str>) -> Self {
        if let Some(brand) = major_brand.map(str::trim) {
            let brand = brand.trim_end_matches('\0').trim();
            if brand == "qt" || brand.starts_with("qt") {
                return Self::Mov;
            }
            if matches!(
                brand,
                "isom" | "iso2" | "iso5" | "iso6" | "mp41" | "mp42" | "mp71" | "avc1" | "msdh"
            ) {
                return Self::Mp4;
            }
        }
        Self::parse_loose(format_name)
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mov => "mov",
            Self::Webm => "webm",
            Self::Mkv => "mkv",
            Self::Unknown(other) => other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoCodec {
    H264,
    Hevc,
    Vp9,
    Av1,
    Unknown(String),
}

impl VideoCodec {
    pub fn parse_loose(raw: &str) -> Self {
        match raw.to_ascii_lowercase().as_str() {
            "h264" | "avc" | "avc1" | "avc3" | "libx264" => Self::H264,
            "hevc" | "h265" | "hvc1" | "hev1" | "libx265" => Self::Hevc,
            "vp9" | "vp09" => Self::Vp9,
            "av1" | "av01" => Self::Av1,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::H264 => "h264",
            Self::Hevc => "hevc",
            Self::Vp9 => "vp9",
            Self::Av1 => "av1",
            Self::Unknown(other) => other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioCodec {
    Aac,
    Mp3,
    Opus,
    Unknown(String),
}

impl AudioCodec {
    pub fn parse_loose(raw: &str) -> Self {
        match raw.to_ascii_lowercase().as_str() {
            "aac" | "mp4a" => Self::Aac,
            "mp3" | "mp3float" => Self::Mp3,
            "opus" => Self::Opus,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Aac => "aac",
            Self::Mp3 => "mp3",
            Self::Opus => "opus",
            Self::Unknown(other) => other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoStream {
    pub codec: Option<VideoCodec>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub sample_rate: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioStream {
    pub codec: Option<AudioCodec>,
    pub channels: Option<u32>,
    pub sample_rate: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectionMeta {
    pub provider: String,
    pub provider_version: Option<String>,
    pub completeness: Completeness,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Completeness {
    Full,
    Partial,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    pub path: Option<PathBuf>,
    pub family: Family,
    pub byte_length: u64,
    pub container: Option<Container>,
    pub video: Option<VideoStream>,
    pub audio: Option<AudioStream>,
    pub duration_ms: Option<u64>,
    pub inspection: InspectionMeta,
}

impl Artifact {
    pub fn media(
        container: Container,
        video: VideoCodec,
        audio: Option<AudioCodec>,
        bytes: u64,
    ) -> Self {
        Self {
            path: None,
            family: Family::Media,
            byte_length: bytes,
            container: Some(container),
            video: Some(VideoStream {
                codec: Some(video),
                width: Some(1920),
                height: Some(1080),
                sample_rate: None,
            }),
            audio: audio.map(|codec| AudioStream {
                codec: Some(codec),
                channels: Some(2),
                sample_rate: Some(48_000),
            }),
            duration_ms: Some(1_000),
            inspection: InspectionMeta {
                provider: "fixture".into(),
                provider_version: None,
                completeness: Completeness::Full,
                warnings: Vec::new(),
            },
        }
    }

    pub fn image_stub(bytes: u64) -> Self {
        Self {
            path: None,
            family: Family::Image,
            byte_length: bytes,
            container: None,
            video: None,
            audio: None,
            duration_ms: None,
            inspection: InspectionMeta {
                provider: "fixture".into(),
                provider_version: None,
                completeness: Completeness::Full,
                warnings: Vec::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_from_probe_uses_major_brand_not_format_name_soup() {
        assert_eq!(
            Container::from_probe("mov,mp4,m4a,3gp,3g2,mj2", Some("qt  ")),
            Container::Mov
        );
        assert_eq!(
            Container::from_probe("mov,mp4,m4a,3gp,3g2,mj2", Some("isom")),
            Container::Mp4
        );
    }

    #[test]
    fn video_codec_parse_does_not_treat_hevc_as_h264() {
        assert_eq!(VideoCodec::parse_loose("hvc1"), VideoCodec::Hevc);
        assert_eq!(VideoCodec::parse_loose("avc1"), VideoCodec::H264);
    }
}
