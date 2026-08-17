//! Local image inspect/adapt for the static drop page.
//!
//! This crate never constructs a media transform provider.
//! Video bytes return `INSPECTION_UNSUPPORTED` and tell the caller to use the CLI.

use fitifact::constraints::image_jpeg;
use fitifact::error::ErrorEnvelope;
use fitifact::{adapt_local_image_bytes, inspect_local_bytes};

pub use fitifact::{sample_jpeg_rgb, sample_png_rgb};

pub struct AdaptJs {
    pub report_json: String,
    pub output: Option<Vec<u8>>,
}

pub fn inspect_bytes(bytes: &[u8]) -> String {
    match inspect_local_bytes(bytes) {
        Ok(artifact) => serde_json::to_string(&artifact).expect("artifact json"),
        Err(err) => serde_json::to_string(&ErrorEnvelope::from(err)).expect("error json"),
    }
}

pub fn check_bytes(bytes: &[u8]) -> String {
    match inspect_local_bytes(bytes) {
        Ok(artifact) => {
            let report = fitifact::check(&artifact, &image_jpeg());
            serde_json::to_string(&report).expect("check json")
        }
        Err(err) => serde_json::to_string(&ErrorEnvelope::from(err)).expect("error json"),
    }
}

pub fn plan_bytes(bytes: &[u8]) -> String {
    match inspect_local_bytes(bytes) {
        Ok(artifact) => {
            let outcome = fitifact::plan(&artifact, &image_jpeg(), &fitifact::default_catalog());
            serde_json::to_string(&outcome).expect("plan json")
        }
        Err(err) => serde_json::to_string(&ErrorEnvelope::from(err)).expect("error json"),
    }
}

pub fn adapt_bytes(bytes: &[u8]) -> AdaptJs {
    match adapt_local_image_bytes(bytes, &image_jpeg()) {
        Ok(result) => AdaptJs {
            output: result.output.clone(),
            report_json: serde_json::to_string(&result).expect("adapt json"),
        },
        Err(err) => AdaptJs {
            report_json: serde_json::to_string(&ErrorEnvelope::from(err)).expect("error json"),
            output: None,
        },
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm_exports {
    use super::AdaptJs as NativeAdapt;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub fn inspect_bytes(bytes: &[u8]) -> String {
        super::inspect_bytes(bytes)
    }

    #[wasm_bindgen]
    pub fn check_bytes(bytes: &[u8]) -> String {
        super::check_bytes(bytes)
    }

    #[wasm_bindgen]
    pub fn plan_bytes(bytes: &[u8]) -> String {
        super::plan_bytes(bytes)
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

        #[wasm_bindgen(getter)]
        pub fn output(&self) -> Option<Vec<u8>> {
            self.output.clone()
        }
    }

    #[wasm_bindgen]
    pub fn adapt_bytes(bytes: &[u8]) -> AdaptJs {
        let NativeAdapt {
            report_json,
            output,
        } = super::adapt_bytes(bytes);
        AdaptJs {
            report_json,
            output,
        }
    }
}
