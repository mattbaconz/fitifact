use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub use crate::contract::{ARTIFACT_SCHEMA, ArtifactSchema};

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

    pub fn parse_constraint(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "media" | "video" | "audio" => Some(Self::Media),
            "image" | "picture" => Some(Self::Image),
            _ => None,
        }
    }

    pub fn from_str_loose(raw: &str) -> Self {
        Self::parse_constraint(raw).unwrap_or(Self::Unknown)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageFormat {
    Jpeg,
    Png,
    Webp,
    Tiff,
    Heif,
    Gif,
    Unknown(String),
}

impl ImageFormat {
    pub fn parse_constraint(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "jpeg" | "jpg" | "jpe" => Some(Self::Jpeg),
            "png" => Some(Self::Png),
            "webp" => Some(Self::Webp),
            "tiff" | "tif" => Some(Self::Tiff),
            "heif" | "heic" => Some(Self::Heif),
            "gif" => Some(Self::Gif),
            _ => None,
        }
    }

    pub fn parse_loose(raw: &str) -> Self {
        Self::parse_constraint(raw).unwrap_or_else(|| Self::Unknown(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Jpeg => "jpeg",
            Self::Png => "png",
            Self::Webp => "webp",
            Self::Tiff => "tiff",
            Self::Heif => "heif",
            Self::Gif => "gif",
            Self::Unknown(other) => other,
        }
    }

    pub fn display_label(&self) -> String {
        match self {
            Self::Unknown(other) => format!("unknown ({other})"),
            known => known.as_str().to_ascii_uppercase(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageFacts {
    pub format: Option<ImageFormat>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub alpha: Option<bool>,
    pub animated: Option<bool>,
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
    pub fn parse_constraint(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "mp4" => Some(Self::Mp4),
            "mov" | "quicktime" | "qt" => Some(Self::Mov),
            "webm" => Some(Self::Webm),
            "mkv" | "matroska" => Some(Self::Mkv),
            _ => None,
        }
    }

    pub fn parse_loose(raw: &str) -> Self {
        let mut found = None;
        for part in raw.split(',') {
            let token = part.trim();
            if token.is_empty() {
                continue;
            }
            let Some(parsed) = Self::parse_constraint(token) else {
                return Self::Unknown(raw.to_string());
            };
            match &found {
                None => found = Some(parsed),
                Some(existing) if *existing == parsed => {}
                Some(_) => return Self::Unknown(raw.to_string()),
            }
        }
        found.unwrap_or_else(|| Self::Unknown(raw.to_string()))
    }

    pub fn from_probe(format_name: &str, major_brand: Option<&str>) -> Self {
        let normalized = format_name.trim().to_ascii_lowercase();
        if iso_bmff_family_label(&normalized) {
            if let Some(brand) = major_brand.map(str::trim) {
                let brand = brand.trim_end_matches('\0').trim().to_ascii_lowercase();
                if brand == "qt" || brand.starts_with("qt") {
                    return Self::Mov;
                }
                if matches!(
                    brand.as_str(),
                    "isom" | "iso2" | "iso5" | "iso6" | "mp41" | "mp42" | "mp71" | "avc1" | "msdh"
                ) {
                    return Self::Mp4;
                }
            }
            return match normalized.as_str() {
                "mp4" => Self::Mp4,
                "mov" | "quicktime" | "qt" => Self::Mov,
                _ => Self::Unknown(format_name.to_string()),
            };
        }
        match normalized.as_str() {
            "webm" => Self::Webm,
            "matroska" | "mkv" => Self::Mkv,
            _ => Self::Unknown(format_name.to_string()),
        }
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

    pub fn display_label(&self) -> String {
        match self {
            Self::Unknown(other) => format!("unknown ({other})"),
            known => known.as_str().to_ascii_uppercase(),
        }
    }
}

fn iso_bmff_family_label(format_name: &str) -> bool {
    !format_name.is_empty()
        && format_name.split(',').all(|part| {
            matches!(
                part.trim(),
                "mp4" | "mov" | "quicktime" | "qt" | "m4a" | "3gp" | "3g2" | "mj2"
            )
        })
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
    pub fn parse_constraint(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "h264" | "avc" | "avc1" | "avc3" => Some(Self::H264),
            "hevc" | "h265" | "hvc1" | "hev1" => Some(Self::Hevc),
            "vp9" | "vp09" => Some(Self::Vp9),
            "av1" | "av01" => Some(Self::Av1),
            _ => None,
        }
    }

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
    pub fn parse_constraint(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "aac" | "mp4a" => Some(Self::Aac),
            "mp3" | "mp3float" => Some(Self::Mp3),
            "opus" => Some(Self::Opus),
            _ => None,
        }
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rational {
    pub numerator: u32,
    pub denominator: u32,
}

impl Rational {
    pub fn new(numerator: u32, denominator: u32) -> Option<Self> {
        (denominator != 0).then_some(Self {
            numerator,
            denominator,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HdrStatus {
    Sdr,
    Hdr,
    Unknown,
}

impl HdrStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sdr => "sdr",
            Self::Hdr => "hdr",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoStream {
    pub codec: Option<VideoCodec>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<Rational>,
    pub pixel_format: Option<String>,
    pub bit_depth: Option<u8>,
    pub color_range: Option<String>,
    pub color_space: Option<String>,
    pub color_transfer: Option<String>,
    pub color_primaries: Option<String>,
    pub hdr: HdrStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioStream {
    pub codec: Option<AudioCodec>,
    pub channels: Option<u32>,
    pub sample_rate: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtherStream {
    pub codec: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamDetails {
    Video {
        #[serde(flatten)]
        facts: VideoStream,
    },
    Audio {
        #[serde(flatten)]
        facts: AudioStream,
    },
    Subtitle {
        #[serde(flatten)]
        facts: OtherStream,
    },
    Data {
        #[serde(flatten)]
        facts: OtherStream,
    },
    Attachment {
        #[serde(flatten)]
        facts: OtherStream,
    },
    Unknown {
        original_type: Option<String>,
        #[serde(flatten)]
        facts: OtherStream,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stream {
    pub index: Option<u32>,
    #[serde(flatten)]
    pub details: StreamDetails,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamType {
    Video,
    Audio,
    Subtitle,
    Data,
    Attachment,
    Unknown(Option<String>),
}

impl Stream {
    pub fn stream_type(&self) -> StreamType {
        match &self.details {
            StreamDetails::Video { .. } => StreamType::Video,
            StreamDetails::Audio { .. } => StreamType::Audio,
            StreamDetails::Subtitle { .. } => StreamType::Subtitle,
            StreamDetails::Data { .. } => StreamType::Data,
            StreamDetails::Attachment { .. } => StreamType::Attachment,
            StreamDetails::Unknown { original_type, .. } => {
                StreamType::Unknown(original_type.clone())
            }
        }
    }

    pub fn video(&self) -> Option<&VideoStream> {
        match &self.details {
            StreamDetails::Video { facts } => Some(facts),
            _ => None,
        }
    }

    pub fn video_mut(&mut self) -> Option<&mut VideoStream> {
        match &mut self.details {
            StreamDetails::Video { facts } => Some(facts),
            _ => None,
        }
    }

    pub fn audio(&self) -> Option<&AudioStream> {
        match &self.details {
            StreamDetails::Audio { facts } => Some(facts),
            _ => None,
        }
    }

    pub fn audio_mut(&mut self) -> Option<&mut AudioStream> {
        match &mut self.details {
            StreamDetails::Audio { facts } => Some(facts),
            _ => None,
        }
    }
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
    pub schema: ArtifactSchema,
    pub path: Option<PathBuf>,
    pub family: Family,
    pub byte_length: u64,
    pub container: Option<Container>,
    pub streams: Vec<Stream>,
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageFacts>,
    pub inspection: InspectionMeta,
}

impl Artifact {
    pub fn media(
        container: Container,
        video: VideoCodec,
        audio: Option<AudioCodec>,
        bytes: u64,
    ) -> Self {
        let mut streams = vec![Stream {
            index: Some(0),
            details: StreamDetails::Video {
                facts: VideoStream {
                    codec: Some(video),
                    width: Some(1920),
                    height: Some(1080),
                    frame_rate: Some(Rational {
                        numerator: 30,
                        denominator: 1,
                    }),
                    pixel_format: Some("yuv420p".into()),
                    bit_depth: Some(8),
                    color_range: Some("tv".into()),
                    color_space: Some("bt709".into()),
                    color_transfer: Some("bt709".into()),
                    color_primaries: Some("bt709".into()),
                    hdr: HdrStatus::Sdr,
                },
            },
        }];
        if let Some(codec) = audio {
            streams.push(Stream {
                index: Some(1),
                details: StreamDetails::Audio {
                    facts: AudioStream {
                        codec: Some(codec),
                        channels: Some(2),
                        sample_rate: Some(48_000),
                    },
                },
            });
        }
        Self {
            schema: ArtifactSchema,
            path: None,
            family: Family::Media,
            byte_length: bytes,
            container: Some(container),
            streams,
            duration_ms: Some(1_000),
            image: None,
            inspection: InspectionMeta {
                provider: "fixture".into(),
                provider_version: Some("test".into()),
                completeness: Completeness::Full,
                warnings: Vec::new(),
            },
        }
    }

    pub fn image(
        format: ImageFormat,
        width: u32,
        height: u32,
        bytes: u64,
        alpha: bool,
        animated: bool,
    ) -> Self {
        Self {
            schema: ArtifactSchema,
            path: None,
            family: Family::Image,
            byte_length: bytes,
            container: None,
            streams: Vec::new(),
            duration_ms: None,
            image: Some(ImageFacts {
                format: Some(format),
                width: Some(width),
                height: Some(height),
                alpha: Some(alpha),
                animated: Some(animated),
            }),
            inspection: InspectionMeta {
                provider: "fixture".into(),
                provider_version: Some("test".into()),
                completeness: Completeness::Full,
                warnings: Vec::new(),
            },
        }
    }

    pub fn video_streams(&self) -> impl Iterator<Item = &VideoStream> {
        self.streams.iter().filter_map(Stream::video)
    }

    pub fn audio_streams(&self) -> impl Iterator<Item = &AudioStream> {
        self.streams.iter().filter_map(Stream::audio)
    }

    pub fn first_video(&self) -> Option<&VideoStream> {
        self.video_streams().next()
    }

    pub fn first_video_mut(&mut self) -> Option<&mut VideoStream> {
        self.streams.iter_mut().find_map(Stream::video_mut)
    }

    pub fn first_audio(&self) -> Option<&AudioStream> {
        self.audio_streams().next()
    }

    pub fn first_audio_mut(&mut self) -> Option<&mut AudioStream> {
        self.streams.iter_mut().find_map(Stream::audio_mut)
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
    fn ambiguous_mov_family_probe_requires_a_recognized_brand() {
        let label = "mov,mp4,m4a,3gp,3g2,mj2";
        for brand in [None, Some("3gp4"), Some("mystery")] {
            assert_eq!(
                Container::from_probe(label, brand),
                Container::Unknown(label.into()),
                "brand {brand:?} must not imply MP4"
            );
        }
    }

    #[test]
    fn unique_probe_labels_remain_affirmative_container_evidence() {
        assert_eq!(Container::from_probe("mp4", None), Container::Mp4);
        assert_eq!(Container::from_probe("mov", None), Container::Mov);
        assert_eq!(Container::from_probe("webm", None), Container::Webm);
        assert_eq!(
            Container::from_probe("matroska,webm", None),
            Container::Unknown("matroska,webm".into())
        );
        assert_eq!(Container::from_probe("matroska", None), Container::Mkv);
    }

    #[test]
    fn probe_brands_do_not_promote_non_iso_bmff_labels() {
        assert_eq!(
            Container::from_probe("matroska,webm", Some("isom")),
            Container::Unknown("matroska,webm".into())
        );
        assert_eq!(Container::from_probe("webm", Some("isom")), Container::Webm);
        assert_eq!(
            Container::from_probe("matroska", Some("qt")),
            Container::Mkv
        );
    }

    #[test]
    fn video_codec_parse_does_not_treat_hevc_as_h264() {
        assert_eq!(VideoCodec::parse_loose("hvc1"), VideoCodec::Hevc);
        assert_eq!(VideoCodec::parse_loose("avc1"), VideoCodec::H264);
    }

    #[test]
    fn parse_loose_uses_exact_tokens_and_keeps_ambiguous_soup_unknown() {
        assert_eq!(Container::parse_loose("mp4"), Container::Mp4);
        assert_eq!(Container::parse_loose("webm"), Container::Webm);
        assert_eq!(Container::parse_loose("matroska"), Container::Mkv);
        assert_eq!(Container::parse_loose("quicktime"), Container::Mov);
        assert_eq!(
            Container::parse_loose("matroska,webm"),
            Container::Unknown("matroska,webm".into())
        );
        assert_eq!(
            Container::parse_loose("mov,mp4,m4a,3gp,3g2,mj2"),
            Container::Unknown("mov,mp4,m4a,3gp,3g2,mj2".into())
        );
        assert_eq!(
            Container::parse_loose("isom"),
            Container::Unknown("isom".into())
        );
        assert_ne!(Container::parse_loose("matroska,webm").as_str(), "webm");
    }

    #[test]
    fn unknown_container_display_label_does_not_look_like_a_known_format() {
        assert_eq!(Container::Mp4.display_label(), "MP4");
        assert_eq!(Container::Mov.display_label(), "MOV");
        assert_eq!(
            Container::Unknown("matroska,webm".into()).display_label(),
            "unknown (matroska,webm)"
        );
    }
}
