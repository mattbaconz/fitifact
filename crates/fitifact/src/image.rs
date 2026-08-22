use std::io::Cursor;
use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::metadata::Orientation;
use image::{ColorType, DynamicImage, ImageDecoder, ImageReader};

use crate::artifact::{
    Artifact, ArtifactSchema, Completeness, Family, ImageFacts, ImageFormat, InspectionMeta,
};
use crate::error::{Error, ErrorCode, Result};
use crate::plan::{ExpectedFact, ExpectedValue, Plan};
use crate::runtime::{ExecutionContext, StreamHashes, TransformProvider};

const PNG_SIG: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
pub const MAX_IMAGE_INPUT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_IMAGE_PIXELS: u64 = 24_000_000;

pub fn looks_like_image(bytes: &[u8]) -> bool {
    sniff_format(bytes).is_some()
}

pub fn sniff_format(bytes: &[u8]) -> Option<ImageFormat> {
    if bytes.starts_with(PNG_SIG) {
        return Some(ImageFormat::Png);
    }
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return Some(ImageFormat::Jpeg);
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some(ImageFormat::Webp);
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some(ImageFormat::Gif);
    }
    if bytes.len() >= 4 && (bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*")) {
        return Some(ImageFormat::Tiff);
    }
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if matches!(
            brand,
            b"heic" | b"heix" | b"hevc" | b"hevx" | b"mif1" | b"msf1" | b"heif"
        ) {
            return Some(ImageFormat::Heif);
        }
    }
    None
}

pub fn artifact_from_bytes(path: Option<&Path>, bytes: &[u8]) -> Result<Artifact> {
    enforce_encoded_limit(bytes.len())?;
    let Some(format) = sniff_format(bytes) else {
        return Err(Error::new(
            ErrorCode::InspectionUnsupported,
            "the input is not a recognized image",
        ));
    };
    if format == ImageFormat::Jpeg && jpeg_is_multi_image(bytes) {
        return Err(Error::new(
            ErrorCode::InspectionUnsupported,
            "image.multi_image_unsupported: multi-image JPEG/MPO inputs are unsupported",
        ));
    }
    let animated = match format {
        ImageFormat::Png => Some(png_is_animated(bytes)),
        ImageFormat::Gif => Some(true),
        ImageFormat::Webp => Some(webp_is_animated(bytes)),
        _ => Some(false),
    };
    let decoded = match format {
        ImageFormat::Jpeg | ImageFormat::Png => decode_still(bytes)?,
        ImageFormat::Webp if animated != Some(true) => decode_still(bytes)?,
        _ => None,
    };
    Ok(Artifact {
        schema: ArtifactSchema,
        path: path.map(Path::to_path_buf),
        family: Family::Image,
        byte_length: bytes.len() as u64,
        container: None,
        streams: Vec::new(),
        duration_ms: None,
        image: Some(ImageFacts {
            format: Some(format),
            width: decoded.as_ref().map(|frame| frame.width),
            height: decoded.as_ref().map(|frame| frame.height),
            alpha: decoded.as_ref().map(|frame| frame.alpha),
            animated,
        }),
        inspection: InspectionMeta {
            provider: "fitifact-image".into(),
            provider_version: Some(env!("CARGO_PKG_VERSION").into()),
            completeness: if decoded.is_some() {
                Completeness::Full
            } else {
                Completeness::Partial
            },
            warnings: Vec::new(),
        },
    })
}

struct DecodedStill {
    width: u32,
    height: u32,
    alpha: bool,
}

