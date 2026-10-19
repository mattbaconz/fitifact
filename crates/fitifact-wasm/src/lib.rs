//! Constraint-driven local bindings for the static consumer product: still images in-process,
//! and plans plus validation for media the browser worker probed and executed itself.
//!
//! Destination policy stays in `fitifact`. This crate only translates bytes and JSON across the
//! WASM boundary. It never constructs a media transform provider or performs network activity.

use std::io::Cursor;
use std::path::PathBuf;

use fitifact::error::ErrorEnvelope;
use fitifact::media_fit::DEFAULT_AUDIO_BPS;
use fitifact::plan::{PlanStep, PreservationClaim};
use fitifact::{
    Artifact, CapabilityCatalog, ConstraintSet, ExternalOutput, ImageAdaptOptions, NeverCancelled,
    Plan, StreamHashes, TransformId, ValidationCheck, ValidationStatus, adapt_external,
    artifact_from_browser_probe, default_catalog, execute_image_adaptation, h264_config_is_sound,
    image_artifact_from_bytes, parse_image_requirements, plan_image_adaptation,
    quality_video_bitrate_bps, video_bitrate_ladder,
};
use image::{DynamicImage, ImageFormat as EncoderFormat, RgbImage, RgbaImage};
use serde::Deserialize;
use serde_json::{Value, json};

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

/// Every transform the engine has, with HDR tone mapping as FFmpeg can run it. The browser runs a
/// plan only when it is the plan this catalog finds, never a lossier one shaped around what the
/// browser lacks.
fn engine_catalog() -> CapabilityCatalog {
    default_catalog().with_hdr_tone_mapping(true)
}

/// What the worker measured, when a plan asked, about encoding this source in this browser.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Measured {
    /// How players show the source's video track; `null` without one.
    geometry: Option<Geometry>,
    /// Whether a decoder accepts the source's video track.
    video_decode: bool,
    /// The AVCDecoderConfigurationRecord the browser's H.264 encoder wrote in a trial of the
    /// planned encode (the source's first half second at the requested size and bitrate); `null`
    /// when it has no such encoder or the trial did not finish.
    h264_sample: Option<Vec<u8>>,
    /// Whether a decoder accepts the source's audio track.
    audio_decode: bool,
    /// Whether an AAC encoder accepts the requested stereo encode.
    aac_encode: bool,
}

/// How a player shows the picture: display size with pixel aspect applied, rotation, mirroring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Geometry {
    display_width: u32,
    display_height: u32,
    rotation: u32,
    flip: bool,
}

impl Geometry {
    fn upright(width: u32, height: u32) -> Self {
        Self {
            display_width: width,
            display_height: height,
            rotation: 0,
            flip: false,
        }
    }

    fn describe(geometry: Option<Self>) -> String {
        geometry.map_or_else(
            || "unknown".into(),
            |shown| {
                format!(
                    "{}x{}, rotated {}, flipped {}",
                    shown.display_width, shown.display_height, shown.rotation, shown.flip
                )
            },
        )
    }
}

