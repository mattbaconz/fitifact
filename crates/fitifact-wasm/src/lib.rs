//! Constraint-driven local bindings for the static consumer product: still images in-process,
//! and plans plus validation for media the browser worker probed and executed itself.
//!
//! Destination policy stays in `fitifact`. This crate only translates bytes and JSON across the
//! WASM boundary. It never constructs a media transform provider or performs network activity.

use std::io::Cursor;
use std::path::PathBuf;

use fitifact::error::ErrorEnvelope;
use fitifact::{
    CapabilityCatalog, ConstraintSet, ExternalOutput, ImageAdaptOptions, NeverCancelled,
    PlanOutcome, StreamHashes, TransformId, adapt_external, artifact_from_browser_probe,
    default_catalog, execute_image_adaptation, image_artifact_from_bytes, parse_image_requirements,
    plan_image_adaptation,
};
use image::{DynamicImage, ImageFormat as EncoderFormat, RgbImage, RgbaImage};

pub use fitifact::{
    sample_animated_gif, sample_bmp_rgb, sample_gif_rgb, sample_jpeg_rgb, sample_png_rgb,
    sample_tiff_rgb, sample_webp_rgb,
};

pub struct AdaptJs {
    pub report_json: String,
    pub output: Option<Vec<u8>>,
}

pub struct RgbaPlanJs {
    pub report_json: String,
    pub preview: Option<Vec<u8>>,
}

fn json_error(error: fitifact::Error) -> String {
    serde_json::to_string(&ErrorEnvelope::from(error)).expect("error envelope serializes")
}

fn parse_constraints(constraints_json: &str) -> Result<ConstraintSet, fitifact::Error> {
    fitifact::compile_from_json(constraints_json)
}

pub fn compile_requirements(requirements: &str) -> String {
    match parse_image_requirements(requirements) {
        Ok(parsed) => serde_json::to_string(&parsed).expect("requirement report serializes"),
        Err(error) => json_error(error),
    }
}

pub fn compile_constraints(constraints_json: &str) -> String {
    match parse_constraints(constraints_json) {
        Ok(constraints) => {
            serde_json::to_string(&constraints).expect("constraint report serializes")
        }
        Err(error) => json_error(error),
    }
}

pub fn compile_profile(id: &str) -> String {
    match fitifact::compile_shipped_profile(id) {
        Ok(constraints) => {
            serde_json::to_string(&constraints).expect("constraint report serializes")
        }
        Err(error) => json_error(error),
    }
}

pub fn image_limits() -> String {
    serde_json::json!({
        "schema": "fitifact.image-limits/v1",
        "max_encoded_bytes": fitifact::MAX_IMAGE_INPUT_BYTES,
        "max_decoded_pixels": fitifact::MAX_IMAGE_PIXELS,
    })
    .to_string()
}

pub fn inspect_bytes(bytes: &[u8]) -> String {
    match image_artifact_from_bytes(None, bytes) {
        Ok(artifact) => serde_json::to_string(&artifact).expect("artifact report serializes"),
        Err(error) => json_error(error),
    }
}

pub fn validate_bytes(bytes: &[u8], constraints_json: &str) -> String {
    let result = parse_constraints(constraints_json)
        .and_then(|constraints| image_artifact_from_bytes(None, bytes).map(|a| (a, constraints)));
    match result {
        Ok((artifact, constraints)) => {
            serde_json::to_string(&fitifact::check(&artifact, &constraints))
                .expect("validation report serializes")
        }
        Err(error) => json_error(error),
    }
}

pub fn plan_bytes(bytes: &[u8], constraints_json: &str) -> String {
    let result = parse_constraints(constraints_json).and_then(|constraints| {
        image_artifact_from_bytes(None, bytes).and_then(|artifact| {
            let report = fitifact::check(&artifact, &constraints);
            plan_image_adaptation(&artifact, &constraints).map(|plan| {
                serde_json::json!({
                    "schema": "fitifact.web-plan/v1",
                    "inspection": artifact,
                    "report": report,
                    "plan": plan,
                })
            })
        })
    });
    match result {
        Ok(report) => serde_json::to_string(&report).expect("plan report serializes"),
        Err(error) => json_error(error),
    }
}

