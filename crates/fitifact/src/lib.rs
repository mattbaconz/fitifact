//! Fitifact core: inspect → compile → check → plan → execute → validate.
//!
//! Integrations stay thin. Destination policy lives here, not in the CLI.

pub mod adapt;
pub mod artifact;
pub mod capability;
pub mod check;
pub mod constraints;
pub mod contract;
pub mod doctor;
pub mod error;
pub mod ffmpeg;
pub mod inspect;
pub mod plan;
pub mod report;
pub mod runtime;
pub mod validate;

pub use adapt::{AdaptRequest, AdaptationResult, AdaptationStatus, adapt};
pub use artifact::{Artifact, AudioCodec, AudioStream, Container, Family, VideoCodec, VideoStream};
pub use capability::{CapabilityCatalog, TransformId, default_catalog};
pub use check::{CheckResult, CompatibilityReport, check};
pub use constraints::{
    ConstraintInput, ConstraintSet, compile, compile_from_yaml, media_h264_mp4_aac,
};
pub use error::{Error, ErrorCode};
pub use ffmpeg::{FfmpegProvider, ffmpeg_args};
pub use inspect::{FfprobeInspector, Inspector, artifact_from_ffprobe_json, inspect};
pub use plan::{Plan, PlanOutcome, plan};
pub use report::{Explanation, explain_check, explain_plan};
pub use runtime::{
    ExecutionContext, ProcessSpawner, RecordingSpawner, SystemSpawner, TransformProvider, execute,
};
pub use validate::{ValidationReport, ValidationStatus, validate};