fn decode_still(bytes: &[u8]) -> Result<Option<DecodedStill>> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| {
            Error::new(
                ErrorCode::InputInvalid,
                "the image header could not be parsed",
            )
        })?;
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| Error::new(ErrorCode::InputInvalid, "the image could not be decoded"))?;
    let (mut width, mut height) = decoder.dimensions();
    enforce_decoded_limit(width, height)?;
    let orientation = decoder.orientation().map_err(|_| {
        Error::new(
            ErrorCode::InputInvalid,
            "the image orientation metadata could not be read",
        )
    })?;
    if matches!(
        orientation,
        Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH
    ) {
        std::mem::swap(&mut width, &mut height);
    }
    let alpha = matches!(
        decoder.color_type(),
        ColorType::Rgba8 | ColorType::Rgba16 | ColorType::La8 | ColorType::La16
    );
    Ok(Some(DecodedStill {
        width,
        height,
        alpha,
    }))
}

pub fn enforce_encoded_limit(length: usize) -> Result<()> {
    if length > MAX_IMAGE_INPUT_BYTES {
        return Err(Error::new(
            ErrorCode::InspectionLimit,
            "image.input_too_large: encoded image exceeds the 32 MiB limit",
        ));
    }
    Ok(())
}

pub fn enforce_decoded_limit(width: u32, height: u32) -> Result<()> {
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if pixels > MAX_IMAGE_PIXELS {
        return Err(Error::new(
            ErrorCode::InspectionLimit,
            "image.decoded_too_large: decoded image exceeds the 24-megapixel limit",
        ));
    }
    Ok(())
}

pub(crate) fn decode_oriented(bytes: &[u8]) -> Result<DynamicImage> {
    enforce_encoded_limit(bytes.len())?;
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| {
            Error::new(
                ErrorCode::InputInvalid,
                "the image header could not be parsed",
            )
        })?;
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| Error::new(ErrorCode::InputInvalid, "the image could not be decoded"))?;
    let (width, height) = decoder.dimensions();
    enforce_decoded_limit(width, height)?;
    let orientation = decoder.orientation().map_err(|_| {
        Error::new(
            ErrorCode::InputInvalid,
            "the image orientation metadata could not be read",
        )
    })?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(|_| {
        Error::new(
            ErrorCode::ExecutionFailed,
            "the image source could not be decoded",
        )
    })?;
    image.apply_orientation(orientation);
    Ok(image)
}

fn png_is_animated(bytes: &[u8]) -> bool {
    png_chunks(bytes).any(|kind| kind == b"acTL")
}

fn webp_is_animated(bytes: &[u8]) -> bool {
    bytes.windows(4).any(|window| window == b"ANIM")
}

pub fn jpeg_is_multi_image(bytes: &[u8]) -> bool {
    jpeg_segments(bytes).any(|(marker, payload)| marker == 0xe2 && payload.starts_with(b"MPF\0"))
}

pub(crate) fn contains_image_metadata(bytes: &[u8], format: &ImageFormat) -> bool {
    match format {
        ImageFormat::Jpeg => {
            jpeg_segments(bytes).any(|(marker, _)| matches!(marker, 0xe1 | 0xe2 | 0xed | 0xfe))
        }
        ImageFormat::Png => png_chunks(bytes)
            .any(|kind| matches!(kind, b"eXIf" | b"iCCP" | b"iTXt" | b"tEXt" | b"zTXt")),
        _ => false,
    }
}

fn jpeg_segments(bytes: &[u8]) -> impl Iterator<Item = (u8, &[u8])> {
    let mut segments = Vec::new();
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return segments.into_iter();
    }
    let mut cursor = 2_usize;
    while cursor + 4 <= bytes.len() {
        if bytes[cursor] != 0xff {
            cursor += 1;
            continue;
        }
        while cursor < bytes.len() && bytes[cursor] == 0xff {
            cursor += 1;
        }
        if cursor >= bytes.len() {
            break;
        }
        let marker = bytes[cursor];
        cursor += 1;
        if matches!(marker, 0xd9 | 0xda) {
            break;
        }
        if matches!(marker, 0x01 | 0xd0..=0xd7) {
            continue;
        }
        if cursor + 2 > bytes.len() {
            break;
        }
        let length = usize::from(u16::from_be_bytes([bytes[cursor], bytes[cursor + 1]]));
        if length < 2 || cursor + length > bytes.len() {
            break;
        }
        segments.push((marker, &bytes[cursor + 2..cursor + length]));
        cursor += length;
    }
    segments.into_iter()
}

