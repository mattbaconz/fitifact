use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::artifact::{Artifact, Container};
use crate::capability::{CapabilityCatalog, default_catalog};
use crate::check::{CompatibilityReport, check};
use crate::constraints::ConstraintSet;
use crate::contract::AdaptationSchema;
use crate::error::{Error, ErrorCode, ErrorEnvelope, Result};
use crate::inspect::Inspector;
use crate::plan::{Plan, PlanOutcome, plan};
use crate::report::{Explanation, explain_plan};
use crate::runtime::{ExecutionContext, TransformProvider, execute};
use crate::validate::{ValidationReport, ValidationStatus, validate_adaptation};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdaptationStatus {
    Compatible,
    Adapted,
    CannotSatisfy,
    Failed,
}

pub struct AdaptRequest<'a> {
    pub input: &'a Path,
    pub constraints: ConstraintSet,
    pub output: Option<PathBuf>,
    pub catalog: Option<&'a CapabilityCatalog>,
    pub inspector: &'a dyn Inspector,
    pub provider: &'a dyn TransformProvider,
    pub execution: ExecutionContext,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdaptationResult {
    pub schema: AdaptationSchema,
    pub status: AdaptationStatus,
    pub original: PathBuf,
    pub output: Option<PathBuf>,
    pub artifact: Artifact,
    pub report: CompatibilityReport,
    pub plan: Option<Plan>,
    pub validation: Option<ValidationReport>,
    pub explanation: Explanation,
    pub error: Option<ErrorEnvelope>,
}