/// What this browser does with the engine's plan.
enum Verdict {
    /// Instructions the worker follows.
    Ready(Value),
    /// The encode the worker must try before the engine can judge this browser.
    Measure(Value),
    /// Why the plan cannot run here, as codes the web app puts into words.
    Refused(Vec<&'static str>),
}

impl Verdict {
    fn json(self) -> Value {
        match self {
            Self::Ready(execution) => json!({ "status": "ready", "execution": execution }),
            Self::Measure(request) => json!({ "status": "measure", "measure": request }),
            Self::Refused(missing) => json!({ "status": "refused", "missing": missing }),
        }
    }
}

fn verdict(artifact: &Artifact, plan: &Plan, measured: Option<&Measured>) -> Verdict {
    match plan.steps.as_slice() {
        [step] if step.operation == TransformId::Remux => {
            Verdict::Ready(json!({ "operation": step.operation }))
        }
        [step] if step.operation == TransformId::TranscodeVideo => {
            transcode_verdict(artifact, step, measured)
        }
        [step] if step.operation == TransformId::TranscodeAudio => {
            Verdict::Refused(vec!["audio_only_encode"])
        }
        _ => Verdict::Refused(vec!["unsupported"]),
    }
}

/// The native scale-to-fit and bitrate ladder, run by WebCodecs. Without a constant-quality mode
/// the first attempt spends the quality bitrate unless the size limit forces less.
fn transcode_verdict(artifact: &Artifact, step: &PlanStep, measured: Option<&Measured>) -> Verdict {
    // The browser encode writes 8-bit BT.709 SDR and has no tone mapping that matches FFmpeg's.
    if step.target.tone_map_hdr {
        return Verdict::Refused(vec!["hdr"]);
    }
    let encodes_audio = step.target.audio_codec.is_some();
    let has_audio = encodes_audio
        || step
            .preservation
            .contains(&PreservationClaim::AudioStreamCopied);
    let ladder = step.target.width.zip(step.target.height).and_then(|size| {
        let quality = quality_video_bitrate_bps(size.0, size.1);
        let bitrates = match (step.target.max_bytes, step.target.duration_ms) {
            (None, _) => Some(vec![quality]),
            (Some(max_bytes), Some(duration_ms)) => {
                video_bitrate_ladder(max_bytes, duration_ms, has_audio, size, Some(quality))
            }
            (Some(_), None) => None,
        };
        Some((size, bitrates?))
    });
    let Some(((width, height), bitrates)) = ladder else {
        return Verdict::Refused(vec!["unsupported"]);
    };
    let Some(measured) = measured else {
        return Verdict::Measure(json!({
            "video": { "codec": "avc", "width": width, "height": height, "bitrate": bitrates[0] },
            "audio": encodes_audio.then(|| json!({
                "codec": "aac",
                "channels": 2,
                "sample_rate": artifact.first_audio().and_then(|audio| audio.sample_rate),
                "bitrate": DEFAULT_AUDIO_BPS,
            })),
        }));
    };
    let mut missing = Vec::new();
    if measured.geometry.is_none() || measured.geometry != stored_geometry(artifact) {
        missing.push("display_transform");
    }
    if !measured.video_decode {
        missing.push("video_decoder");
    }
    match &measured.h264_sample {
        None => missing.push("h264_encoder"),
        Some(sample) if !h264_config_is_sound(sample) => missing.push("h264_encoder_output"),
        Some(_) => {}
    }
    if encodes_audio && !measured.audio_decode {
        missing.push("audio_decoder");
    }
    if encodes_audio && !measured.aac_encode {
        missing.push("aac_encoder");
    }
    if !missing.is_empty() {
        return Verdict::Refused(missing);
    }
    Verdict::Ready(json!({
        "operation": step.operation,
        "width": width,
        "height": height,
        "video_bitrates": bitrates,
        "audio": if encodes_audio {
            json!({ "codec": "aac", "channels": 2, "bitrate": DEFAULT_AUDIO_BPS })
        } else {
            json!("copy")
        },
        "max_bytes": step.target.max_bytes,
    }))
}

/// The video shown exactly as stored: no rotation, mirroring, or pixel-aspect stretch.
fn stored_geometry(artifact: &Artifact) -> Option<Geometry> {
    let video = artifact.first_video()?;
    Some(Geometry::upright(video.width?, video.height?))
}

fn parse_measured(measured_json: &str) -> Result<Option<Measured>, fitifact::Error> {
    serde_json::from_str(measured_json).map_err(|error| {
        fitifact::Error::new(
            fitifact::ErrorCode::InputInvalid,
            format!("media.measured_invalid: {error}"),
        )
    })
}

pub fn inspect_media_probe(probe_json: &str) -> String {
    match artifact_from_browser_probe(probe_json) {
        Ok(artifact) => serde_json::to_string(&artifact).expect("artifact report serializes"),
        Err(error) => json_error(error),
    }
}

/// `measured_json` is `null` until a plan asks the worker to measure an encode, then what it
/// measured. `browser` says whether this browser can run the engine's plan.
pub fn plan_media_probe(probe_json: &str, constraints_json: &str, measured_json: &str) -> String {
    let result = parse_constraints(constraints_json).and_then(|constraints| {
        let measured = parse_measured(measured_json)?;
        let artifact = artifact_from_browser_probe(probe_json)?;
        let report = fitifact::check(&artifact, &constraints);
        let outcome = fitifact::plan(&artifact, &constraints, &engine_catalog());
        let browser = outcome
            .plan()
            .map(|plan| verdict(&artifact, plan, measured.as_ref()).json());
        Ok(json!({
            "schema": "fitifact.web-media-plan/v2",
            "inspection": artifact,
            "constraints": constraints,
            "report": report,
            "outcome": outcome,
            "browser": browser,
        }))
    });
    match result {
        Ok(report) => serde_json::to_string(&report).expect("plan report serializes"),
        Err(error) => json_error(error),
    }
}

/// `output_json` is `null` when nothing ran, or `{name, probe, source_hashes, output_hashes,
/// source_geometry, output_geometry}` describing the re-probed output and SHA-256 digests of
/// each stream's packet payloads.
pub fn adapt_media_probe(probe_json: &str, constraints_json: &str, output_json: &str) -> String {
    let result = parse_constraints(constraints_json).and_then(|constraints| {
        let catalog = engine_catalog();
        let artifact = artifact_from_browser_probe(probe_json)?;
        let outcome = fitifact::plan(&artifact, &constraints, &catalog);
        if let Some(Verdict::Refused(missing)) =
            outcome.plan().map(|plan| verdict(&artifact, plan, None))
        {
            return Err(fitifact::Error::new(
                fitifact::ErrorCode::ProviderMissing,
                format!(
                    "media.browser_cannot_run: this browser cannot run the plan ({})",
                    missing.join(", ")
                ),
            ));
        }
        let output = external_output(output_json, &artifact, outcome.plan())?;
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

fn external_output(
    output_json: &str,
    source: &Artifact,
    plan: Option<&Plan>,
) -> Result<Option<ExternalOutput>, fitifact::Error> {
    let invalid = |message: String| {
        fitifact::Error::new(
            fitifact::ErrorCode::ValidationFailed,
            format!("media.output_invalid: {message}"),
        )
    };
    let output: Value =
        serde_json::from_str(output_json).map_err(|error| invalid(error.to_string()))?;
    if output.is_null() {
        return Ok(None);
    }
    let hashes = |key: &str| {
        let hashes = serde_json::from_value::<StreamHashes>(output[key].clone())
            .map_err(|error| invalid(format!("{key}: {error}")))?;
        let mut digests = std::iter::once(&hashes.video).chain(&hashes.audio);
        if hashes.algorithm != "sha256" || !digests.all(|digest| is_sha256(digest)) {
            return Err(invalid(format!("{key} must be SHA-256 hex digests")));
        }
        Ok(hashes)
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
    let geometry = |key: &str| {
        serde_json::from_value::<Option<Geometry>>(output[key].clone())
            .map_err(|error| invalid(format!("{key}: {error}")))
    };
    let (shown, produced) = (geometry("source_geometry")?, geometry("output_geometry")?);
    let transcode = plan
        .and_then(|plan| plan.steps.first())
        .filter(|step| step.operation == TransformId::TranscodeVideo);
    // A copy must look exactly like the source; an encode is upright at the planned size, and
    // only ever made from video shown as stored.
    let expected = match transcode {
        Some(step) => {
            if shown.is_none() || shown != stored_geometry(source) {
                return Err(invalid(
                    "the browser re-encodes only video shown as stored, without rotation, \
                     mirroring, or stretching"
                        .into(),
                ));
            }
            step.target
                .width
                .zip(step.target.height)
                .map(|(width, height)| Geometry::upright(width, height))
        }
        None => shown,
    };
    let integrity = if expected.is_none() && produced.is_none() {
        Vec::new()
    } else {
        vec![ValidationCheck {
            id: "display_geometry".into(),
            status: if expected.is_some() && expected == produced {
                ValidationStatus::Pass
            } else {
                ValidationStatus::Fail
            },
            expected: Geometry::describe(expected),
            actual: Geometry::describe(produced),
        }]
    };
    Ok(Some(ExternalOutput {
        path: PathBuf::from(name),
        artifact,
        source_hashes: hashes("source_hashes")?,
        output_hashes: hashes("output_hashes")?,
        integrity,
    }))
}

fn is_sha256(digest: &str) -> bool {
    digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
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
        measured_json: &str,
    ) -> String {
        super::plan_media_probe(probe_json, constraints_json, measured_json)
    }

    #[wasm_bindgen]
    pub fn adapt_media_probe(
        probe_json: &str,
        constraints_json: &str,
        output_json: &str,
    ) -> String {
        super::adapt_media_probe(probe_json, constraints_json, output_json)
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