pub fn adapt_bytes(bytes: &[u8], constraints_json: &str, options_json: &str) -> AdaptJs {
    let result = parse_constraints(constraints_json).and_then(|constraints| {
        let options: ImageAdaptOptions = serde_json::from_str(options_json).map_err(|error| {
            fitifact::Error::new(
                fitifact::ErrorCode::InputInvalid,
                format!("image.options_invalid: invalid adaptation options: {error}"),
            )
        })?;
        let artifact = image_artifact_from_bytes(None, bytes)?;
        let plan = plan_image_adaptation(&artifact, &constraints)?;
        execute_image_adaptation(bytes, &constraints, &plan, &options, &NeverCancelled)
    });
    match result {
        Ok(mut execution) => AdaptJs {
            output: execution.output.take(),
            report_json: serde_json::to_string(&execution).expect("adaptation report serializes"),
        },
        Err(error) => AdaptJs {
            report_json: json_error(error),
            output: None,
        },
    }
}

pub fn adapt_rgba(
    rgba: &[u8],
    width: u32,
    height: u32,
    constraints_json: &str,
    options_json: &str,
) -> AdaptJs {
    match rgba_as_png(rgba, width, height) {
        Ok(bytes) => {
            let mut adapted = adapt_bytes(&bytes, constraints_json, options_json);
            let compatible = serde_json::from_str::<serde_json::Value>(&adapted.report_json)
                .ok()
                .and_then(|report| report.get("status").cloned())
                .is_some_and(|status| status == "compatible");
            if compatible && adapted.output.is_none() {
                adapted.output = Some(bytes);
            }
            adapted
        }
        Err(error) => AdaptJs {
            report_json: json_error(error),
            output: None,
        },
    }
}

pub fn plan_rgba(rgba: &[u8], width: u32, height: u32, constraints_json: &str) -> RgbaPlanJs {
    match rgba_as_png(rgba, width, height) {
        Ok(bytes) => {
            let report_json = plan_bytes(&bytes, constraints_json);
            let failed = serde_json::from_str::<serde_json::Value>(&report_json)
                .ok()
                .and_then(|report| report.get("schema").cloned())
                .is_some_and(|schema| schema == "fitifact.error/v1");
            RgbaPlanJs {
                report_json,
                preview: (!failed).then_some(bytes),
            }
        }
        Err(error) => RgbaPlanJs {
            report_json: json_error(error),
            preview: None,
        },
    }
}

/// The default catalog narrowed to the transforms this browser measured it can run.
fn media_catalog(transforms_json: &str) -> Result<CapabilityCatalog, fitifact::Error> {
    let allowed: Vec<TransformId> = serde_json::from_str(transforms_json).map_err(|error| {
        fitifact::Error::new(
            fitifact::ErrorCode::InputInvalid,
            format!("media.transforms_invalid: {error}"),
        )
    })?;
    let mut catalog = default_catalog();
    catalog
        .capabilities
        .retain(|capability| allowed.contains(&capability.id));
    Ok(catalog)
}

pub fn inspect_media_probe(probe_json: &str) -> String {
    match artifact_from_browser_probe(probe_json) {
        Ok(artifact) => serde_json::to_string(&artifact).expect("artifact report serializes"),
        Err(error) => json_error(error),
    }
}

pub fn plan_media_probe(probe_json: &str, constraints_json: &str, transforms_json: &str) -> String {
    let result = parse_constraints(constraints_json).and_then(|constraints| {
        let catalog = media_catalog(transforms_json)?;
        let artifact = artifact_from_browser_probe(probe_json)?;
        let report = fitifact::check(&artifact, &constraints);
        let outcome = fitifact::plan(&artifact, &constraints, &catalog);
        // The full catalog (with an FFmpeg that can tone-map) finds a plan this browser cannot run.
        let needs_encoder = matches!(outcome, PlanOutcome::CannotSatisfy { .. })
            && fitifact::plan(
                &artifact,
                &constraints,
                &default_catalog().with_hdr_tone_mapping(true),
            )
            .plan()
            .is_some();
        Ok(serde_json::json!({
            "schema": "fitifact.web-media-plan/v1",
            "inspection": artifact,
            "constraints": constraints,
            "report": report,
            "outcome": outcome,
            "needs_encoder": needs_encoder,
        }))
    });
    match result {
        Ok(report) => serde_json::to_string(&report).expect("plan report serializes"),
        Err(error) => json_error(error),
    }
}

