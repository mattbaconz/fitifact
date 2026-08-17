use std::io::Cursor;
use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::{ColorType, DynamicImage, ImageDecoder, ImageReader};

use crate::artifact::{
    Artifact, ArtifactSchema, Completeness, Family, ImageFacts, ImageFormat, InspectionMeta,
};
use crate::error::{Error, ErrorCode, Result};
use crate::plan::{ExpectedFact, ExpectedValue, Plan};
use crate::runtime::{ExecutionContext, StreamHashes, TransformProvider};

const PNG_SIG: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

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
    let Some(format) = sniff_format(bytes) else {
        return Err(Error::new(
            ErrorCode::InspectionUnsupported,
            "the input is not a recognized image",
        ));
    };
    let animated = match format {
        ImageFormat::Png => Some(png_is_animated(bytes)),
        ImageFormat::Gif => Some(true),
        ImageFormat::Webp => Some(webp_is_animated(bytes)),
        _ => Some(false),
    };
    let decoded = match format {
        ImageFormat::Jpeg | ImageFormat::Png => decode_still(bytes)?,
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
    let decoder = reader
        .into_decoder()
        .map_err(|_| Error::new(ErrorCode::InputInvalid, "the image could not be decoded"))?;
    let (width, height) = decoder.dimensions();
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

fn png_is_animated(bytes: &[u8]) -> bool {
    bytes.windows(4).any(|window| window == b"acTL")
}

fn webp_is_animated(bytes: &[u8]) -> bool {
    bytes.windows(4).any(|window| window == b"ANIM")
}

pub fn encode_jpeg_bytes(bytes: &[u8]) -> Result<Vec<u8>> {
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
    let image = image::load_from_memory(bytes).map_err(|_| {
        Error::new(
            ErrorCode::ExecutionFailed,
            "the PNG source could not be decoded",
        )
    })?;
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
