#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::constraints::{
    Constraint, ConstraintSet, ConstraintValue, Field, Operator, Preferences,
    validate_and_normalize,
};
use crate::contract::ConstraintsSchema;
use crate::error::{Error, ErrorCode, Result};

const PROFILE_SCHEMA: &str = "fitifact.profile/v1";
const MAX_PROFILE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Deserialize)]
struct ProfileDocument {
    schema: String,
    id: String,
    name: String,
    revision: u64,
    last_verified: String,
    #[serde(default)]
    scope: Option<ProfileScope>,
    constraints: Vec<ProfileConstraint>,
    #[serde(default)]
    preferences: Option<ProfilePreferences>,
    #[serde(default)]
    sources: Vec<ProfileSource>,
}

#[derive(Debug, Deserialize)]
struct ProfileConstraint {
    id: Option<String>,
    field: Field,
    op: Operator,
    value: ConstraintValue,
    #[serde(default)]
    #[allow(dead_code)]
    source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProfileScope {
    #[serde(default)]
    kind: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProfilePreferences {
    #[serde(default)]
    preserve: Option<ProfilePreserve>,
}

#[derive(Debug, Deserialize)]
struct ProfilePreserve {
    #[serde(default)]
    audio: Option<String>,
    #[serde(default)]
    resolution: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProfileSource {
    id: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    observed_at: Option<String>,
    #[serde(default)]
    note: Option<String>,
}

pub fn compile_profile_yaml(text: &str) -> Result<ConstraintSet> {
    if text.len() > MAX_PROFILE_BYTES {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            "profile document exceeds the 1 MiB safety limit",
        ));
    }
    if text.contains("\0") {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            "profile document contains a NUL byte",
        ));
    }
    let profile: ProfileDocument = yaml_serde::from_str(text).map_err(|error| {
        Error::new(
            ErrorCode::InputInvalid,
            format!("profile YAML is invalid: {error}"),
        )
    })?;
    if profile.schema != PROFILE_SCHEMA {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            "profile schema must be fitifact.profile/v1",
        ));
    }
    validate_profile_id(&profile.id)?;
    if profile.name.trim().is_empty() || profile.revision == 0 {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            "profile name and a positive revision are required",
        ));
    }
    if profile.last_verified.trim().is_empty() {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            "profile last_verified is required",
        ));
    }
    if profile.scope.is_none() {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            "profile must declare a scope",
        ));
    }
    let _ = profile.scope.and_then(|scope| scope.kind);
    if profile.constraints.is_empty() {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            "profile must declare at least one constraint",
        ));
    }
    if profile.sources.is_empty() {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            "profile must include source provenance",
        ));
    }
    let source_ids: std::collections::HashSet<&str> = profile
        .sources
        .iter()
        .map(|source| source.id.as_str())
        .collect();
    if profile.sources.iter().any(|source| {
        source.id.trim().is_empty()
            || source
                .observed_at
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
            || (source.url.as_deref().unwrap_or("").trim().is_empty()
                && source.note.as_deref().unwrap_or("").trim().is_empty())
    }) {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            "each profile source needs an id, observed_at, and a url or note",
        ));
    }
    if profile.constraints.iter().any(|item| {
        item.source
            .as_deref()
            .is_some_and(|id| !source_ids.contains(id))
    }) {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            "constraint source ids must match a declared profile source",
        ));
    }
    let hard = profile
        .constraints
        .into_iter()
        .enumerate()
        .map(|(index, item)| Constraint {
            id: item
                .id
                .filter(|id| !id.trim().is_empty())
                .unwrap_or_else(|| {
                    format!("{}-{}", item.field.as_str().replace('.', "-"), index + 1)
                }),
            field: item.field,
            op: item.op,
            value: item.value,
        })
        .collect();
    let preferences = match profile.preferences.and_then(|prefs| prefs.preserve) {
        Some(preserve) => Preferences {
            preserve_audio: preserve.audio.as_deref() != Some("low"),
            preserve_resolution: preserve.resolution.as_deref() != Some("low"),
        },
        None => Preferences::default(),
    };
    validate_and_normalize(ConstraintSet {
        schema: ConstraintsSchema,
        hard,
        preferences,
    })
}

