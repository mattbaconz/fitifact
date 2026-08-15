use std::fmt;

use serde::{Deserialize, Serialize};

use crate::contract::ErrorSchema;

/// Machine-readable error categories from the v0 error model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InputInvalid,
    InspectionUnsupported,
    InspectionLimit,
    RequirementsAmbiguous,
    RequirementsConflict,
    NoValidPlan,
    ProviderMissing,
    ExecutionFailed,
    ExecutionLimit,
    ValidationFailed,
    SecurityBlocked,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            Self::InputInvalid => "INPUT_INVALID",
            Self::InspectionUnsupported => "INSPECTION_UNSUPPORTED",
            Self::InspectionLimit => "INSPECTION_LIMIT",
            Self::RequirementsAmbiguous => "REQUIREMENTS_AMBIGUOUS",
            Self::RequirementsConflict => "REQUIREMENTS_CONFLICT",
            Self::NoValidPlan => "NO_VALID_PLAN",
            Self::ProviderMissing => "PROVIDER_MISSING",
            Self::ExecutionFailed => "EXECUTION_FAILED",
            Self::ExecutionLimit => "EXECUTION_LIMIT",
            Self::ValidationFailed => "VALIDATION_FAILED",
            Self::SecurityBlocked => "SECURITY_BLOCKED",
        };
        f.write_str(code)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{code}: {message}")]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
}

impl Error {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorEnvelope {
    pub schema: ErrorSchema,
    pub code: ErrorCode,
    pub message: String,
    pub details: std::collections::BTreeMap<String, serde_json::Value>,
    pub retryable: bool,
    pub suggestions: Vec<String>,
}

impl From<Error> for ErrorEnvelope {
    fn from(error: Error) -> Self {
        Self {
            schema: ErrorSchema,
            code: error.code,
            message: error.message,
            details: std::collections::BTreeMap::new(),
            retryable: false,
            suggestions: Vec::new(),
        }
    }
}
