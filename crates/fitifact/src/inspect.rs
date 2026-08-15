use std::path::Path;
use std::time::Duration;

use serde::Deserialize;

use crate::artifact::{
    Artifact, ArtifactSchema, AudioCodec, AudioStream, Completeness, Container, Family, HdrStatus,
    InspectionMeta, OtherStream, Rational, Stream, StreamDetails, VideoCodec, VideoStream,
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
            "-show_program_version".into(),
            "-protocol_whitelist".into(),
            "file".into(),
            path_str.into(),
        ];
        let output = self
            .spawner
            .spawn(&self.ffprobe_program, &args, self.timeout)?;
        if output.stdout_truncated {
            return Err(Error::new(
                ErrorCode::InspectionLimit,
                "ffprobe inspection output exceeded the 1 MiB safety limit",
            ));
        }
        if !output.success() {
            return Err(Error::new(
                ErrorCode::InputInvalid,
                "ffprobe could not parse the input as supported media",
            ));
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
    program_version: Option<ProbeProgramVersion>,
    error: Option<ProbeError>,
}

#[derive(Debug, Deserialize)]
struct ProbeProgramVersion {
    version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProbeError {
    string: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProbeStream {
    index: Option<u32>,
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    avg_frame_rate: Option<String>,
    r_frame_rate: Option<String>,
    pix_fmt: Option<String>,
    bits_per_raw_sample: Option<serde_json::Value>,
    color_range: Option<String>,
    color_space: Option<String>,
    color_transfer: Option<String>,
    color_primaries: Option<String>,
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

    let streams = streams.into_iter().map(normalize_stream).collect();

    let duration_ms = format.duration.as_deref().and_then(|d| {
        d.parse::<f64>()
            .ok()
            .map(|secs| (secs * 1000.0).round() as u64)
    });

    Ok(Artifact {
        schema: ArtifactSchema,
        path: Some(path.to_path_buf()),
        family: Family::Media,
        byte_length,
        container,
        streams,
        duration_ms,
        inspection: InspectionMeta {
            provider: "ffprobe".into(),
            provider_version: probe.program_version.and_then(|version| version.version),
            completeness: Completeness::Full,
            warnings: Vec::new(),
        },
    })
}

fn normalize_stream(stream: ProbeStream) -> Stream {
    let index = stream.index;
    let codec = stream.codec_name.clone();
    let details = match stream.codec_type.as_deref() {
        Some("video") => {
            let bit_depth = stream
                .bits_per_raw_sample
                .as_ref()
                .and_then(json_u32)
                .and_then(|value| u8::try_from(value).ok())
                .filter(|value| *value > 0)
                .or_else(|| {
                    stream
                        .pix_fmt
                        .as_deref()
                        .and_then(bit_depth_from_pixel_format)
                });
            let frame_rate = stream
                .avg_frame_rate
                .as_deref()
                .and_then(parse_rational)
                .or_else(|| stream.r_frame_rate.as_deref().and_then(parse_rational));
            let hdr = hdr_status(stream.color_transfer.as_deref());
            StreamDetails::Video {
                facts: VideoStream {
                    codec: stream.codec_name.as_deref().map(VideoCodec::parse_loose),
                    width: stream.width,
                    height: stream.height,
                    frame_rate,
                    pixel_format: stream.pix_fmt,
                    bit_depth,
                    color_range: stream.color_range,
                    color_space: stream.color_space,
                    color_transfer: stream.color_transfer,
                    color_primaries: stream.color_primaries,
                    hdr,
                },
            }
        }
        Some("audio") => StreamDetails::Audio {
            facts: AudioStream {
                codec: stream.codec_name.as_deref().map(AudioCodec::parse_loose),
                channels: stream.channels,
                sample_rate: stream.sample_rate.as_ref().and_then(json_u32),
            },
        },
        Some("subtitle") => StreamDetails::Subtitle {
            facts: OtherStream { codec },
        },
        Some("data") => StreamDetails::Data {
            facts: OtherStream { codec },
        },
        Some("attachment") => StreamDetails::Attachment {
            facts: OtherStream { codec },
        },
        other => StreamDetails::Unknown {
            original_type: other.map(str::to_string),
            facts: OtherStream { codec },
        },
    };
    Stream { index, details }
}

fn parse_rational(value: &str) -> Option<Rational> {
    let (numerator, denominator) = value.split_once('/')?;
    Rational::new(numerator.parse().ok()?, denominator.parse().ok()?)
}

fn bit_depth_from_pixel_format(value: &str) -> Option<u8> {
    let after_planar = value.rsplit_once('p').map(|(_, suffix)| suffix)?;
    let digits: String = after_planar
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    if digits.is_empty() {
        return Some(8);
    }
    digits.parse().ok()
}

fn hdr_status(transfer: Option<&str>) -> HdrStatus {
    match transfer.map(|value| value.to_ascii_lowercase()) {
        Some(value) if matches!(value.as_str(), "smpte2084" | "arib-std-b67" | "hlg" | "pq") => {
            HdrStatus::Hdr
        }
        Some(value)
            if matches!(
                value.as_str(),
                "bt709" | "bt601" | "smpte170m" | "gamma22" | "gamma28" | "iec61966-2-1"
            ) =>
        {
            HdrStatus::Sdr
        }
        _ => HdrStatus::Unknown,
    }
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
    use crate::runtime::SpawnOutput;

    struct ProbeSpawner {
        output: SpawnOutput,
        args: std::sync::Mutex<Vec<String>>,
    }

    impl ProcessSpawner for ProbeSpawner {
        fn spawn(
            &self,
            _program: &str,
            args: &[String],
            _timeout: Duration,
        ) -> Result<SpawnOutput> {
            *self.args.lock().unwrap() = args.to_vec();
            Ok(self.output.clone())
        }
    }

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
            artifact.first_video().and_then(|v| v.codec.clone()),
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

    #[test]
    fn ffprobe_stdout_overflow_is_a_stable_inspection_limit() {
        let path =
            std::env::temp_dir().join(format!("fitifact-probe-limit-{}.mp4", std::process::id()));
        std::fs::write(&path, b"x").unwrap();
        let spawner = ProbeSpawner {
            output: SpawnOutput {
                status: 0,
                stdout: br#"{"streams":[]}"#.to_vec(),
                stderr: Vec::new(),
                stdout_truncated: true,
                stderr_truncated: false,
            },
            args: std::sync::Mutex::new(Vec::new()),
        };
        let err = FfprobeInspector::new(&spawner).inspect(&path).unwrap_err();
        assert_eq!(err.code, ErrorCode::InspectionLimit);
        let args = spawner.args.lock().unwrap();
        assert!(
            args.windows(2)
                .any(|w| w == ["-protocol_whitelist", "file"])
        );
        std::fs::remove_file(path).unwrap();
    }
}
