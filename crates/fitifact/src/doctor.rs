use serde::{Deserialize, Serialize};

use crate::contract::DoctorSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorTool {
    pub name: String,
    pub available: bool,
    pub version: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorReport {
    pub schema: DoctorSchema,
    pub tools: Vec<DoctorTool>,
}

impl DoctorReport {
    pub fn new(tools: Vec<DoctorTool>) -> Self {
        Self {
            schema: DoctorSchema,
            tools,
        }
    }
}