pub fn adapt(request: AdaptRequest<'_>) -> Result<AdaptationResult> {
    let original = request.input.to_path_buf();
    let artifact = request.inspector.inspect(request.input)?;
    let report = check(&artifact, &request.constraints);
    let catalog = request.catalog.cloned().unwrap_or_else(default_catalog);
    let outcome = plan(&artifact, &request.constraints, &catalog);
    let explanation = explain_plan(&artifact, &report, &outcome);

    match outcome {
        PlanOutcome::Compatible { .. } => Ok(AdaptationResult {
            schema: AdaptationSchema,
            status: AdaptationStatus::Compatible,
            original,
            output: None,
            artifact,
            report,
            plan: None,
            validation: None,
            explanation,
            error: None,
        }),
        PlanOutcome::CannotSatisfy { blocking, .. } => Ok(AdaptationResult {
            schema: AdaptationSchema,
            status: AdaptationStatus::CannotSatisfy,
            original,
            output: None,
            artifact,
            report,
            plan: None,
            validation: None,
            explanation: Explanation {
                summary: explanation.summary,
                details: {
                    let mut d = explanation.details;
                    if !blocking.is_empty() {
                        d.push(format!(
                            "Blocking: {}",
                            blocking
                                .iter()
                                .map(|reason| reason.message.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                    d
                },
            },
            error: Some({
                let mut envelope = ErrorEnvelope::from(Error::new(
                    ErrorCode::NoValidPlan,
                    "no acceptable plan satisfies the hard constraints",
                ));
                envelope.details.insert(
                    "blocking".into(),
                    serde_json::to_value(&blocking).unwrap_or(serde_json::Value::Null),
                );
                envelope
            }),
        }),
        PlanOutcome::Planned { plan, .. } => {
            let explicit_output = request.output.is_some();
            let output = match request.output {
                Some(path) => path,
                None => match default_output(&original, &plan, artifact.container.as_ref()) {
                    Ok(path) => path,
                    Err(err) => {
                        return Ok(failed(
                            original,
                            artifact,
                            report,
                            Some(plan),
                            explanation,
                            err,
                            None,
                        ));
                    }
                },
            };
            if explicit_output && path_is_occupied(&output) {
                return Ok(failed(
                    original,
                    artifact,
                    report,
                    Some(plan),
                    explanation,
                    Error::new(
                        ErrorCode::SecurityBlocked,
                        "the requested output already exists; choose a new output path",
                    ),
                    None,
                ));
            }
            let mut staged = match StageWorkspace::new(&output) {
                Ok(staged) => staged,
                Err(err) => {
                    return Ok(failed(
                        original,
                        artifact,
                        report,
                        Some(plan),
                        explanation,
                        err,
                        None,
                    ));
                }
            };
            if let Err(err) = execute(
                request.provider,
                request.input,
                staged.path(),
                &plan,
                &artifact,
                &request.execution,
            ) {
                let _ = staged.claim_created_file();
                return Ok(failed(
                    original,
                    artifact,
                    report,
                    Some(plan),
                    explanation,
                    err,
                    None,
                ));
            }
            if let Err(err) = staged.claim_created_file() {
                return Ok(failed(
                    original,
                    artifact,
                    report,
                    Some(plan),
                    explanation,
                    err,
                    None,
                ));
            }
            let source_hashes =
                match request
                    .provider
                    .stream_hashes(request.input, &artifact, &request.execution)
                {
                    Ok(hashes) => hashes,
                    Err(err) => {
                        return Ok(failed(
                            original,
                            artifact,
                            report,
                            Some(plan),
                            explanation,
                            err,
                            None,
                        ));
                    }
                };
            let output_hashes =
                match request
                    .provider
                    .stream_hashes(staged.path(), &artifact, &request.execution)
                {
                    Ok(hashes) => hashes,
                    Err(err) => {
                        return Ok(failed(
                            original,
                            artifact,
                            report,
                            Some(plan),
                            explanation,
                            err,
                            None,
                        ));
                    }
                };
            match validate_adaptation(
                staged.path(),
                &request.constraints,
                request.inspector,
                &artifact,
                &plan,
                &source_hashes,
                &output_hashes,
            ) {
                Ok(staged_validation) if staged_validation.status == ValidationStatus::Pass => {
                    let mut published = match staged.publish(&output) {
                        Ok(published) => published,
                        Err(err) => {
                            return Ok(failed(
                                original,
                                artifact,
                                report,
                                Some(plan),
                                explanation,
                                err,
                                Some(staged_validation),
                            ));
                        }
                    };
                    let final_hashes =
                        match request
                            .provider
                            .stream_hashes(&output, &artifact, &request.execution)
                        {
                            Ok(hashes) => hashes,
                            Err(err) => {
                                staged.release_protection();
                                let _ = published.remove_if_owned();
                                return Ok(failed(
                                    original,
                                    artifact,
                                    report,
                                    Some(plan),
                                    explanation,
                                    err,
                                    Some(staged_validation),
                                ));
                            }
                        };
                    let final_validation = validate_adaptation(
                        &output,
                        &request.constraints,
                        request.inspector,
                        &artifact,
                        &plan,
                        &source_hashes,
                        &final_hashes,
                    );
                    let identity_check = published.verify();
                    match (final_validation, identity_check) {
                        (Ok(validation), Ok(())) if validation.status == ValidationStatus::Pass => {
                            published.release_protection();
                            let cleanup = staged.cleanup_owned();
                            let mut explanation = explanation;
                            if !cleanup.complete {
                                explanation.details.push(
                                    "The validated output was published, but its private staging workspace could not be fully removed; no published or unverified path was deleted."
                                        .into(),
                                );
                            }
                            Ok(AdaptationResult {
                                schema: AdaptationSchema,
                                status: AdaptationStatus::Adapted,
                                original,
                                output: Some(output),
                                artifact,
                                report,
                                plan: Some(plan),
                                validation: Some(validation),
                                explanation,
                                error: None,
                            })
                        }
                        (Ok(validation), identity) => {
                            staged.release_protection();
                            let removed = published.remove_if_owned();
                            let err = identity.err().unwrap_or_else(|| {
                                Error::new(
                                    ErrorCode::ValidationFailed,
                                    "the published output failed fresh validation",
                                )
                            });
                            let mut explanation = explanation;
                            if !removed {
                                explanation.details.push(
                                    "The untrusted published path was left in place because its identity could not be proven for safe cleanup."
                                        .into(),
                                );
                            }
                            Ok(failed(
                                original,
                                artifact,
                                report,
                                Some(plan),
                                explanation,
                                err,
                                Some(validation),
                            ))
                        }
                        (Err(err), _) => {
                            staged.release_protection();
                            let removed = published.remove_if_owned();
                            let mut explanation = explanation;
                            if !removed {
                                explanation.details.push(
                                    "The untrusted published path was left in place because its identity could not be proven for safe cleanup."
                                        .into(),
                                );
                            }
                            Ok(failed(
                                original,
                                artifact,
                                report,
                                Some(plan),
                                explanation,
                                err,
                                None,
                            ))
                        }
                    }
                }
                Ok(validation) => Ok(failed(
                    original,
                    artifact,
                    report,
                    Some(plan),
                    explanation,
                    Error::new(
                        ErrorCode::ValidationFailed,
                        "output did not satisfy the same hard constraints",
                    ),
                    Some(validation),
                )),
                Err(err) => Ok(failed(
                    original,
                    artifact,
                    report,
                    Some(plan),
                    explanation,
                    err,
                    None,
                )),
            }
        }
    }
}

fn failed(
    original: PathBuf,
    artifact: Artifact,
    report: CompatibilityReport,
    plan: Option<Plan>,
    explanation: Explanation,
    err: Error,
    validation: Option<ValidationReport>,
) -> AdaptationResult {
    AdaptationResult {
        schema: AdaptationSchema,
        status: AdaptationStatus::Failed,
        original,
        output: None,
        artifact,
        report,
        plan,
        validation,
        explanation,
        error: Some(ErrorEnvelope::from(err)),
    }
}

fn default_output(input: &Path, plan: &Plan, current: Option<&Container>) -> Result<PathBuf> {
    default_output_with_limit(input, plan, current, 10_000, path_is_occupied)
}

fn default_output_with_limit(
    input: &Path,
    plan: &Plan,
    current: Option<&Container>,
    limit: u32,
    occupied: impl Fn(&Path) -> bool,
) -> Result<PathBuf> {
    let container = plan
        .steps
        .iter()
        .rev()
        .find_map(|step| step.target.container.clone())
        .or_else(|| current.cloned())
        .unwrap_or(Container::Mp4);
    let ext = match container {
        Container::Mov => "mov",
        Container::Webm => "webm",
        Container::Mkv => "mkv",
        _ => "mp4",
    };
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    for suffix in 1..=limit {
        let name = if suffix == 1 {
            format!("{stem}.fitifact.{ext}")
        } else {
            format!("{stem}.fitifact.{suffix}.{ext}")
        };
        let candidate = parent.join(name);
        if !occupied(&candidate) {
            return Ok(candidate);
        }
    }
    Err(Error::new(
        ErrorCode::ExecutionFailed,
        "could not allocate an unused default output name",
    ))
}

fn path_is_occupied(path: &Path) -> bool {
    match std::fs::symlink_metadata(path) {
        Ok(_) => true,
        Err(error) => error.kind() != std::io::ErrorKind::NotFound,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    volume: u64,
    file: u64,
}

#[cfg(windows)]
fn file_identity(file: &File) -> Option<FileIdentity> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };

    let mut information = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::zeroed();
    // SAFETY: `file` owns a live Windows handle and the output pointer refers to
    // initialized, writable storage for the exact structure the API fills.
    let success =
        unsafe { GetFileInformationByHandle(file.as_raw_handle(), information.as_mut_ptr()) };
    if success == 0 {
        return None;
    }
    // SAFETY: a nonzero API result guarantees that the structure was initialized.
    let information = unsafe { information.assume_init() };
    Some(FileIdentity {
        volume: u64::from(information.dwVolumeSerialNumber),
        file: (u64::from(information.nFileIndexHigh) << 32) | u64::from(information.nFileIndexLow),
    })
}

#[cfg(unix)]
fn file_identity(file: &File) -> Option<FileIdentity> {
    use std::os::unix::fs::MetadataExt;

    let metadata = file.metadata().ok()?;
    Some(FileIdentity {
        volume: metadata.dev(),
        file: metadata.ino(),
    })
}

#[cfg(not(any(windows, unix)))]
fn file_identity(_file: &File) -> Option<FileIdentity> {
    None
}

fn path_identity(path: &Path, expected_directory: bool) -> Option<FileIdentity> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink()
        || (expected_directory && !metadata.is_dir())
        || (!expected_directory && !metadata.is_file())
    {
        return None;
    }
    identity_read(path, expected_directory)
        .ok()
        .and_then(|file| file_identity(&file))
}

#[cfg(windows)]
fn identity_read(path: &Path, directory: bool) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_SHARE_ALL: u32 = 0x0000_0001 | 0x0000_0002 | 0x0000_0004;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    let mut options = OpenOptions::new();
    options.read(true).share_mode(FILE_SHARE_ALL);
    if directory {
        options.custom_flags(FILE_FLAG_BACKUP_SEMANTICS);
    }
    options.open(path)
}

#[cfg(not(windows))]
fn identity_read(path: &Path, _directory: bool) -> std::io::Result<File> {
    OpenOptions::new().read(true).open(path)
}

fn open_protected_file(path: &Path) -> Result<(File, FileIdentity)> {
    if path_identity(path, false).is_none() {
        return Err(Error::new(
            ErrorCode::SecurityBlocked,
            "the staging output is not a regular owned file",
        ));
    }
    let file = protected_read(path).map_err(|_| {
        Error::new(
            ErrorCode::SecurityBlocked,
            "the staging output could not be protected for validation",
        )
    })?;
    let identity = file_identity(&file).ok_or_else(|| {
        Error::new(
            ErrorCode::SecurityBlocked,
            "this platform cannot establish a stable file identity",
        )
    })?;
    if path_identity(path, false) != Some(identity) {
        return Err(Error::new(
            ErrorCode::SecurityBlocked,
            "the staging output changed while it was being claimed",
        ));
    }
    Ok((file, identity))
}

#[cfg(windows)]
fn protected_read(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_SHARE_READ: u32 = 0x0000_0001;
    OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(path)
}

#[cfg(not(windows))]
fn protected_read(path: &Path) -> std::io::Result<File> {
    OpenOptions::new().read(true).open(path)
}

#[derive(Debug, Clone, Copy)]
struct CleanupOutcome {
    complete: bool,
}

#[derive(Debug)]
struct PublishedOutput {
    path: PathBuf,
    identity: FileIdentity,
    protection: Option<File>,
}

impl PublishedOutput {
    fn verify(&self) -> Result<()> {
        let handle_identity = self.protection.as_ref().and_then(file_identity);
        if handle_identity != Some(self.identity)
            || path_identity(&self.path, false) != Some(self.identity)
        {
            return Err(Error::new(
                ErrorCode::SecurityBlocked,
                "the published output changed during validation",
            ));
        }
        Ok(())
    }

    fn release_protection(&mut self) {
        self.protection.take();
    }

    fn remove_if_owned(&mut self) -> bool {
        if self.verify().is_err() {
            return false;
        }
        self.release_protection();
        if path_identity(&self.path, false) != Some(self.identity) {
            return false;
        }
        std::fs::remove_file(&self.path).is_ok()
    }
}

#[derive(Debug)]
struct StageWorkspace {
    directory: PathBuf,
    directory_identity: FileIdentity,
    path: PathBuf,
    file_identity: Option<FileIdentity>,
    protection: Option<File>,
}

impl StageWorkspace {
    fn new(destination: &Path) -> Result<Self> {
        let parent = destination
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let metadata = std::fs::metadata(parent).map_err(|_| {
            Error::new(
                ErrorCode::ExecutionFailed,
                "the output directory is unavailable or not writable",
            )
        })?;
        if !metadata.is_dir() {
            return Err(Error::new(
                ErrorCode::ExecutionFailed,
                "the output destination is not a directory",
            ));
        }
        let extension = destination
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("mp4");
        for _ in 0..100_u32 {
            let mut random = [0_u8; 16];
            getrandom::fill(&mut random).map_err(|_| {
                Error::new(
                    ErrorCode::ExecutionFailed,
                    "secure randomness was unavailable for staging allocation",
                )
            })?;
            let token = u128::from_ne_bytes(random);
            let directory = parent.join(format!(".fitifact-stage-{token:032x}"));
            match std::fs::create_dir(&directory) {
                Ok(()) => {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = std::fs::set_permissions(
                            &directory,
                            std::fs::Permissions::from_mode(0o700),
                        );
                    }
                    let directory_identity = path_identity(&directory, true).ok_or_else(|| {
                        Error::new(
                            ErrorCode::SecurityBlocked,
                            "the staging workspace identity could not be established",
                        )
                    })?;
                    let path = directory.join(format!("artifact.{extension}"));
                    return Ok(Self {
                        directory,
                        directory_identity,
                        path,
                        file_identity: None,
                        protection: None,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => {
                    return Err(Error::new(
                        ErrorCode::ExecutionFailed,
                        "the staging workspace could not be created safely",
                    ));
                }
            }
        }
        Err(Error::new(
            ErrorCode::ExecutionFailed,
            "could not allocate a unique staging path",
        ))
    }

    fn path(&self) -> &Path {
        &self.path
    }

    #[cfg(test)]
    fn directory(&self) -> &Path {
        &self.directory
    }

    fn verify_directory(&self) -> Result<()> {
        if path_identity(&self.directory, true) != Some(self.directory_identity) {
            return Err(Error::new(
                ErrorCode::SecurityBlocked,
                "the staging workspace changed unexpectedly",
            ));
        }
        Ok(())
    }

    fn claim_created_file(&mut self) -> Result<()> {
        self.verify_directory()?;
        if self.file_identity.is_some() {
            return self.verify_claim();
        }
        let (file, identity) = open_protected_file(&self.path)?;
        self.file_identity = Some(identity);
        self.protection = Some(file);
        Ok(())
    }

    fn verify_claim(&self) -> Result<()> {
        self.verify_directory()?;
        let expected = self.file_identity.ok_or_else(|| {
            Error::new(
                ErrorCode::SecurityBlocked,
                "the staging output has not been claimed",
            )
        })?;
        let handle_identity = self.protection.as_ref().and_then(file_identity);
        if handle_identity != Some(expected) || path_identity(&self.path, false) != Some(expected) {
            return Err(Error::new(
                ErrorCode::SecurityBlocked,
                "the staging output changed during validation",
            ));
        }
        Ok(())
    }

    fn release_protection(&mut self) {
        self.protection.take();
    }

    fn publish(&self, destination: &Path) -> Result<PublishedOutput> {
        self.verify_claim()?;
        std::fs::hard_link(&self.path, destination).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                Error::new(
                    ErrorCode::SecurityBlocked,
                    "the output appeared before persistence; no file was overwritten",
                )
            } else {
                Error::new(
                    ErrorCode::ExecutionFailed,
                    "the validated output could not be persisted without overwriting",
                )
            }
        })?;
        let (protection, identity) = open_protected_file(destination)?;
        let expected = self.file_identity.ok_or_else(|| {
            Error::new(
                ErrorCode::SecurityBlocked,
                "the staging output has not been claimed",
            )
        })?;
        if identity != expected || path_identity(&self.path, false) != Some(expected) {
            return Err(Error::new(
                ErrorCode::SecurityBlocked,
                "the published output was not the validated staging file",
            ));
        }
        Ok(PublishedOutput {
            path: destination.to_path_buf(),
            identity,
            protection: Some(protection),
        })
    }

    fn cleanup_owned(&mut self) -> CleanupOutcome {
        if self.verify_directory().is_err() {
            return CleanupOutcome { complete: false };
        }
        if self.path.exists() {
            let Some(expected) = self.file_identity else {
                return CleanupOutcome { complete: false };
            };
            if path_identity(&self.path, false) != Some(expected) {
                return CleanupOutcome { complete: false };
            }
            self.release_protection();
            if path_identity(&self.path, false) != Some(expected)
                || std::fs::remove_file(&self.path).is_err()
            {
                return CleanupOutcome { complete: false };
            }
        } else {
            self.release_protection();
        }
        if self.verify_directory().is_err() {
            return CleanupOutcome { complete: false };
        }
        CleanupOutcome {
            complete: std::fs::remove_dir(&self.directory).is_ok(),
        }
    }
}

