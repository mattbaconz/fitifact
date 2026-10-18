#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::constraints::{
    Constraint, ConstraintSet, ConstraintValue, Field, Operator, Preferences,
    validate_and_normalize,
};
use crate::contract::ConstraintsSchema;
use crate::error::{Error, ErrorCode, Result};

const PROFILE_SCHEMA: &str = "fitifact.profile/v1";
const MAX_PROFILE_BYTES: usize = 1024 * 1024;
const SOURCE_TYPES: [&str; 7] = [
    "official-doc",
    "official-api",
    "observed-page",
    "acceptance-test",
    "user-config",
    "community",
    "fitifact-rule",
];

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
    #[serde(default)]
    history: Vec<ProfileChange>,
}

/// One published revision of a profile, listed newest first.
#[derive(Debug, Deserialize, Serialize)]
struct ProfileChange {
    revision: u64,
    date: String,
    change: String,
}

#[derive(Debug, Deserialize)]
struct ProfileConstraint {
    id: Option<String>,
    field: Field,
    op: Operator,
    value: ConstraintValue,
    #[serde(default)]
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
    #[serde(default, rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    observed_at: Option<String>,
    #[serde(default)]
    note: Option<String>,
}

pub fn compile_profile_yaml(text: &str) -> Result<ConstraintSet> {
    let profile = parse_profile(text)?;
    let hard = profile
        .constraints
        .into_iter()
        .enumerate()
        .map(|(index, item)| Constraint {
            id: rule_id(item.id.as_deref(), item.field, index),
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

fn rule_id(id: Option<&str>, field: Field, index: usize) -> String {
    id.filter(|id| !id.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}-{}", field.as_str().replace('.', "-"), index + 1))
}

fn parse_profile(text: &str) -> Result<ProfileDocument> {
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
    let _ = profile.scope.as_ref().and_then(|scope| scope.kind.as_ref());
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
    if let Some(kind) = profile
        .sources
        .iter()
        .filter_map(|source| source.kind.as_deref())
        .find(|kind| !SOURCE_TYPES.contains(kind))
    {
        return Err(Error::new(
            ErrorCode::InputInvalid,
            format!("profile source type '{kind}' is not recognized"),
        ));
    }
    let mut newer = None;
    for entry in &profile.history {
        let in_order = match newer {
            None => entry.revision == profile.revision,
            Some(newer) => entry.revision < newer,
        };
        if !in_order || entry.date.trim().is_empty() || entry.change.trim().is_empty() {
            return Err(Error::new(
                ErrorCode::InputInvalid,
                "profile history must list revisions newest first, starting at the current revision, each with a date and a change",
            ));
        }
        newer = Some(entry.revision);
    }
    Ok(profile)
}

/// Trust follows from where each rule comes from; profiles never declare it.
fn derived_trust(profile: &ProfileDocument) -> &'static str {
    let kinds: Vec<&str> = profile
        .constraints
        .iter()
        .map(|rule| {
            profile
                .sources
                .iter()
                .find(|source| rule.source.as_deref() == Some(source.id.as_str()))
                .and_then(|source| source.kind.as_deref())
                .unwrap_or("")
        })
        .collect();
    let documented = |kind: &&str| matches!(*kind, "official-doc" | "official-api");
    if kinds.contains(&"user-config") {
        "starter"
    } else if kinds.iter().all(documented) {
        "documented"
    } else if kinds
        .iter()
        .all(|kind| documented(kind) || *kind == "fitifact-rule")
    {
        "documented-with-rules"
    } else {
        "observed"
    }
}

/// Every shipped profile as one JSON document, served as `registry.json` and read by the
/// web UI for rule provenance and age.
pub fn shipped_registry_json() -> Result<String> {
    let mut profiles = Vec::new();
    for id in shipped_profile_ids() {
        let text = shipped_profile_yaml(id).ok_or_else(|| {
            Error::new(
                ErrorCode::InputInvalid,
                format!("profile '{id}' is not embedded"),
            )
        })?;
        let profile = parse_profile(text)?;
        let rules: Vec<_> = profile
            .constraints
            .iter()
            .enumerate()
            .map(|(index, rule)| {
                let origin = profile
                    .sources
                    .iter()
                    .find(|source| rule.source.as_deref() == Some(source.id.as_str()));
                serde_json::json!({
                    "id": rule_id(rule.id.as_deref(), rule.field, index),
                    "field": rule.field,
                    "op": rule.op,
                    "value": rule.value,
                    "source": rule.source,
                    "source_type": origin.and_then(|source| source.kind.as_deref()),
                    "source_url": origin.and_then(|source| source.url.as_deref()),
                    "observed_at": origin.and_then(|source| source.observed_at.as_deref()),
                })
            })
            .collect();
        let sources: Vec<_> = profile
            .sources
            .iter()
            .map(|source| {
                serde_json::json!({
                    "id": source.id,
                    "type": source.kind,
                    "url": source.url,
                    "observed_at": source.observed_at,
                    "note": source.note,
                })
            })
            .collect();
        profiles.push(serde_json::json!({
            "id": profile.id,
            "name": profile.name,
            "revision": profile.revision,
            "last_verified": profile.last_verified,
            "trust": derived_trust(&profile),
            "rules": rules,
            "sources": sources,
            "history": profile.history,
        }));
    }
    let registry = serde_json::json!({
        "schema": "fitifact.registry/v1",
        "profiles": profiles,
    });
    serde_json::to_string_pretty(&registry)
        .map(|text| text + "\n")
        .map_err(|error| Error::new(ErrorCode::InputInvalid, error.to_string()))
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
        "discord/file-upload",
        "discord/file-upload-nitro-basic",
        "discord/file-upload-nitro",
        "gmail/attachment",
        "github/comment-image",
        "whatsapp/photo",
        "x/image",
        "slack/file-image",
        "slack/file-video",
        "slack/file-upload",
        "jpeg/photo-upload",
        "generic/video-upload",
        "generic/web-image",
        "whatsapp/video",
        "whatsapp/audio",
        "x/video",
        "github/comment-video",
        "anthropic/vision",
        "anthropic/vision-cloud",
        "openai/vision",
        "openai/transcription",
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
        "discord/file-upload" => Some(include_str!("../../../profiles/discord/file-upload.yaml")),
        "discord/file-upload-nitro-basic" => Some(include_str!(
            "../../../profiles/discord/file-upload-nitro-basic.yaml"
        )),
        "discord/file-upload-nitro" => Some(include_str!(
            "../../../profiles/discord/file-upload-nitro.yaml"
        )),
        "gmail/attachment" => Some(include_str!("../../../profiles/gmail/attachment.yaml")),
        "github/comment-image" => Some(include_str!("../../../profiles/github/comment-image.yaml")),
        "whatsapp/photo" => Some(include_str!("../../../profiles/whatsapp/photo.yaml")),
        "x/image" => Some(include_str!("../../../profiles/x/image.yaml")),
        "slack/file-image" => Some(include_str!("../../../profiles/slack/file-image.yaml")),
        "slack/file-video" => Some(include_str!("../../../profiles/slack/file-video.yaml")),
        "slack/file-upload" => Some(include_str!("../../../profiles/slack/file-upload.yaml")),
        "jpeg/photo-upload" => Some(include_str!("../../../profiles/jpeg/photo-upload.yaml")),
        "generic/video-upload" => Some(include_str!("../../../profiles/generic/video-upload.yaml")),
        "generic/web-image" => Some(include_str!("../../../profiles/generic/web-image.yaml")),
        "whatsapp/video" => Some(include_str!("../../../profiles/whatsapp/video.yaml")),
        "whatsapp/audio" => Some(include_str!("../../../profiles/whatsapp/audio.yaml")),
        "x/video" => Some(include_str!("../../../profiles/x/video.yaml")),
        "github/comment-video" => Some(include_str!("../../../profiles/github/comment-video.yaml")),
        "anthropic/vision" => Some(include_str!("../../../profiles/anthropic/vision.yaml")),
        "anthropic/vision-cloud" => Some(include_str!(
            "../../../profiles/anthropic/vision-cloud.yaml"
        )),
        "openai/vision" => Some(include_str!("../../../profiles/openai/vision.yaml")),
        "openai/transcription" => Some(include_str!(
            "../../../profiles/openai/transcription.yaml"
        )),
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
            Some(1_000_000_000)
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
        assert_eq!(
            crate::media_fit::file_bytes_limit(
                &compile_shipped_profile("github/comment-video").unwrap()
            ),
            Some(10_000_000)
        );
        assert_eq!(
            crate::media_fit::file_bytes_limit(&compile_shipped_profile("whatsapp/video").unwrap()),
            Some(64_000_000)
        );
        assert_eq!(
            crate::media_fit::file_bytes_limit(&compile_shipped_profile("x/video").unwrap()),
            Some(512_000_000)
        );
        assert_eq!(
            crate::media_fit::file_bytes_limit(
                &compile_shipped_profile("discord/file-upload").unwrap()
            ),
            Some(20_000_000)
        );
        assert_eq!(
            crate::media_fit::file_bytes_limit(&compile_shipped_profile("whatsapp/audio").unwrap()),
            Some(2_000_000_000)
        );
        assert_eq!(
            crate::media_fit::file_bytes_limit(
                &compile_shipped_profile("discord/image-upload-nitro").unwrap()
            ),
            Some(20_000_000)
        );
        assert_eq!(
            crate::media_fit::file_bytes_limit(
                &compile_shipped_profile("slack/file-upload").unwrap()
            ),
            Some(1_000_000_000)
        );
        assert_eq!(
            crate::media_fit::file_bytes_limit(
                &compile_shipped_profile("generic/web-image").unwrap()
            ),
            Some(8_000_000)
        );
        // Base64 ceilings fold into the raw ceiling: 3 bytes per 4 characters.
        assert_eq!(
            crate::media_fit::file_bytes_limit(
                &compile_shipped_profile("anthropic/vision").unwrap()
            ),
            Some(7_500_000)
        );
        assert_eq!(
            crate::media_fit::file_bytes_limit(
                &compile_shipped_profile("anthropic/vision-cloud").unwrap()
            ),
            Some(3_750_000)
        );
        let transcription = compile_shipped_profile("openai/transcription").unwrap();
        assert_eq!(
            crate::media_fit::file_bytes_limit(&transcription),
            Some(25_000_000)
        );
        assert!(transcription.hard.iter().any(|item| {
            item.field == Field::MediaContainer
                && item.value.as_text_list() == ["mp3", "mp4", "wav", "webm"]
        }));
    }

    #[test]
    fn every_shipped_rule_names_a_typed_source() {
        for id in shipped_profile_ids() {
            let profile = parse_profile(shipped_profile_yaml(id).unwrap()).unwrap();
            for rule in &profile.constraints {
                let kind = profile
                    .sources
                    .iter()
                    .find(|source| rule.source.as_deref() == Some(source.id.as_str()))
                    .and_then(|source| source.kind.as_deref());
                assert!(
                    kind.is_some(),
                    "{id} rule {:?} lacks a typed source",
                    rule.id
                );
            }
        }
    }

    #[test]
    fn every_shipped_profile_records_its_change_history() {
        for id in shipped_profile_ids() {
            let profile = parse_profile(shipped_profile_yaml(id).unwrap()).unwrap();
            let oldest = profile.history.last().map(|entry| entry.revision);
            assert_eq!(oldest, Some(1), "{id} history must reach revision 1");
        }
    }

    #[test]
    fn rejects_history_that_skips_the_current_revision() {
        let base = "schema: fitifact.profile/v1\nid: example/x\nname: Example\nrevision: 2\nlast_verified: 2026-10-17\nscope:\n  kind: web-upload\nconstraints:\n  - field: file.bytes\n    op: lte\n    value: 10\n    source: docs\nsources:\n  - id: docs\n    type: official-doc\n    url: https://example.invalid\n    observed_at: 2026-10-17\nhistory:\n";
        let newer = "  - revision: 2\n    date: 2026-10-17\n    change: Raised the cap.\n";
        let first = "  - revision: 1\n    date: 2026-08-26\n    change: First published.\n";
        let valid = format!("{base}{newer}{first}");
        assert!(compile_profile_yaml(&valid).is_ok());
        let missing_current = format!("{base}{first}");
        assert!(compile_profile_yaml(&missing_current).is_err());
        let oldest_first = format!("{base}{first}{newer}");
        assert!(compile_profile_yaml(&oldest_first).is_err());
    }

    #[test]
    fn rejects_an_unknown_source_type() {
        let yaml = "schema: fitifact.profile/v1\nid: example/x\nname: Example\nrevision: 1\nlast_verified: 2026-10-17\nscope:\n  kind: web-upload\nconstraints:\n  - field: file.bytes\n    op: lte\n    value: 10\n    source: docs\nsources:\n  - id: docs\n    type: rumor\n    url: https://example.invalid\n    observed_at: 2026-10-17\n";
        assert!(compile_profile_yaml(yaml).is_err());
    }

    #[test]
    fn trust_is_derived_from_rule_sources() {
        let trust = |id| derived_trust(&parse_profile(shipped_profile_yaml(id).unwrap()).unwrap());
        assert_eq!(trust("gmail/attachment"), "documented");
        assert_eq!(trust("discord/video-upload"), "documented-with-rules");
        assert_eq!(trust("generic/video-upload"), "starter");
        assert_eq!(trust("anthropic/vision"), "documented");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn committed_registry_matches_the_shipped_profiles() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/public/registry.json");
        let generated = shipped_registry_json().unwrap();
        if std::env::var_os("FITIFACT_BLESS").is_some() {
            std::fs::write(&path, &generated).unwrap();
        }
        let committed = std::fs::read_to_string(&path)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        assert!(
            committed == generated,
            "web/public/registry.json is out of date; regenerate it with FITIFACT_BLESS=1 cargo test -p fitifact --lib registry"
        );
    }
}