/// `output_json` is `null` when nothing ran, or `{name, probe, source_hashes, output_hashes}`
/// describing the re-probed output and SHA-256 digests of each stream's packet payloads.
pub fn adapt_media_probe(
    probe_json: &str,
    constraints_json: &str,
    transforms_json: &str,
    output_json: &str,
) -> String {
    let result = parse_constraints(constraints_json).and_then(|constraints| {
        let catalog = media_catalog(transforms_json)?;
        let artifact = artifact_from_browser_probe(probe_json)?;
        let output = external_output(output_json)?;
        let original = artifact.path.clone().unwrap_or_default();
        Ok(adapt_external(
            original,
            artifact,
            &constraints,
            &catalog,
            output,
        ))
    });
    match result {
        Ok(result) => serde_json::to_string(&result).expect("adaptation report serializes"),
        Err(error) => json_error(error),
    }
}

fn external_output(output_json: &str) -> Result<Option<ExternalOutput>, fitifact::Error> {
    let invalid = |message: String| {
        fitifact::Error::new(
            fitifact::ErrorCode::ValidationFailed,
            format!("media.output_invalid: {message}"),
        )
    };
    let output: serde_json::Value =
        serde_json::from_str(output_json).map_err(|error| invalid(error.to_string()))?;
    if output.is_null() {
        return Ok(None);
    }
    let hashes = |key: &str| {
        serde_json::from_value::<StreamHashes>(output[key].clone())
            .map_err(|error| invalid(format!("{key}: {error}")))
    };
    let name = output["name"]
        .as_str()
        .ok_or_else(|| invalid("name is missing".into()))?;
    let artifact = artifact_from_browser_probe(&output["probe"].to_string()).map_err(|error| {
        invalid(format!(
            "the output could not be freshly inspected: {}",
            error.message
        ))
    })?;
    Ok(Some(ExternalOutput {
        path: PathBuf::from(name),
        artifact,
        source_hashes: hashes("source_hashes")?,
        output_hashes: hashes("output_hashes")?,
    }))
}

#[allow(clippy::chunks_exact_to_as_chunks)]
fn rgba_as_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, fitifact::Error> {
    fitifact::image::enforce_decoded_limit(width, height)?;
    let expected = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| {
            fitifact::Error::new(
                fitifact::ErrorCode::InspectionLimit,
                "image.rgba_size_overflow: decoded RGBA dimensions exceed the addressable range",
            )
        })?;
    if rgba.len() != expected {
        return Err(fitifact::Error::new(
            fitifact::ErrorCode::InputInvalid,
            format!(
                "image.rgba_length_mismatch: expected {expected} RGBA bytes, received {}",
                rgba.len()
            ),
        ));
    }
    let image = if rgba.chunks_exact(4).all(|pixel| pixel[3] == u8::MAX) {
        let rgb = rgba
            .chunks_exact(4)
            .flat_map(|pixel| pixel[..3].iter().copied())
            .collect();
        let pixels = RgbImage::from_raw(width, height, rgb).ok_or_else(|| {
            fitifact::Error::new(
                fitifact::ErrorCode::InputInvalid,
                "image.rgba_invalid: decoded opaque pixels are invalid",
            )
        })?;
        DynamicImage::ImageRgb8(pixels)
    } else {
        let pixels = RgbaImage::from_raw(width, height, rgba.to_vec()).ok_or_else(|| {
            fitifact::Error::new(
                fitifact::ErrorCode::InputInvalid,
                "image.rgba_invalid: decoded RGBA pixels are invalid",
            )
        })?;
        DynamicImage::ImageRgba8(pixels)
    };
    let mut encoded = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut encoded), EncoderFormat::Png)
        .map_err(|error| {
            fitifact::Error::new(
                fitifact::ErrorCode::ExecutionFailed,
                format!("image.rgba_encode_failed: could not encode decoded pixels: {error}"),
            )
        })?;
    Ok(encoded)
}

