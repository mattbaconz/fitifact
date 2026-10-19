//! Media facts measured in the browser (Mediabunny in a web worker), mapped onto the facts
//! ffprobe reports so browser and native inspection share one normalization path.
//!
//! The worker supplies the container track census, Mediabunny's per-track facts, and the raw
//! H.264 decoder configuration. Pixel format, bit depth, and color come from the SPS here, the
//! way FFmpeg's decoder derives them, so profile picture rules judge both surfaces alike.

use std::path::Path;

use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::artifact::Artifact;
use crate::error::{Error, ErrorCode, Result};
use crate::inspect::artifact_from_ffprobe_json;

pub const BROWSER_PROBE_SCHEMA: &str = "fitifact.browser-probe/v1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BrowserProbe {
    schema: String,
    provider_version: Option<String>,
    name: String,
    byte_length: u64,
    container: String,
    major_brand: Option<String>,
    duration_ms: Option<u64>,
    tracks: Vec<BrowserTrack>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum BrowserTrack {
    Video {
        codec: Option<String>,
        width: Option<u32>,
        height: Option<u32>,
        /// AVCDecoderConfigurationRecord bytes for H.264.
        avcc: Option<Vec<u8>>,
        /// Container-level color, in WebCodecs names.
        color: Option<BrowserColor>,
    },
    Audio {
        codec: Option<String>,
        channels: Option<u32>,
        sample_rate: Option<u32>,
    },
    Subtitle,
    Data,
    Attachment,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct BrowserColor {
    primaries: Option<String>,
    transfer: Option<String>,
    matrix: Option<String>,
    full_range: Option<bool>,
}

pub fn artifact_from_browser_probe(json: &str) -> Result<Artifact> {
    let probe: BrowserProbe = serde_json::from_str(json).map_err(|err| {
        Error::new(
            ErrorCode::InputInvalid,
            format!("the browser probe was not usable: {err}"),
        )
    })?;
    if probe.schema != BROWSER_PROBE_SCHEMA {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            format!("expected a {BROWSER_PROBE_SCHEMA} probe"),
        ));
    }
    let format_name = match probe.container.as_str() {
        "isobmff" => "mov,mp4,m4a,3gp,3g2,mj2",
        "matroska" => "matroska,webm",
        other => {
            return Err(Error::new(
                ErrorCode::InspectionUnsupported,
                format!("the browser cannot inspect {other} containers"),
            ));
        }
    };
    let streams: Vec<Value> = probe
        .tracks
        .iter()
        .enumerate()
        .map(|(index, track)| ffprobe_stream(index, track))
        .collect();
    let mut format = json!({ "format_name": format_name });
    if let Some(ms) = probe.duration_ms {
        format["duration"] = json!(format!("{}.{:03}", ms / 1000, ms % 1000));
    }
    if let Some(brand) = &probe.major_brand {
        format["tags"] = json!({ "major_brand": brand });
    }
    let ffprobe = json!({ "streams": streams, "format": format }).to_string();
    let mut artifact =
        artifact_from_ffprobe_json(Path::new(&probe.name), probe.byte_length, &ffprobe)?;
    artifact.inspection.provider = "mediabunny".into();
    artifact.inspection.provider_version = probe.provider_version;
    Ok(artifact)
}

fn ffprobe_stream(index: usize, track: &BrowserTrack) -> Value {
    let mut stream = Map::new();
    stream.insert("index".into(), json!(index));
    let mut put = |key: &str, value: Option<Value>| {
        if let Some(value) = value {
            stream.insert(key.into(), value);
        }
    };
    match track {
        BrowserTrack::Video {
            codec,
            width,
            height,
            avcc,
            color,
        } => {
            put("codec_type", Some(json!("video")));
            put("codec_name", codec.as_ref().map(|codec| json!(codec)));
            put("width", width.map(|value| json!(value)));
            put("height", height.map(|value| json!(value)));
            let picture = Picture::new(
                avcc.as_deref().and_then(first_sps).and_then(parse_sps),
                color.as_ref().unwrap_or(&BrowserColor::default()),
            );
            put("pix_fmt", picture.pixel_format.map(Value::from));
            put("bits_per_raw_sample", picture.bit_depth.map(Value::from));
            put("color_range", picture.range.map(Value::from));
            put("color_space", picture.matrix.map(Value::from));
            put("color_transfer", picture.transfer.map(Value::from));
            put("color_primaries", picture.primaries.map(Value::from));
        }
        BrowserTrack::Audio {
            codec,
            channels,
            sample_rate,
        } => {
            put("codec_type", Some(json!("audio")));
            put("codec_name", codec.as_ref().map(|codec| json!(codec)));
            put("channels", channels.map(|value| json!(value)));
            put("sample_rate", sample_rate.map(|value| json!(value)));
        }
        BrowserTrack::Subtitle => put("codec_type", Some(json!("subtitle"))),
        BrowserTrack::Data => put("codec_type", Some(json!("data"))),
        BrowserTrack::Attachment => put("codec_type", Some(json!("attachment"))),
    }
    Value::Object(stream)
}

