//! Fitifact core: inspect → compile → check → plan → execute → validate.
//!
//! Integrations stay thin. Destination policy lives here, not in the CLI.

pub mod adapt;
pub mod artifact;
pub mod bench;
pub mod capability;
pub mod check;
pub mod constraints;
pub mod contract;
pub mod doctor;
pub mod error;
pub mod ffmpeg;
pub mod image;
pub mod image_adapt;
pub mod inspect;
pub mod local;
pub mod plan;
pub mod report;
pub mod requirements;
pub mod runtime;
pub mod validate;

pub use adapt::{AdaptRequest, AdaptationResult, AdaptationStatus, CleanupWarning, adapt};
pub use artifact::{
    Artifact, AudioCodec, AudioStream, Container, Family, ImageFacts, ImageFormat, VideoCodec,
    VideoStream,
};
pub use bench::{
    BenchOptions, BenchReport, find_lockfile, network_crates_in_lockfile, resolve_fixtures,
    run_bench,
};
pub use capability::{CapabilityCatalog, TransformId, default_catalog};
pub use check::{CheckResult, CompatibilityReport, check};
pub use constraints::{
    ConstraintInput, ConstraintSet, compile, compile_from_json, compile_from_yaml, image_jpeg,
    media_h264_mp4_aac,
};
pub use error::{Error, ErrorCode};
pub use ffmpeg::{FfmpegProvider, ffmpeg_args};
pub use image::{
    ImageProvider, MAX_IMAGE_INPUT_BYTES, MAX_IMAGE_PIXELS,
    artifact_from_bytes as image_artifact_from_bytes, encode_jpeg_bytes, sample_jpeg_rgb,
    sample_png_rgb, sample_webp_rgb,
};
pub use image_adapt::{
    AtomicCancellation, BuiltinImageProvider, CancellationSignal, ImageAdaptExecution,
    ImageAdaptOperation, ImageAdaptOptions, ImageAdaptPlan, ImageAdaptProvider,
    ImageAdaptStepTarget, ImageAdaptTarget, ImageCropRequirement, ImageExecutionStats,
    ImageMetadataBehavior, ImagePreservationClaim, ImageProviderOutput, NeverCancelled,
    NormalizedCropRectangle, execute_image_adaptation, execute_image_adaptation_with_provider,
    plan_image_adaptation,
};
pub use inspect::{
    DefaultInspector, FfprobeInspector, Inspector, artifact_from_ffprobe_json, inspect,
};
pub use local::{LocalImageResult, adapt_local_image_bytes, inspect_local_bytes};
pub use plan::{Plan, PlanOutcome, plan};
pub use report::{Explanation, explain_check, explain_plan};
pub use requirements::{
    RequirementAmbiguity, RequirementParse, RequirementSourceSpan, UnresolvedRequirement,
    parse_image_requirements,
};
pub use runtime::{
    ExecutionContext, ProcessSpawner, RecordingSpawner, StreamHashes, SystemSpawner,
    TransformProvider, execute, validate_plan_for_execution,
};
pub use validate::{
    DURATION_TOLERANCE_MS, ProvenanceCheck, ValidationCheck, ValidationReport, ValidationStatus,
    validate, validate_adaptation,
};