#[cfg(target_arch = "wasm32")]
mod wasm_exports {
    use super::{AdaptJs as NativeAdapt, RgbaPlanJs as NativeRgbaPlan};
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub fn compile_requirements(requirements: &str) -> String {
        super::compile_requirements(requirements)
    }

    #[wasm_bindgen]
    pub fn compile_constraints(constraints_json: &str) -> String {
        super::compile_constraints(constraints_json)
    }

    #[wasm_bindgen]
    pub fn compile_profile(id: &str) -> String {
        super::compile_profile(id)
    }

    #[wasm_bindgen]
    pub fn image_limits() -> String {
        super::image_limits()
    }

    #[wasm_bindgen]
    pub fn inspect_bytes(bytes: &[u8]) -> String {
        super::inspect_bytes(bytes)
    }

    #[wasm_bindgen]
    pub fn validate_bytes(bytes: &[u8], constraints_json: &str) -> String {
        super::validate_bytes(bytes, constraints_json)
    }

    #[wasm_bindgen]
    pub fn plan_bytes(bytes: &[u8], constraints_json: &str) -> String {
        super::plan_bytes(bytes, constraints_json)
    }

    #[wasm_bindgen]
    pub fn inspect_media_probe(probe_json: &str) -> String {
        super::inspect_media_probe(probe_json)
    }

    #[wasm_bindgen]
    pub fn plan_media_probe(
        probe_json: &str,
        constraints_json: &str,
        transforms_json: &str,
    ) -> String {
        super::plan_media_probe(probe_json, constraints_json, transforms_json)
    }

    #[wasm_bindgen]
    pub fn adapt_media_probe(
        probe_json: &str,
        constraints_json: &str,
        transforms_json: &str,
        output_json: &str,
    ) -> String {
        super::adapt_media_probe(probe_json, constraints_json, transforms_json, output_json)
    }

    #[wasm_bindgen]
    pub struct RgbaPlanJs {
        report_json: String,
        preview: Option<Vec<u8>>,
    }

    #[wasm_bindgen]
    impl RgbaPlanJs {
        #[wasm_bindgen(getter)]
        pub fn report_json(&self) -> String {
            self.report_json.clone()
        }

        pub fn take_preview(&mut self) -> Option<Vec<u8>> {
            self.preview.take()
        }
    }

    impl From<NativeRgbaPlan> for RgbaPlanJs {
        fn from(value: NativeRgbaPlan) -> Self {
            Self {
                report_json: value.report_json,
                preview: value.preview,
            }
        }
    }

    #[wasm_bindgen]
    pub fn plan_rgba(rgba: &[u8], width: u32, height: u32, constraints_json: &str) -> RgbaPlanJs {
        super::plan_rgba(rgba, width, height, constraints_json).into()
    }

    #[wasm_bindgen]
    pub struct AdaptJs {
        report_json: String,
        output: Option<Vec<u8>>,
    }

    #[wasm_bindgen]
    impl AdaptJs {
        #[wasm_bindgen(getter)]
        pub fn report_json(&self) -> String {
            self.report_json.clone()
        }

        pub fn take_output(&mut self) -> Option<Vec<u8>> {
            self.output.take()
        }
    }

    impl From<NativeAdapt> for AdaptJs {
        fn from(value: NativeAdapt) -> Self {
            Self {
                report_json: value.report_json,
                output: value.output,
            }
        }
    }

    #[wasm_bindgen]
    pub fn adapt_bytes(bytes: &[u8], constraints_json: &str, options_json: &str) -> AdaptJs {
        super::adapt_bytes(bytes, constraints_json, options_json).into()
    }

    #[wasm_bindgen]
    pub fn adapt_rgba(
        rgba: &[u8],
        width: u32,
        height: u32,
        constraints_json: &str,
        options_json: &str,
    ) -> AdaptJs {
        super::adapt_rgba(rgba, width, height, constraints_json, options_json).into()
    }
}