/// Picture facts with FFmpeg's precedence: the SPS VUI wins over the container when present.
struct Picture {
    pixel_format: Option<String>,
    bit_depth: Option<String>,
    range: Option<&'static str>,
    matrix: Option<&'static str>,
    transfer: Option<&'static str>,
    primaries: Option<&'static str>,
}

impl Picture {
    fn new(sps: Option<Sps>, container: &BrowserColor) -> Self {
        let range = match sps.as_ref().and_then(|sps| sps.full_range) {
            Some(full) => Some(full),
            None => container.full_range,
        }
        .map(|full| if full { "pc" } else { "tv" });
        let (primaries, transfer, matrix) = match sps.as_ref().and_then(|sps| sps.colour) {
            Some((primaries, transfer, matrix)) => (
                primaries_name(primaries),
                transfer_name(transfer),
                matrix_name(matrix),
            ),
            None => (
                container.primaries.as_deref().and_then(web_primaries),
                container.transfer.as_deref().and_then(web_transfer),
                container.matrix.as_deref().and_then(web_matrix),
            ),
        };
        let pixel_format = sps.as_ref().map(|sps| {
            pixel_format_name(
                sps.chroma_format_idc,
                sps.bit_depth,
                range == Some("pc"),
                matrix == Some("gbr"),
            )
        });
        Self {
            pixel_format,
            bit_depth: sps.map(|sps| sps.bit_depth.to_string()),
            range,
            matrix,
            transfer,
            primaries,
        }
    }
}

/// FFmpeg's H.264 decoder output format: 4:0:0 decodes as 4:2:0, 8-bit full range uses the
/// `yuvj` names, and 4:4:4 with an identity matrix is planar RGB.
fn pixel_format_name(
    chroma_format_idc: u32,
    bit_depth: u32,
    full_range: bool,
    rgb: bool,
) -> String {
    let layout = match chroma_format_idc {
        2 => "422",
        3 => "444",
        _ => "420",
    };
    let base = if chroma_format_idc == 3 && rgb {
        "gbrp".to_string()
    } else if bit_depth == 8 && full_range {
        format!("yuvj{layout}p")
    } else {
        format!("yuv{layout}p")
    };
    if bit_depth == 8 {
        base
    } else {
        format!("{base}{bit_depth}le")
    }
}

fn primaries_name(code: u8) -> Option<&'static str> {
    Some(match code {
        1 => "bt709",
        4 => "bt470m",
        5 => "bt470bg",
        6 => "smpte170m",
        7 => "smpte240m",
        8 => "film",
        9 => "bt2020",
        10 => "smpte428",
        11 => "smpte431",
        12 => "smpte432",
        22 => "ebu3213",
        _ => return None,
    })
}

fn transfer_name(code: u8) -> Option<&'static str> {
    Some(match code {
        1 => "bt709",
        4 => "gamma22",
        5 => "gamma28",
        6 => "smpte170m",
        7 => "smpte240m",
        8 => "linear",
        9 => "log100",
        10 => "log316",
        11 => "iec61966-2-4",
        12 => "bt1361e",
        13 => "iec61966-2-1",
        14 => "bt2020-10",
        15 => "bt2020-12",
        16 => "smpte2084",
        17 => "smpte428",
        18 => "arib-std-b67",
        _ => return None,
    })
}

fn matrix_name(code: u8) -> Option<&'static str> {
    Some(match code {
        0 => "gbr",
        1 => "bt709",
        4 => "fcc",
        5 => "bt470bg",
        6 => "smpte170m",
        7 => "smpte240m",
        8 => "ycgco",
        9 => "bt2020nc",
        10 => "bt2020c",
        11 => "smpte2085",
        12 => "chroma-derived-nc",
        13 => "chroma-derived-c",
        14 => "ictcp",
        _ => return None,
    })
}