fn png_chunks(bytes: &[u8]) -> impl Iterator<Item = &[u8; 4]> {
    let mut chunks = Vec::new();
    let mut cursor = PNG_SIG.len();
    while bytes.starts_with(PNG_SIG) && cursor + 12 <= bytes.len() {
        let length = u32::from_be_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]) as usize;
        let Some(end) = cursor
            .checked_add(12)
            .and_then(|value| value.checked_add(length))
        else {
            break;
        };
        if end > bytes.len() {
            break;
        }
        let kind: &[u8; 4] = bytes[cursor + 4..cursor + 8]
            .try_into()
            .expect("four-byte PNG chunk type");
        chunks.push(kind);
        cursor = end;
    }
    chunks.into_iter()
}

pub fn encode_jpeg_bytes(bytes: &[u8]) -> Result<Vec<u8>> {
    enforce_encoded_limit(bytes.len())?;
    let format = sniff_format(bytes).ok_or_else(|| {
        Error::new(
            ErrorCode::InputInvalid,
            "the input is not a recognized image",
        )
    })?;
    if format != ImageFormat::Png {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            "v0.1 encodes only PNG sources to JPEG",
        ));
    }
    if png_is_animated(bytes) {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            "v0.1 refuses animated PNG",
        ));
    }
    let artifact = artifact_from_bytes(None, bytes)?;
    if artifact
        .image
        .as_ref()
        .and_then(|facts| facts.alpha)
        .unwrap_or(false)
    {
        return Err(Error::new(
            ErrorCode::NoValidPlan,
            "image.transparency_flattening_refused: PNG alpha cannot be flattened implicitly",
        ));
    }
    let image = decode_oriented(bytes)?;
    encode_dynamic_jpeg(&image)
}

fn encode_dynamic_jpeg(image: &DynamicImage) -> Result<Vec<u8>> {
    let rgb = image.to_rgb8();
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, 90)
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            ColorType::Rgb8.into(),
        )
        .map_err(|_| Error::new(ErrorCode::ExecutionFailed, "JPEG encoding failed"))?;
    Ok(out)
}

#[derive(Debug, Default)]
pub struct ImageProvider;

impl TransformProvider for ImageProvider {
    fn execute(
        &self,
        input: &Path,
        output: &Path,
        plan: &Plan,
        _ctx: &ExecutionContext,
    ) -> Result<()> {
        crate::runtime::validate_plan_shape(plan)?;
        let step = plan
            .steps
            .first()
            .ok_or_else(|| Error::new(ErrorCode::InputInvalid, "the image plan has no steps"))?;
        if step.operation != crate::capability::TransformId::EncodeJpeg {
            return Err(Error::new(
                ErrorCode::InputInvalid,
                "the image provider only executes JPEG encode plans",
            ));
        }
        let input_length = std::fs::metadata(input)
            .map_err(|err| {
                Error::new(
                    ErrorCode::InputInvalid,
                    format!("cannot inspect image input metadata: {err}"),
                )
            })?
            .len();
        if input_length > MAX_IMAGE_INPUT_BYTES as u64 {
            return Err(Error::new(
                ErrorCode::InspectionLimit,
                "image.input_too_large: encoded image exceeds the 32 MiB limit",
            ));
        }
        let bytes = std::fs::read(input).map_err(|err| {
            Error::new(
                ErrorCode::InputInvalid,
                format!("cannot read image input: {err}"),
            )
        })?;
        let encoded = encode_jpeg_bytes(&bytes)?;
        std::fs::write(output, encoded).map_err(|err| {
            Error::new(
                ErrorCode::ExecutionFailed,
                format!("cannot write JPEG output: {err}"),
            )
        })?;
        Ok(())
    }