pub fn validate_profile_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 128
        || id.contains("..")
        || id.contains('\\')
        || id.starts_with('/')
    {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            "profile id is not a stable slash path",
        ));
    }
    let mut parts = 0usize;
    for part in id.split('/') {
        parts += 1;
        if part.is_empty()
            || !part
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        {
            return Err(Error::new(
                ErrorCode::InputInvalid,
                "profile id must be lowercase slash segments",
            ));
        }
    }
    if parts < 2 {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            "profile id must contain a vendor/feature path",
        ));
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_profile(id: &str) -> Result<ConstraintSet> {
    validate_profile_id(id)?;
    let relative = PathBuf::from(id).with_extension("yaml");
    for root in profile_search_roots() {
        let path = root.join(&relative);
        if path.is_file() {
            let text = read_profile_text(&path)?;
            return compile_profile_yaml(&text);
        }
    }
    Err(Error::new(
        ErrorCode::InputInvalid,
        format!("profile '{id}' was not found in the local profiles directory"),
    ))
}

#[cfg(target_arch = "wasm32")]
pub fn load_profile(id: &str) -> Result<ConstraintSet> {
    compile_shipped_profile(id)
}

pub fn shipped_profile_ids() -> &'static [&'static str] {
    &[
        "discord/video-upload",
        "discord/video-upload-nitro-basic",
        "discord/video-upload-nitro",
        "discord/image-upload",
        "discord/image-upload-nitro-basic",
        "discord/image-upload-nitro",
        "gmail/attachment",
        "github/comment-image",
        "whatsapp/photo",
        "x/image",
        "slack/file-image",
        "jpeg/photo-upload",
        "generic/video-upload",
    ]
}

pub fn compile_shipped_profile(id: &str) -> Result<ConstraintSet> {
    validate_profile_id(id)?;
    let text = shipped_profile_yaml(id).ok_or_else(|| {
        Error::new(
            ErrorCode::InputInvalid,
            format!("profile '{id}' was not found in the local profiles directory"),
        )
    })?;
    compile_profile_yaml(text)
}