fn web_primaries(name: &str) -> Option<&'static str> {
    Some(match name {
        "bt709" => "bt709",
        "bt470bg" => "bt470bg",
        "smpte170m" => "smpte170m",
        "bt2020" => "bt2020",
        "smpte432" => "smpte432",
        _ => return None,
    })
}

fn web_transfer(name: &str) -> Option<&'static str> {
    Some(match name {
        "bt709" => "bt709",
        "smpte170m" => "smpte170m",
        "linear" => "linear",
        "iec61966-2-1" => "iec61966-2-1",
        "pq" => "smpte2084",
        "hlg" => "arib-std-b67",
        _ => return None,
    })
}

fn web_matrix(name: &str) -> Option<&'static str> {
    Some(match name {
        "rgb" => "gbr",
        "bt709" => "bt709",
        "bt470bg" => "bt470bg",
        "smpte170m" => "smpte170m",
        "bt2020-ncl" => "bt2020nc",
        _ => return None,
    })
}

/// The first SPS NAL unit of an AVCDecoderConfigurationRecord (ISO/IEC 14496-15, 5.3.3.1).
fn first_sps(avcc: &[u8]) -> Option<&[u8]> {
    if *avcc.first()? != 1 || avcc.get(5)? & 0x1f == 0 {
        return None;
    }
    let length = usize::from(u16::from_be_bytes([*avcc.get(6)?, *avcc.get(7)?]));
    avcc.get(8..8 + length)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Sps {
    chroma_format_idc: u32,
    bit_depth: u32,
    full_range: Option<bool>,
    /// (colour_primaries, transfer_characteristics, matrix_coefficients) when described.
    colour: Option<(u8, u8, u8)>,
}

/// Reads an H.264 SPS (ITU-T H.264, 7.3.2.1.1) up to the VUI colour description.
fn parse_sps(nal: &[u8]) -> Option<Sps> {
    let rbsp = strip_emulation_prevention(nal);
    let mut bits = Bits::new(&rbsp);
    if bits.read(8)? & 0x1f != 7 {
        return None;
    }
    let profile_idc = bits.read(8)?;
    bits.read(16)?; // constraint flags, level_idc
    bits.ue()?; // seq_parameter_set_id
    let mut sps = Sps {
        chroma_format_idc: 1,
        bit_depth: 8,
        full_range: None,
        colour: None,
    };
    if matches!(
        profile_idc,
        100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 | 138 | 139 | 134 | 135
    ) {
        sps.chroma_format_idc = bits.ue()?;
        if sps.chroma_format_idc == 3 {
            bits.read(1)?; // separate_colour_plane_flag
        }
        sps.bit_depth = bits.ue()?.checked_add(8)?;
        bits.ue()?; // bit_depth_chroma_minus8
        bits.read(1)?; // qpprime_y_zero_transform_bypass_flag
        if bits.read(1)? == 1 {
            let lists = if sps.chroma_format_idc == 3 { 12 } else { 8 };
            for list in 0..lists {
                if bits.read(1)? == 1 {
                    skip_scaling_list(&mut bits, if list < 6 { 16 } else { 64 })?;
                }
            }
        }
    }
    bits.ue()?; // log2_max_frame_num_minus4
    match bits.ue()? {
        0 => {
            bits.ue()?; // log2_max_pic_order_cnt_lsb_minus4
        }
        1 => {
            bits.read(1)?; // delta_pic_order_always_zero_flag
            bits.se()?; // offset_for_non_ref_pic
            bits.se()?; // offset_for_top_to_bottom_field
            for _ in 0..bits.ue()? {
                bits.se()?; // offset_for_ref_frame
            }
        }
        _ => {}
    }
    bits.ue()?; // max_num_ref_frames
    bits.read(1)?; // gaps_in_frame_num_value_allowed_flag
    bits.ue()?; // pic_width_in_mbs_minus1
    bits.ue()?; // pic_height_in_map_units_minus1
    if bits.read(1)? == 0 {
        bits.read(1)?; // mb_adaptive_frame_field_flag
    }
    bits.read(1)?; // direct_8x8_inference_flag
    if bits.read(1)? == 1 {
        for _ in 0..4 {
            bits.ue()?; // frame_crop_*_offset
        }
    }
    if bits.read(1)? == 0 {
        return Some(sps); // no VUI
    }
    if bits.read(1)? == 1 && bits.read(8)? == 255 {
        bits.read(32)?; // Extended_SAR sar_width, sar_height
    }
    if bits.read(1)? == 1 {
        bits.read(1)?; // overscan_appropriate_flag
    }
    if bits.read(1)? == 1 {
        bits.read(3)?; // video_format
        sps.full_range = Some(bits.read(1)? == 1);
        if bits.read(1)? == 1 {
            let primaries = bits.read(8)? as u8;
            let transfer = bits.read(8)? as u8;
            let matrix = bits.read(8)? as u8;
            sps.colour = Some((primaries, transfer, matrix));
        }
    }
    Some(sps)
}

fn skip_scaling_list(bits: &mut Bits<'_>, size: usize) -> Option<()> {
    let (mut last, mut next) = (8_i64, 8_i64);
    for _ in 0..size {
        if next != 0 {
            next = (last + bits.se()? + 256).rem_euclid(256);
        }
        if next != 0 {
            last = next;
        }
    }
    Some(())
}

fn strip_emulation_prevention(nal: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(nal.len());
    let mut zeros = 0;
    for &byte in nal {
        if zeros >= 2 && byte == 3 {
            zeros = 0;
            continue;
        }
        zeros = if byte == 0 { zeros + 1 } else { 0 };
        out.push(byte);
    }
    out
}

struct Bits<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Bits<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn bit(&mut self) -> Option<u32> {
        let byte = *self.bytes.get(self.position / 8)?;
        let bit = (byte >> (7 - self.position % 8)) & 1;
        self.position += 1;
        Some(u32::from(bit))
    }

    fn read(&mut self, count: u32) -> Option<u32> {
        let mut value = 0_u32;
        for _ in 0..count {
            value = (value << 1) | self.bit()?;
        }
        Some(value)
    }

    /// Unsigned Exp-Golomb; values above 32 bits are malformed for every field read here.
    fn ue(&mut self) -> Option<u32> {
        let mut zeros = 0;
        while self.bit()? == 0 {
            zeros += 1;
            if zeros > 31 {
                return None;
            }
        }
        let suffix = self.read(zeros)?;
        ((1_u64 << zeros) - 1 + u64::from(suffix)).try_into().ok()
    }

    fn se(&mut self) -> Option<i64> {
        let code = i64::from(self.ue()?);
        Some(if code % 2 == 1 {
            (code + 1) / 2
        } else {
            -(code / 2)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{Container, HdrStatus, StreamType, VideoCodec};

    /// Writes H.264 syntax elements so tests can build SPS variants.
    #[derive(Default)]
    struct Writer {
        bits: Vec<u8>,
    }

    impl Writer {
        fn u(&mut self, count: u32, value: u32) -> &mut Self {
            for shift in (0..count).rev() {
                self.bits.push(((value >> shift) & 1) as u8);
            }
            self
        }

        fn ue(&mut self, value: u32) -> &mut Self {
            let code = u64::from(value) + 1;
            let length = 64 - code.leading_zeros();
            self.u(length - 1, 0);
            for shift in (0..length).rev() {
                self.bits.push(((code >> shift) & 1) as u8);
            }
            self
        }

        fn bytes(&self) -> Vec<u8> {
            let mut bits = self.bits.clone();
            bits.push(1); // rbsp_stop_one_bit
            bits.resize(bits.len().div_ceil(8) * 8, 0);
            bits.chunks(8)
                .map(|chunk| chunk.iter().fold(0, |byte, bit| (byte << 1) | bit))
                .collect()
        }
    }

    /// A 64x64 progressive SPS with optional high-profile chroma/depth and VUI signal type.
    fn sps(
        profile: u32,
        high: Option<(u32, u32)>,
        vui: Option<(bool, Option<[u32; 3]>)>,
    ) -> Vec<u8> {
        let mut w = Writer::default();
        w.u(8, 0x67).u(8, profile).u(8, 0).u(8, 40).ue(0);
        if let Some((chroma, depth)) = high {
            w.ue(chroma);
            if chroma == 3 {
                w.u(1, 0);
            }
            w.ue(depth - 8).ue(depth - 8).u(1, 0).u(1, 0);
        }
        w.ue(0)
            .ue(0)
            .ue(0)
            .ue(1)
            .u(1, 0)
            .ue(3)
            .ue(3)
            .u(1, 1)
            .u(1, 1)
            .u(1, 0);
        match vui {
            None => {
                w.u(1, 0);
            }
            Some((full, colour)) => {
                w.u(1, 1)
                    .u(1, 0)
                    .u(1, 0)
                    .u(1, 1)
                    .u(3, 5)
                    .u(1, u32::from(full));
                match colour {
                    Some([primaries, transfer, matrix]) => {
                        w.u(1, 1).u(8, primaries).u(8, transfer).u(8, matrix);
                    }
                    None => {
                        w.u(1, 0);
                    }
                }
                w.u(1, 0).u(1, 0).u(1, 0).u(1, 0).u(1, 0).u(1, 0);
            }
        }
        w.bytes()
    }

    fn avcc(sps: &[u8]) -> Vec<u8> {
        let mut record = vec![1, sps[1], 0, sps[3], 0xff, 0xe1];
        record.extend_from_slice(&u16::try_from(sps.len()).unwrap().to_be_bytes());
        record.extend_from_slice(sps);
        record.extend_from_slice(&[1, 0, 2, 0x68, 0xce]);
        record
    }

    /// The avcC payload of a real fixture, found by its box type.
    fn fixture_avcc(name: &str) -> Vec<u8> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/media")
            .join(name);
        let bytes = std::fs::read(path).unwrap();
        let at = bytes.windows(4).position(|w| w == b"avcC").unwrap();
        let size = u32::from_be_bytes(bytes[at - 4..at].try_into().unwrap()) as usize;
        bytes[at + 4..at - 4 + size].to_vec()
    }

    fn probe(container: &str, brand: Option<&str>, tracks: Value) -> String {
        json!({
            "schema": BROWSER_PROBE_SCHEMA,
            "provider_version": "1.61.1",
            "name": "clip",
            "byte_length": 1000,
            "container": container,
            "major_brand": brand,
            "duration_ms": 2000,
            "tracks": tracks,
        })
        .to_string()
    }

    fn video(avcc: &[u8], color: Value) -> Value {
        json!({ "type": "video", "codec": "avc", "width": 64, "height": 64, "avcc": avcc, "color": color })
    }

    #[test]
    fn libx264_fixtures_read_as_8_bit_420() {
        for name in ["compatible-h264-aac.mp4", "remux-h264-aac.mov"] {
            let sps = parse_sps(first_sps(&fixture_avcc(name)).unwrap()).unwrap();
            assert_eq!((sps.chroma_format_idc, sps.bit_depth), (1, 8), "{name}");
        }
    }

    #[test]
    fn mov_probe_matches_ffprobe_normalization() {
        let json = probe(
            "isobmff",
            Some("qt  "),
            json!([
                video(&fixture_avcc("remux-h264-aac.mov"), json!({})),
                { "type": "audio", "codec": "aac", "channels": 2, "sample_rate": 48000 }
            ]),
        );
        let artifact = artifact_from_browser_probe(&json).unwrap();
        assert_eq!(artifact.container, Some(Container::Mov));
        assert_eq!(artifact.duration_ms, Some(2000));
        assert_eq!(artifact.inspection.provider, "mediabunny");
        let video = artifact.first_video().unwrap();
        assert_eq!(video.codec, Some(VideoCodec::H264));
        assert_eq!(video.pixel_format.as_deref(), Some("yuv420p"));
        assert_eq!(video.bit_depth, Some(8));
        assert_eq!(artifact.first_audio().unwrap().sample_rate, Some(48000));
    }

    #[test]
    fn high_profiles_report_chroma_and_depth_like_ffmpeg() {
        let cases = [
            (Some((1, 10)), "yuv420p10le", 10),
            (Some((2, 8)), "yuv422p", 8),
            (Some((3, 8)), "yuv444p", 8),
            (Some((0, 8)), "yuv420p", 8),
            (None, "yuv420p", 8),
        ];
        for (high, pixel_format, depth) in cases {
            let profile = if high.is_some() { 100 } else { 66 };
            let json = probe(
                "isobmff",
                Some("isom"),
                json!([video(&avcc(&sps(profile, high, None)), json!({}))]),
            );
            let artifact = artifact_from_browser_probe(&json).unwrap();
            let facts = artifact.first_video().unwrap();
            assert_eq!(
                facts.pixel_format.as_deref(),
                Some(pixel_format),
                "{high:?}"
            );
            assert_eq!(facts.bit_depth, Some(depth), "{high:?}");
        }
    }

    #[test]
    fn vui_signal_type_wins_over_container_color() {
        let pq = sps(100, Some((1, 10)), Some((false, Some([9, 16, 9]))));
        let json = probe(
            "isobmff",
            Some("isom"),
            json!([video(
                &avcc(&pq),
                json!({ "primaries": "bt709", "transfer": "bt709", "matrix": "bt709", "full_range": true })
            )]),
        );
        let facts = artifact_from_browser_probe(&json)
            .unwrap()
            .first_video()
            .unwrap()
            .clone();
        assert_eq!(facts.color_range.as_deref(), Some("tv"));
        assert_eq!(facts.color_primaries.as_deref(), Some("bt2020"));
        assert_eq!(facts.color_transfer.as_deref(), Some("smpte2084"));
        assert_eq!(facts.color_space.as_deref(), Some("bt2020nc"));
        assert_eq!(facts.hdr, HdrStatus::Hdr);

        let full = sps(66, None, Some((true, None)));
        let json = probe(
            "isobmff",
            Some("isom"),
            json!([video(
                &avcc(&full),
                json!({ "primaries": "bt709", "transfer": "hlg", "matrix": "bt709" })
            )]),
        );
        let facts = artifact_from_browser_probe(&json)
            .unwrap()
            .first_video()
            .unwrap()
            .clone();
        assert_eq!(facts.pixel_format.as_deref(), Some("yuvj420p"));
        assert_eq!(facts.color_range.as_deref(), Some("pc"));
        assert_eq!(facts.color_transfer.as_deref(), Some("arib-std-b67"));
        assert_eq!(facts.hdr, HdrStatus::Hdr);
    }

    #[test]
    fn missing_or_damaged_decoder_config_leaves_picture_unknown() {
        let damaged = sps(100, Some((1, 8)), None)[..4].to_vec();
        for avcc in [
            Vec::new(),
            vec![1, 100, 0, 40, 0xff, 0xe0],
            self::avcc(&damaged),
        ] {
            let json = probe("isobmff", Some("isom"), json!([video(&avcc, json!({}))]));
            let facts = artifact_from_browser_probe(&json)
                .unwrap()
                .first_video()
                .unwrap()
                .clone();
            assert_eq!(facts.pixel_format, None);
            assert_eq!(facts.bit_depth, None);
        }
    }

    #[test]
    fn matroska_with_h264_is_mkv_and_census_tracks_stay_visible() {
        let json = probe(
            "matroska",
            None,
            json!([
                video(&fixture_avcc("remux-h264-aac.mov"), json!({})),
                { "type": "audio", "codec": "aac", "channels": 2, "sample_rate": 48000 },
                { "type": "subtitle" },
                { "type": "attachment" }
            ]),
        );
        let artifact = artifact_from_browser_probe(&json).unwrap();
        assert_eq!(artifact.container, Some(Container::Mkv));
        let types: Vec<StreamType> = artifact.streams.iter().map(|s| s.stream_type()).collect();
        assert_eq!(
            types,
            [
                StreamType::Video,
                StreamType::Audio,
                StreamType::Subtitle,
                StreamType::Attachment
            ]
        );
    }

    #[test]
    fn foreign_schemas_and_containers_are_refused() {
        let wrong = probe("isobmff", None, json!([])).replace(BROWSER_PROBE_SCHEMA, "other/v1");
        assert_eq!(
            artifact_from_browser_probe(&wrong).unwrap_err().code,
            ErrorCode::InputInvalid
        );
        let ogg = probe("ogg", None, json!([{ "type": "audio", "codec": "opus" }]));
        assert_eq!(
            artifact_from_browser_probe(&ogg).unwrap_err().code,
            ErrorCode::InspectionUnsupported
        );
        let empty = probe("isobmff", None, json!([]));
        assert_eq!(
            artifact_from_browser_probe(&empty).unwrap_err().code,
            ErrorCode::InspectionUnsupported
        );
        // A fact the engine doesn't model is refused, not silently dropped.
        let extra = probe(
            "isobmff",
            None,
            json!([{ "type": "video", "codec": "avc", "rotation": 90 }]),
        );
        assert_eq!(
            artifact_from_browser_probe(&extra).unwrap_err().code,
            ErrorCode::InputInvalid
        );
        let extra_top = probe("isobmff", None, json!([{ "type": "data" }])).replacen(
            '{',
            "{\"rotation\":90,",
            1,
        );
        assert_eq!(
            artifact_from_browser_probe(&extra_top).unwrap_err().code,
            ErrorCode::InputInvalid
        );
    }

    #[test]
    fn emulation_prevention_bytes_are_removed() {
        assert_eq!(
            strip_emulation_prevention(&[0, 0, 3, 1, 0, 0, 3, 0, 3]),
            [0, 0, 1, 0, 0, 0, 3]
        );
    }
}