impl Drop for StageWorkspace {
    fn drop(&mut self) {
        let _ = self.cleanup_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{AudioCodec, Container, VideoCodec};
    use crate::constraints::media_h264_mp4_aac;
    use crate::inspect::Inspector;
    use crate::plan::Plan;
    use crate::runtime::{ExecutionContext, TransformProvider};
    use std::path::Path;

    fn test_dir(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("fitifact-{label}-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    struct FakeInspector(Artifact);

    impl Inspector for FakeInspector {
        fn inspect(&self, _path: &Path) -> Result<Artifact> {
            Ok(self.0.clone())
        }
    }

    struct PanicProvider;

    impl TransformProvider for PanicProvider {
        fn execute(
            &self,
            _in: &Path,
            _out: &Path,
            _plan: &Plan,
            _ctx: &ExecutionContext,
        ) -> Result<()> {
            panic!("encoder should not start for a compatible file");
        }
    }

    #[test]
    fn compatible_adapt_does_not_execute() {
        let artifact = Artifact::media(Container::Mp4, VideoCodec::H264, Some(AudioCodec::Aac), 10);
        let result = adapt(AdaptRequest {
            input: Path::new("already.mp4"),
            constraints: media_h264_mp4_aac(),
            output: None,
            catalog: None,
            inspector: &FakeInspector(artifact),
            provider: &PanicProvider,
            execution: ExecutionContext::default(),
        })
        .unwrap();
        assert_eq!(result.status, AdaptationStatus::Compatible);
        assert!(result.output.is_none());
        assert!(result.plan.is_none());
    }

    struct FailProvider;

    impl TransformProvider for FailProvider {
        fn execute(
            &self,
            _in: &Path,
            _out: &Path,
            _plan: &Plan,
            _ctx: &ExecutionContext,
        ) -> Result<()> {
            Err(Error::new(
                ErrorCode::ProviderMissing,
                "ffmpeg was not found on PATH",
            ))
        }
    }

    #[test]
    fn missing_provider_is_failed_not_adapted() {
        let artifact = Artifact::media(Container::Mp4, VideoCodec::Hevc, Some(AudioCodec::Aac), 10);
        let result = adapt(AdaptRequest {
            input: Path::new("bad.mp4"),
            constraints: media_h264_mp4_aac(),
            output: Some(PathBuf::from("bad.adapted.mp4")),
            catalog: None,
            inspector: &FakeInspector(artifact),
            provider: &FailProvider,
            execution: ExecutionContext::default(),
        })
        .unwrap();
        assert_eq!(result.status, AdaptationStatus::Failed);
        assert_eq!(
            result.error.as_ref().unwrap().code,
            ErrorCode::ProviderMissing
        );
    }

    struct RoutingInspector {
        input: PathBuf,
        source: Artifact,
        output: Artifact,
    }

    impl Inspector for RoutingInspector {
        fn inspect(&self, path: &Path) -> Result<Artifact> {
            if path == self.input {
                Ok(self.source.clone())
            } else {
                Ok(self.output.clone())
            }
        }
    }

    #[derive(Default)]
    struct PartialFailProvider {
        output: std::sync::Mutex<Option<PathBuf>>,
    }

    impl TransformProvider for PartialFailProvider {
        fn execute(
            &self,
            _in: &Path,
            out: &Path,
            _plan: &Plan,
            _ctx: &ExecutionContext,
        ) -> Result<()> {
            *self.output.lock().unwrap() = Some(out.to_path_buf());
            std::fs::write(out, b"partial").unwrap();
            Err(Error::new(
                ErrorCode::ExecutionFailed,
                "simulated provider failure",
            ))
        }
    }

    struct PassProvider;

    impl TransformProvider for PassProvider {
        fn execute(
            &self,
            _in: &Path,
            out: &Path,
            _plan: &Plan,
            _ctx: &ExecutionContext,
        ) -> Result<()> {
            std::fs::write(out, b"valid-output").unwrap();
            Ok(())
        }

        fn stream_hashes(
            &self,
            _path: &Path,
            _artifact: &Artifact,
            _ctx: &ExecutionContext,
        ) -> Result<crate::runtime::StreamHashes> {
            Ok(crate::runtime::StreamHashes::new(
                "a".repeat(64),
                Some("b".repeat(64)),
            ))
        }
    }

    struct RacingProvider {
        destination: PathBuf,
    }

    impl TransformProvider for RacingProvider {
        fn execute(
            &self,
            _in: &Path,
            out: &Path,
            _plan: &Plan,
            _ctx: &ExecutionContext,
        ) -> Result<()> {
            std::fs::write(out, b"valid-output").unwrap();
            std::fs::write(&self.destination, b"foreign-race-winner").unwrap();
            Ok(())
        }

        fn stream_hashes(
            &self,
            _path: &Path,
            _artifact: &Artifact,
            _ctx: &ExecutionContext,
        ) -> Result<crate::runtime::StreamHashes> {
            Ok(crate::runtime::StreamHashes::new(
                "a".repeat(64),
                Some("b".repeat(64)),
            ))
        }
    }

    #[test]
    fn existing_explicit_output_is_refused_before_transform_spawn() {
        let dir = test_dir("existing-output");
        let input = dir.join("input.mov");
        let output = dir.join("occupied.mp4");
        std::fs::write(&input, b"original").unwrap();
        std::fs::write(&output, b"keep-me").unwrap();
        let artifact = Artifact::media(Container::Mov, VideoCodec::H264, Some(AudioCodec::Aac), 8);
        let result = adapt(AdaptRequest {
            input: &input,
            constraints: media_h264_mp4_aac(),
            output: Some(output.clone()),
            catalog: None,
            inspector: &FakeInspector(artifact),
            provider: &PanicProvider,
            execution: ExecutionContext::default(),
        })
        .unwrap();
        assert_eq!(result.status, AdaptationStatus::Failed);
        assert_eq!(
            result.error.as_ref().unwrap().code,
            ErrorCode::SecurityBlocked
        );
        assert_eq!(std::fs::read(&output).unwrap(), b"keep-me");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn output_created_during_transform_wins_without_being_overwritten_or_deleted() {
        let dir = test_dir("output-race");
        let input = dir.join("input.mov");
        let output = dir.join("requested.mp4");
        std::fs::write(&input, b"original").unwrap();
        let source = Artifact::media(Container::Mov, VideoCodec::H264, Some(AudioCodec::Aac), 8);
        let compatible =
            Artifact::media(Container::Mp4, VideoCodec::H264, Some(AudioCodec::Aac), 12);
        let inspector = RoutingInspector {
            input: input.clone(),
            source,
            output: compatible,
        };
        let result = adapt(AdaptRequest {
            input: &input,
            constraints: media_h264_mp4_aac(),
            output: Some(output.clone()),
            catalog: None,
            inspector: &inspector,
            provider: &RacingProvider {
                destination: output.clone(),
            },
            execution: ExecutionContext::default(),
        })
        .unwrap();
        assert_eq!(result.status, AdaptationStatus::Failed);
        assert_eq!(
            result.error.as_ref().unwrap().code,
            ErrorCode::SecurityBlocked
        );
        assert_eq!(std::fs::read(&output).unwrap(), b"foreign-race-winner");
        assert!(std::fs::read_dir(&dir).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("fitifact-stage")
        }));
        std::fs::remove_dir_all(dir).unwrap();
    }

    struct RecordingInspector {
        input: PathBuf,
        source: Artifact,
        output: Artifact,
        calls: std::sync::Mutex<Vec<PathBuf>>,
    }

    impl Inspector for RecordingInspector {
        fn inspect(&self, path: &Path) -> Result<Artifact> {
            self.calls.lock().unwrap().push(path.to_path_buf());
            if path == self.input {
                Ok(self.source.clone())
            } else {
                Ok(self.output.clone())
            }
        }
    }

    #[test]
    fn adapted_status_requires_fresh_inspection_of_the_published_path() {
        let dir = test_dir("final-validation");
        let input = dir.join("input.mov");
        let output = dir.join("requested.mp4");
        std::fs::write(&input, b"original").unwrap();
        let inspector = RecordingInspector {
            input: input.clone(),
            source: Artifact::media(Container::Mov, VideoCodec::H264, Some(AudioCodec::Aac), 8),
            output: Artifact::media(Container::Mp4, VideoCodec::H264, Some(AudioCodec::Aac), 12),
            calls: std::sync::Mutex::new(Vec::new()),
        };
        let result = adapt(AdaptRequest {
            input: &input,
            constraints: media_h264_mp4_aac(),
            output: Some(output.clone()),
            catalog: None,
            inspector: &inspector,
            provider: &PassProvider,
            execution: ExecutionContext::default(),
        })
        .unwrap();
        assert_eq!(result.status, AdaptationStatus::Adapted);
        let calls = inspector.calls.lock().unwrap();
        assert!(calls.iter().any(|path| path == &output));
        assert!(calls.iter().any(|path| {
            path.parent()
                .and_then(Path::file_name)
                .is_some_and(|name| name.to_string_lossy().starts_with(".fitifact-stage-"))
        }));
        drop(calls);
        std::fs::remove_file(output).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn provider_failure_cleans_hidden_sibling_stage() {
        let dir = test_dir("stage-cleanup");
        let input = dir.join("input.mov");
        let output = dir.join("requested.mp4");
        std::fs::write(&input, b"original").unwrap();
        let artifact = Artifact::media(Container::Mov, VideoCodec::H264, Some(AudioCodec::Aac), 8);
        let provider = PartialFailProvider::default();
        let result = adapt(AdaptRequest {
            input: &input,
            constraints: media_h264_mp4_aac(),
            output: Some(output.clone()),
            catalog: None,
            inspector: &FakeInspector(artifact),
            provider: &provider,
            execution: ExecutionContext::default(),
        })
        .unwrap();
        assert_eq!(result.status, AdaptationStatus::Failed);
        let staged = provider.output.lock().unwrap().clone().unwrap();
        assert_eq!(staged.parent().and_then(Path::parent), output.parent());
        assert!(
            staged
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".fitifact-stage-")
        );
        assert!(!staged.exists());
        assert!(!staged.parent().unwrap().exists());
        assert!(!output.exists());
        assert_eq!(std::fs::read(&input).unwrap(), b"original");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn default_output_uses_unique_fitifact_sibling_name() {
        let dir = test_dir("default-name");
        let input = dir.join("clip.mov");
        let occupied = dir.join("clip.fitifact.mp4");
        std::fs::write(&input, b"original").unwrap();
        std::fs::write(&occupied, b"occupied").unwrap();
        let source = Artifact::media(Container::Mov, VideoCodec::H264, Some(AudioCodec::Aac), 8);
        let compatible =
            Artifact::media(Container::Mp4, VideoCodec::H264, Some(AudioCodec::Aac), 12);
        let inspector = RoutingInspector {
            input: input.clone(),
            source,
            output: compatible,
        };
        let result = adapt(AdaptRequest {
            input: &input,
            constraints: media_h264_mp4_aac(),
            output: None,
            catalog: None,
            inspector: &inspector,
            provider: &PassProvider,
            execution: ExecutionContext::default(),
        })
        .unwrap();
        assert_eq!(result.status, AdaptationStatus::Adapted);
        assert_eq!(result.output, Some(dir.join("clip.fitifact.2.mp4")));
        assert_eq!(std::fs::read(&occupied).unwrap(), b"occupied");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn validation_mismatch_cleans_stage_and_never_persists_output() {
        let dir = test_dir("validation-cleanup");
        let input = dir.join("clip.mov");
        let output = dir.join("clip.mp4");
        std::fs::write(&input, b"original").unwrap();
        let source = Artifact::media(Container::Mov, VideoCodec::H264, Some(AudioCodec::Aac), 8);
        let wrong = Artifact::media(Container::Mp4, VideoCodec::Hevc, Some(AudioCodec::Aac), 12);
        let inspector = RoutingInspector {
            input: input.clone(),
            source,
            output: wrong,
        };
        let result = adapt(AdaptRequest {
            input: &input,
            constraints: media_h264_mp4_aac(),
            output: Some(output.clone()),
            catalog: None,
            inspector: &inspector,
            provider: &PassProvider,
            execution: ExecutionContext::default(),
        })
        .unwrap();
        assert_eq!(result.status, AdaptationStatus::Failed);
        assert_eq!(
            result.error.as_ref().unwrap().code,
            ErrorCode::ValidationFailed
        );
        assert!(!output.exists());
        assert!(std::fs::read_dir(&dir).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("fitifact-stage")
        }));
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn remux_plan(source: &Artifact) -> Plan {
        crate::plan::plan(
            source,
            &media_h264_mp4_aac(),
            &crate::capability::default_catalog(),
        )
        .plan()
        .unwrap()
        .clone()
    }

    #[test]
    fn stage_workspace_reservation_is_atomic_and_unique() {
        let dir = test_dir("atomic-reservation");
        let output = dir.join("output.mp4");
        let mut first = StageWorkspace::new(&output).unwrap();
        let mut second = StageWorkspace::new(&output).unwrap();
        assert_ne!(first.directory(), second.directory());
        assert!(first.directory().is_dir());
        assert!(second.directory().is_dir());
        assert!(first.cleanup_owned().complete);
        assert!(second.cleanup_owned().complete);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn cleanup_never_deletes_an_unclaimed_foreign_stage_file() {
        let dir = test_dir("foreign-stage");
        let output = dir.join("output.mp4");
        let mut stage = StageWorkspace::new(&output).unwrap();
        std::fs::write(stage.path(), b"foreign").unwrap();
        let cleanup = stage.cleanup_owned();
        assert!(!cleanup.complete);
        assert_eq!(std::fs::read(stage.path()).unwrap(), b"foreign");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn stage_replacement_after_claim_is_rejected_before_publish() {
        let dir = test_dir("stage-swap");
        let output = dir.join("output.mp4");
        let mut stage = StageWorkspace::new(&output).unwrap();
        std::fs::write(stage.path(), b"validated").unwrap();
        stage.claim_created_file().unwrap();
        let replacement_attempt = std::fs::remove_file(stage.path());
        if replacement_attempt.is_ok() {
            std::fs::write(stage.path(), b"foreign").unwrap();
            let err = stage.publish(&output).unwrap_err();
            assert_eq!(err.code, ErrorCode::SecurityBlocked);
            assert!(!output.exists());
            let cleanup = stage.cleanup_owned();
            assert!(!cleanup.complete);
            assert_eq!(std::fs::read(stage.path()).unwrap(), b"foreign");
        } else {
            let published = stage.publish(&output).unwrap();
            published.verify().unwrap();
            assert_eq!(std::fs::read(&output).unwrap(), b"validated");
            drop(published);
            assert!(stage.cleanup_owned().complete);
            std::fs::remove_file(&output).unwrap();
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn staging_unlink_failure_never_removes_published_final() {
        let dir = test_dir("unlink-failure");
        let output = dir.join("output.mp4");
        let mut stage = StageWorkspace::new(&output).unwrap();
        std::fs::write(stage.path(), b"validated").unwrap();
        stage.claim_created_file().unwrap();
        let published = stage.publish(&output).unwrap();
        published.verify().unwrap();
        let blocker = {
            use std::os::windows::fs::OpenOptionsExt;
            OpenOptions::new()
                .read(true)
                .share_mode(0x0000_0001)
                .open(stage.path())
                .unwrap()
        };
        drop(published);
        let cleanup = stage.cleanup_owned();
        assert!(!cleanup.complete);
        assert_eq!(std::fs::read(&output).unwrap(), b"validated");
        drop(blocker);
        assert!(stage.cleanup_owned().complete);
        assert_eq!(std::fs::read(&output).unwrap(), b"validated");
        std::fs::remove_file(output).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn default_name_exhaustion_returns_an_error() {
        let dir = test_dir("name-exhaustion");
        let input = dir.join("clip.mov");
        let source = Artifact::media(Container::Mov, VideoCodec::H264, Some(AudioCodec::Aac), 8);
        let err = default_output_with_limit(
            &input,
            &remux_plan(&source),
            source.container.as_ref(),
            2,
            |_| true,
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::ExecutionFailed);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