fn shipped_profile_yaml(id: &str) -> Option<&'static str> {
    match id {
        "discord/video-upload" => Some(include_str!("../../../profiles/discord/video-upload.yaml")),
        "discord/video-upload-nitro-basic" => Some(include_str!(
            "../../../profiles/discord/video-upload-nitro-basic.yaml"
        )),
        "discord/video-upload-nitro" => Some(include_str!(
            "../../../profiles/discord/video-upload-nitro.yaml"
        )),
        "discord/image-upload" => Some(include_str!("../../../profiles/discord/image-upload.yaml")),
        "discord/image-upload-nitro-basic" => Some(include_str!(
            "../../../profiles/discord/image-upload-nitro-basic.yaml"
        )),
        "discord/image-upload-nitro" => Some(include_str!(
            "../../../profiles/discord/image-upload-nitro.yaml"
        )),
        "gmail/attachment" => Some(include_str!("../../../profiles/gmail/attachment.yaml")),
        "github/comment-image" => Some(include_str!("../../../profiles/github/comment-image.yaml")),
        "whatsapp/photo" => Some(include_str!("../../../profiles/whatsapp/photo.yaml")),
        "x/image" => Some(include_str!("../../../profiles/x/image.yaml")),
        "slack/file-image" => Some(include_str!("../../../profiles/slack/file-image.yaml")),
        "jpeg/photo-upload" => Some(include_str!("../../../profiles/jpeg/photo-upload.yaml")),
        "generic/video-upload" => Some(include_str!("../../../profiles/generic/video-upload.yaml")),
        _ => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_profile_text(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).map_err(|_| {
        Error::new(
            ErrorCode::InputInvalid,
            format!("cannot read profile {}", path.display()),
        )
    })?;
    if bytes.len() > MAX_PROFILE_BYTES {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            "profile document exceeds the 1 MiB safety limit",
        ));
    }
    String::from_utf8(bytes).map_err(|_| {
        Error::new(
            ErrorCode::InputInvalid,
            "profile document is not valid UTF-8",
        )
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn profile_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(explicit) = std::env::var("FITIFACT_PROFILES") {
        roots.push(PathBuf::from(explicit));
    }
    if let Ok(exe) = std::env::current_exe() {
        roots.extend(exe.parent().map(|dir| dir.join("profiles")));
    }
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir = cwd;
        for _ in 0..6 {
            roots.push(dir.join("profiles"));
            if !dir.pop() {
                break;
            }
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    roots.push(manifest.join("../../profiles"));
    roots
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraints::Field;

    #[test]
    fn compile_profile_yaml_emits_a_constraint_set() {
        let yaml = r#"
schema: fitifact.profile/v1
id: example/video-upload
name: Example
revision: 1
last_verified: 2026-08-25
scope:
  kind: web-upload
constraints:
  - id: container
    field: media.container
    op: in
    value: [mp4]
    source: docs
  - id: bytes
    field: file.bytes
    op: lte
    value: 25000000
    source: docs
sources:
  - id: docs
    type: official-doc
    url: https://example.invalid/docs
    observed_at: 2026-08-25
"#;
        let set = compile_profile_yaml(yaml).unwrap();
        assert!(
            set.hard
                .iter()
                .any(|item| item.field == Field::MediaContainer)
        );
        assert!(set.hard.iter().any(|item| item.field == Field::FileBytes));
    }

    #[test]
    fn rejects_a_profile_without_provenance() {
        let yaml = r#"
schema: fitifact.profile/v1
id: example/video-upload
name: Example
revision: 1
last_verified: 2026-08-25
scope:
  kind: web-upload
constraints:
  - field: media.container
    op: in
    value: [mp4]
"#;
        assert!(compile_profile_yaml(yaml).is_err());
    }

    #[test]
    fn shipped_profiles_compile_to_constraint_sets() {
        for id in shipped_profile_ids() {
            let set = compile_shipped_profile(id).expect(id);
            assert!(!set.hard.is_empty(), "{id} must emit hard constraints");
        }
        let discord = compile_shipped_profile("discord/video-upload").unwrap();
        assert_eq!(
            crate::media_fit::file_bytes_limit(&discord),
            Some(20_000_000)
        );
        assert!(
            discord
                .hard
                .iter()
                .any(|item| item.field == Field::MediaContainer)
        );
        assert!(
            discord
                .hard
                .iter()
                .any(|item| item.field == Field::MediaVideoCodec)
        );
        assert_eq!(
            crate::media_fit::file_bytes_limit(
                &compile_shipped_profile("discord/video-upload-nitro-basic").unwrap()
            ),
            Some(50_000_000)
        );
        assert_eq!(
            crate::media_fit::file_bytes_limit(
                &compile_shipped_profile("discord/video-upload-nitro").unwrap()
            ),
            Some(500_000_000)
        );
        let image = compile_shipped_profile("discord/image-upload").unwrap();
        assert!(
            image
                .hard
                .iter()
                .any(|item| item.field == Field::ImageFormat)
        );
        assert_eq!(crate::media_fit::file_bytes_limit(&image), Some(20_000_000));
        assert_eq!(
            crate::media_fit::file_bytes_limit(
                &compile_shipped_profile("github/comment-image").unwrap()
            ),
            Some(10_000_000)
        );
        assert_eq!(
            crate::media_fit::file_bytes_limit(&compile_shipped_profile("whatsapp/photo").unwrap()),
            Some(16_000_000)
        );
        assert_eq!(
            crate::media_fit::file_bytes_limit(&compile_shipped_profile("x/image").unwrap()),
            Some(5_000_000)
        );
        assert_eq!(
            crate::media_fit::file_bytes_limit(
                &compile_shipped_profile("slack/file-image").unwrap()
            ),
            Some(1_000_000_000)
        );
    }
}
