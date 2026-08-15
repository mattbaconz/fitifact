use serde::{Deserialize, Serialize};

use crate::artifact::{Artifact, Family};
use crate::constraints::Field;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransformId {
    #[serde(rename = "media.remux")]
    Remux,
    #[serde(rename = "media.transcode_video")]
    TranscodeVideo,
}

impl TransformId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Remux => "media.remux",
            Self::TranscodeVideo => "media.transcode_video",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LossClass {
    None,
    Low,
    Semantic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    pub id: TransformId,
    pub can_set: Vec<Field>,
    pub loss: LossClass,
    pub streams_changed: u32,
    pub requires_media: bool,
    pub requires_video: bool,
    /// If true, the step is only instantiated when video codec is fail/unknown.
    pub requires_video_codec_change: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CapabilityCatalog {
    pub capabilities: Vec<Capability>,
}

pub fn default_catalog() -> CapabilityCatalog {
    CapabilityCatalog {
        capabilities: vec![
            Capability {
                id: TransformId::Remux,
                can_set: vec![Field::MediaContainer],
                loss: LossClass::None,
                streams_changed: 0,
                requires_media: true,
                requires_video: false,
                requires_video_codec_change: false,
            },
            Capability {
                id: TransformId::TranscodeVideo,
                can_set: vec![Field::MediaVideoCodec, Field::MediaContainer],
                loss: LossClass::Low,
                streams_changed: 1,
                requires_media: true,
                requires_video: true,
                requires_video_codec_change: true,
            },
        ],
    }
}

impl Capability {
    pub fn preconditions_met(&self, artifact: &Artifact) -> bool {
        if self.requires_media && artifact.family != Family::Media {
            return false;
        }
        if self.requires_video && artifact.first_video().is_none() {
            return false;
        }
        true
    }

    pub fn semantic_penalty(&self) -> u32 {
        match self.loss {
            LossClass::Semantic => 1,
            _ => 0,
        }
    }

    pub fn lossy_penalty(&self) -> u32 {
        match self.loss {
            LossClass::None => 0,
            LossClass::Low | LossClass::Semantic => 1,
        }
    }
}
