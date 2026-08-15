use std::path::Path;
use std::time::Duration;

use serde::Deserialize;

use crate::artifact::{
    Artifact, AudioCodec, AudioStream, Completeness, Container, Family, InspectionMeta, VideoCodec,
    VideoStream,
};
use crate::error::{Error, ErrorCode, Result};
use crate::runtime::{ProcessSpawner, SystemSpawner};

pub trait Inspector {
    fn inspect(&self, path: &Path) -> Result<Artifact>;
}

#[derive(Debug, Clone)]
pub struct FfprobeInspector<S = SystemSpawner> {
    pub ffprobe_program: String,
    pub timeout: Duration,
    spawner: S,
}

impl Default for FfprobeInspector<SystemSpawner> {
    fn default() -> Self {
        Self::new(SystemSpawner)
    }
}

impl<S> FfprobeInspector<S> {
    pub fn new(spawner: S) -> Self {
        Self {
            ffprobe_program: "ffprobe".into(),
            timeout: Duration::from_secs(30),
            spawner,
        }
    }
}

impl<S: ProcessSpawner> Inspector for FfprobeInspector<S> {
    fn inspect(&self, path: &Path) -> Result<Artifact> {
        if !path.exists() {
            return Err(Error::new(
                ErrorCode::InputInvalid,
                format!("file not found: {}", path.display()),
            ));
        }
        let bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let path_str = path
            .to_str()
            .ok_or_else(|| Error::new(ErrorCode::InputInvalid, "path is not valid UTF-8"))?;
        let args = vec![
            "-v".into(),
            "error".into(),
            "-print_format".into(),
            "json".into(),
            "-show_format".into(),
            "-show_streams".into(),
            "-protocol_whitelist".into(),
            "file,crypto,data".into(),
            path_str.into(),
        ];
        let output = self
            .spawner
            .spawn(&self.ffprobe_program, &args, self.timeout)?;
        if !output.success() {
            let stderr = output.stderr_str();
            if stderr.to_ascii_lowercase().contains("invalid")
                || stderr.to_ascii_lowercase().contains("error")
                || output.status != 0
            {
                return Err(Error::new(
                    ErrorCode::InputInvalid,
                    format!(
                        "ffprobe could not read {}: {}",
                        path.display(),
                        stderr.trim()
                    ),
                ));
            }
        }
        let json = if output.stdout.is_empty() {
            output.stderr_str()
        } else {
            output.stdout_str()
        };
        artifact_from_ffprobe_json(path, bytes, &json)
    }
}

pub fn inspect(path: &Path, inspector: &dyn Inspector) -> Result<Artifact> {
    inspector.inspect(path)
}

#[derive(Debug, Deserialize)]
struct Probe {
    streams: Option<Vec<ProbeStream>>,
    format: Option<ProbeFormat>,
    error: Option<ProbeError>,
}

#[derive(Debug, Deserialize)]
struct ProbeError {
    string: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProbeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    channels: Option<u32>,
    sample_rate: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ProbeFormat {
    format_name: Option<String>,
    duration: Option<String>,
    tags: Option<ProbeTags>,
}

#[derive(Debug, Deserialize)]
struct ProbeTags {
    major_brand: Option<String>,
}

pub fn artifact_from_ffprobe_json(path: &Path, byte_length: u64, json: &str) -> Result<Artifact> {
    let probe: Probe = serde_json::from_str(json).map_err(|err| {
        Error::new(
            ErrorCode::InspectionUnsupported,
            format!("ffprobe json was not usable: {err}"),
        )
    })?;
    if let Some(error) = probe.error.and_then(|e| e.string) {
        return Err(Error::new(ErrorCode::InputInvalid, error));
    }
    let streams = probe.streams.unwrap_or_default();
    if streams.is_empty() {
        return Err(Error::new(
            ErrorCode::InspectionUnsupported,
            "no media streams found",
        ));
    }

    let format = probe.format.unwrap_or(ProbeFormat {
        format_name: None,
        duration: None,
        tags: None,
    });
    let major_brand = format.tags.as_ref().and_then(|t| t.major_brand.as_deref());
    let container = format
        .format_name
        .as_deref()
        .map(|name| Container::from_probe(name, major_brand));

    let video = streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("video"));
    let audio = streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("audio"));

    let video = video.map(|stream| VideoStream {
        codec: stream.codec_name.as_deref().map(VideoCodec::parse_loose),
        width: stream.width,
        height: stream.height,
        sample_rate: None,
    });
    let audio = audio.map(|stream| AudioStream {
        codec: stream.codec_name.as_deref().map(AudioCodec::parse_loose),
        channels: stream.channels,
        sample_rate: stream.sample_rate.as_ref().and_then(json_u32),
    });

    let duration_ms = format.duration.as_deref().and_then(|d| {
        d.parse::<f64>()
            .ok()
            .map(|secs| (secs * 1000.0).round() as u64)
    });

    Ok(Artifact {
        path: Some(path.to_path_buf()),
        family: Family::Media,
        byte_length,
        container,
        video,
        audio,
        duration_ms,
        inspection: InspectionMeta {
            provider: "ffprobe".into(),
            provider_version: None,
            completeness: Completeness::Full,
            warnings: Vec::new(),
        },
    })
}

fn json_u32(value: &serde_json::Value) -> Option<u32> {
    match value {
        serde_json::Value::Number(n) => n.as_u64().and_then(|n| u32::try_from(n).ok()),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::VideoCodec;

    const HEVC_MP4: &str = r#"{
        "streams": [
            {"codec_type": "video", "codec_name": "hevc", "width": 320, "height": 240},
            {"codec_type": "audio", "codec_name": "aac", "channels": 2, "sample_rate": "48000"}
        ],
        "format": {
            "format_name": "mov,mp4,m4a,3gp,3g2,mj2",
            "duration": "1.0",
            "tags": {"major_brand": "isom"}
        }
    }"#;

    #[test]
    fn inspection_uses_codec_not_extension() {
        let artifact = artifact_from_ffprobe_json(Path::new("video.mp4"), 1000, HEVC_MP4).unwrap();
        assert_eq!(artifact.container, Some(Container::Mp4));
        assert_eq!(
            artifact.video.as_ref().and_then(|v| v.codec.clone()),
            Some(VideoCodec::Hevc)
        );
        assert_eq!(artifact.family, Family::Media);
    }

    #[test]
    fn quicktime_brand_is_mov_even_when_format_name_lists_mp4() {
        let json = r#"{
            "streams": [{"codec_type": "video", "codec_name": "h264", "width": 16, "height": 16}],
            "format": {
                "format_name": "mov,mp4,m4a,3gp,3g2,mj2",
                "tags": {"major_brand": "qt  "}
            }
        }"#;
        let artifact = artifact_from_ffprobe_json(Path::new("clip.mov"), 100, json).unwrap();
        assert_eq!(artifact.container, Some(Container::Mov));
    }

    #[test]
    fn empty_streams_are_unsupported() {
        let err =
            artifact_from_ffprobe_json(Path::new("x.mp4"), 1, r#"{"streams":[]}"#).unwrap_err();
        assert_eq!(err.code, ErrorCode::InspectionUnsupported);
    }
}