    fn stream_hashes(
        &self,
        _path: &Path,
        _artifact: &Artifact,
        _ctx: &ExecutionContext,
    ) -> Result<StreamHashes> {
        Ok(StreamHashes {
            algorithm: "none".into(),
            video: String::new(),
            audio: None,
        })
    }
}

pub fn jpeg_expected(width: u32, height: u32) -> Vec<ExpectedFact> {
    vec![
        ExpectedFact {
            field: crate::constraints::Field::ImageFormat,
            value: ExpectedValue::Text("jpeg".into()),
        },
        ExpectedFact {
            field: crate::constraints::Field::ImageWidth,
            value: ExpectedValue::Integer(u64::from(width)),
        },
        ExpectedFact {
            field: crate::constraints::Field::ImageHeight,
            value: ExpectedValue::Integer(u64::from(height)),
        },
    ]
}

pub fn sample_png_rgb(width: u32, height: u32) -> Vec<u8> {
    use image::{ImageFormat, Rgb, RgbImage};
    let img = RgbImage::from_pixel(width, height, Rgb([200, 40, 40]));
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(img)
        .write_to(&mut out, ImageFormat::Png)
        .expect("png encode");
    out.into_inner()
}

pub fn sample_jpeg_rgb(width: u32, height: u32) -> Vec<u8> {
    encode_dynamic_jpeg(&DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        width,
        height,
        image::Rgb([200, 40, 40]),
    )))
    .expect("jpeg encode")
}

pub fn sample_webp_rgb(width: u32, height: u32) -> Vec<u8> {
    use image::{ImageFormat, Rgb, RgbImage};
    let img = RgbImage::from_pixel(width, height, Rgb([200, 40, 40]));
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(img)
        .write_to(&mut out, ImageFormat::WebP)
        .expect("webp encode");
    out.into_inner()
}

pub fn animated_webp_header() -> Vec<u8> {
    let mut bytes = b"RIFF\0\0\0\0WEBPVP8X".to_vec();
    bytes.extend_from_slice(&[0x0a, 0, 0, 0, 0x10, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    bytes.extend_from_slice(b"ANIM");
    bytes.extend_from_slice(&[0, 0, 0, 0]);
    let payload = (bytes.len() - 8) as u32;
    bytes[4..8].copy_from_slice(&payload.to_le_bytes());
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_png_and_jpeg_magic_not_extension() {
        assert_eq!(sniff_format(PNG_SIG), Some(ImageFormat::Png));
        assert_eq!(
            sniff_format(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Some(ImageFormat::Jpeg)
        );
        assert_eq!(sniff_format(b"not-an-image"), None);
    }

    #[test]
    fn riff_without_webp_is_not_an_image() {
        let mut bytes = b"RIFF....XXXX".to_vec();
        bytes[4..8].copy_from_slice(b"size");
        assert_eq!(sniff_format(&bytes), None);
    }

    #[test]
    fn jpeg_noop_and_png_encode_round_trip() {
        let jpeg = sample_jpeg_rgb(8, 8);
        let png = sample_png_rgb(8, 8);
        let jpeg_art = artifact_from_bytes(None, &jpeg).unwrap();
        let png_art = artifact_from_bytes(None, &png).unwrap();
        assert_eq!(
            jpeg_art.image.as_ref().unwrap().format,
            Some(ImageFormat::Jpeg)
        );
        assert_eq!(
            png_art.image.as_ref().unwrap().format,
            Some(ImageFormat::Png)
        );
        let encoded = encode_jpeg_bytes(&png).unwrap();
        let out = artifact_from_bytes(None, &encoded).unwrap();
        assert_eq!(out.image.as_ref().unwrap().format, Some(ImageFormat::Jpeg));
        assert_eq!(out.image.as_ref().unwrap().width, Some(8));
        assert_eq!(out.image.as_ref().unwrap().height, Some(8));
    }
}
