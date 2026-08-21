//! Constraint-driven local image bindings for the static consumer product.
//!
//! Destination policy stays in `fitifact`. This crate only translates bytes and JSON across the
//! WASM boundary. It never constructs a media transform provider or performs network activity.

use std::io::Cursor;

use fitifact::error::ErrorEnvelope;
use fitifact::{
    ConstraintSet, ImageAdaptOptions, NeverCancelled, execute_image_adaptation,
    image_artifact_from_bytes, parse_image_requirements, plan_image_adaptation,
};
use image::{DynamicImage, ImageFormat as EncoderFormat, RgbaImage};

pub use fitifact::{sample_jpeg_rgb, sample_png_rgb};

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
    let pixels = RgbaImage::from_raw(width, height, rgba.to_vec()).ok_or_else(|| {
        fitifact::Error::new(
            fitifact::ErrorCode::InputInvalid,
            "image.rgba_invalid: decoded RGBA pixels are invalid",
        )
    })?;
    let mut encoded = Vec::new();
    DynamicImage::ImageRgba8(pixels)
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
