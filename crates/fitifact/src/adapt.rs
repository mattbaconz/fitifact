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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleanup_warning: Option<CleanupWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupWarning {
    pub path: PathBuf,
    pub message: String,
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
            cleanup_warning: None,
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
            cleanup_warning: None,
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
                let cleanup = staged.cleanup_owned();
                return Ok(with_cleanup_warning(
                    failed(
                        original,
                        artifact,
                        report,
                        Some(plan),
                        explanation,
                        err,
                        None,
                    ),
                    cleanup,
                ));
            }
            if let Err(err) = staged.claim_created_file() {
                let cleanup = staged.cleanup_owned();
                return Ok(with_cleanup_warning(
                    failed(
                        original,
                        artifact,
                        report,
                        Some(plan),
                        explanation,
                        err,
                        None,
                    ),
                    cleanup,
                ));
            }
            let source_hashes =
                match request
                    .provider
                    .stream_hashes(request.input, &artifact, &request.execution)
                {
                    Ok(hashes) => hashes,
                    Err(err) => {
                        let cleanup = staged.cleanup_owned();
                        return Ok(with_cleanup_warning(
                            failed(
                                original,
                                artifact,
                                report,
                                Some(plan),
                                explanation,
                                err,
                                None,
                            ),
                            cleanup,
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
                        let cleanup = staged.cleanup_owned();
                        return Ok(with_cleanup_warning(
                            failed(
                                original,
                                artifact,
                                report,
                                Some(plan),
                                explanation,
                                err,
                                None,
                            ),
                            cleanup,
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
                    let identity_confirmed = match staged.publish(&output) {
                        Ok(confirmed) => confirmed,
                        Err(err) => {
                            let cleanup = staged.cleanup_owned();
                            return Ok(with_cleanup_warning(
                                failed(
                                    original,
                                    artifact,
                                    report,
                                    Some(plan),
                                    explanation,
                                    err,
                                    Some(staged_validation),
                                ),
                                cleanup,
                            ));
                        }
                    };
                    let cleanup = staged.cleanup_owned();
                    if !identity_confirmed {
                        let mut result = failed(
                            original,
                            artifact,
                            report,
                            Some(plan),
                            explanation,
                            Error::new(
                                ErrorCode::SecurityBlocked,
                                "the published output identity could not be confirmed; it was preserved",
                            ),
                            Some(staged_validation),
                        );
                        result.output = Some(output.clone());
                        let publication_warning = CleanupOutcome::warning(
                            output,
                            "The published path could not be identity-confirmed and was intentionally preserved."
                                .into(),
                        );
                        if let Some(staging_warning) = cleanup.warning {
                            result
                                .explanation
                                .details
                                .push(staging_warning.message.clone());
                            if let Some(error) = result.error.as_mut() {
                                error.details.insert(
                                    "staging_cleanup_warning".into(),
                                    serde_json::to_value(staging_warning)
                                        .unwrap_or(serde_json::Value::Null),
                                );
                            }
                        }
                        return Ok(with_cleanup_warning(result, publication_warning));
                    }
                    Ok(with_cleanup_warning(
                        AdaptationResult {
                            schema: AdaptationSchema,
                            status: AdaptationStatus::Adapted,
                            original,
                            output: Some(output),
                            artifact,
                            report,
                            plan: Some(plan),
                            validation: Some(staged_validation),
                            explanation,
                            error: None,
                            cleanup_warning: None,
                        },
                        cleanup,
                    ))
                }
                Ok(validation) => {
                    let cleanup = staged.cleanup_owned();
                    Ok(with_cleanup_warning(
                        failed(
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
                        ),
                        cleanup,
                    ))
                }
                Err(err) => {
                    let cleanup = staged.cleanup_owned();
                    Ok(with_cleanup_warning(
                        failed(
                            original,
                            artifact,
                            report,
                            Some(plan),
                            explanation,
                            err,
                            None,
                        ),
                        cleanup,
                    ))
                }
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
        cleanup_warning: None,
    }
}

fn with_cleanup_warning(mut result: AdaptationResult, cleanup: CleanupOutcome) -> AdaptationResult {
    let Some(warning) = cleanup.warning else {
        return result;
    };
    result.explanation.details.push(warning.message.clone());
    if let Some(error) = result.error.as_mut() {
        error.details.insert(
            "cleanup_warning".into(),
            serde_json::to_value(&warning).unwrap_or(serde_json::Value::Null),
        );
    }
    result.cleanup_warning = Some(warning);
    result
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

#[cfg(unix)]
fn create_private_workspace(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    let mut builder = std::fs::DirBuilder::new();
    builder.mode(0o700).create(path)
}

#[cfg(not(unix))]
fn create_private_workspace(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir(path)
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

#[cfg(windows)]
fn open_workspace_handle(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{DELETE, FILE_GENERIC_READ};

    const FILE_SHARE_READ_WRITE: u32 = 0x0000_0001 | 0x0000_0002;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    OpenOptions::new()
        .access_mode(FILE_GENERIC_READ | DELETE)
        .share_mode(FILE_SHARE_READ_WRITE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
}

#[cfg(not(windows))]
fn open_workspace_handle(path: &Path) -> std::io::Result<File> {
    identity_read(path, true)
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

    const FILE_SHARE_READ_DELETE: u32 = 0x0000_0001 | 0x0000_0004;
    OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ_DELETE)
        .open(path)
}

#[cfg(not(windows))]
fn protected_read(path: &Path) -> std::io::Result<File> {
    OpenOptions::new().read(true).open(path)
}

#[cfg(windows)]
fn deletion_read(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{DELETE, FILE_GENERIC_READ};

    const FILE_SHARE_READ: u32 = 0x0000_0001;
    OpenOptions::new()
        .access_mode(FILE_GENERIC_READ | DELETE)
        .share_mode(FILE_SHARE_READ)
        .open(path)
}

#[cfg(windows)]
fn delete_held(handle: File) -> bool {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_DISPOSITION_INFO, FileDispositionInfo, SetFileInformationByHandle,
    };

    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
    // SAFETY: `handle` is live and was opened with DELETE access. The input
    // pointer and byte count describe an initialized FILE_DISPOSITION_INFO.
    let marked = unsafe {
        SetFileInformationByHandle(
            handle.as_raw_handle(),
            FileDispositionInfo,
            std::ptr::from_ref(&disposition).cast(),
            std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    } != 0;
    drop(handle);
    marked
}

#[derive(Debug, Clone)]
struct CleanupOutcome {
    warning: Option<CleanupWarning>,
}

impl CleanupOutcome {
    fn complete() -> Self {
        Self { warning: None }
    }

    fn warning(path: PathBuf, message: String) -> Self {
        Self {
            warning: Some(CleanupWarning { path, message }),
        }
    }
}

#[derive(Debug)]
struct StageWorkspace {
    directory: PathBuf,
    directory_identity: FileIdentity,
    directory_handle: Option<File>,
    path: PathBuf,
    file_identity: Option<FileIdentity>,
    protection: Option<File>,
    deletion_ready: bool,
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
            match create_private_workspace(&directory) {
                Ok(()) => {
                    let directory_handle = open_workspace_handle(&directory).map_err(|_| {
                        Error::new(
                            ErrorCode::SecurityBlocked,
                            "the staging workspace could not be held safely",
                        )
                    })?;
                    let directory_identity = file_identity(&directory_handle).ok_or_else(|| {
                        Error::new(
                            ErrorCode::SecurityBlocked,
                            "the staging workspace identity could not be established",
                        )
                    })?;
                    let path = directory.join(format!("artifact.{extension}"));
                    return Ok(Self {
                        directory,
                        directory_identity,
                        directory_handle: Some(directory_handle),
                        path,
                        file_identity: None,
                        protection: None,
                        deletion_ready: false,
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
        self.deletion_ready = false;
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

    #[cfg(windows)]
    fn prepare_for_publication(&mut self) -> Result<()> {
        if self.deletion_ready {
            return self.verify_claim();
        }
        self.verify_claim()?;
        let expected = self.file_identity.expect("verified claim has identity");
        let deletion_handle = deletion_read(&self.path).map_err(|_| {
            Error::new(
                ErrorCode::SecurityBlocked,
                "the staging output could not be bound to an owned deletion handle",
            )
        })?;
        if file_identity(&deletion_handle) != Some(expected)
            || path_identity(&self.path, false) != Some(expected)
        {
            self.file_identity = None;
            self.protection.take();
            self.deletion_ready = false;
            return Err(Error::new(
                ErrorCode::SecurityBlocked,
                "the staging output changed before publication",
            ));
        }
        self.protection = Some(deletion_handle);
        self.deletion_ready = true;
        Ok(())
    }

    #[cfg(not(windows))]
    fn prepare_for_publication(&mut self) -> Result<()> {
        self.verify_claim()
    }

    fn publish(&mut self, destination: &Path) -> Result<bool> {
        if let Err(error) = self.prepare_for_publication() {
            self.file_identity = None;
            self.protection.take();
            self.deletion_ready = false;
            return Err(error);
        }
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
        let expected = self.file_identity.expect("verified claim has identity");
        Ok(path_identity(destination, false) == Some(expected)
            && self.protection.as_ref().and_then(file_identity) == Some(expected))
    }

    #[cfg(windows)]
    fn cleanup_owned(&mut self) -> CleanupOutcome {
        if self.verify_directory().is_err() {
            return CleanupOutcome::warning(
                self.directory.clone(),
                "The staging workspace identity changed; it was intentionally preserved.".into(),
            );
        }
        if path_is_occupied(&self.path) {
            if self.file_identity.is_none() {
                return CleanupOutcome::warning(
                    self.directory.clone(),
                    "The provider left an unclaimed staging object; the private workspace was intentionally preserved."
                        .into(),
                );
            }
            if let Err(error) = self.prepare_for_publication() {
                return CleanupOutcome::warning(
                    self.directory.clone(),
                    format!(
                        "The staging file could not be bound to its owned deletion handle; it was intentionally preserved ({error})."
                    ),
                );
            }
            let Some(handle) = self.protection.take() else {
                return CleanupOutcome::warning(
                    self.directory.clone(),
                    "The owned staging handle was unavailable; the workspace was intentionally preserved."
                        .into(),
                );
            };
            self.deletion_ready = false;
            if !delete_held(handle) {
                return CleanupOutcome::warning(
                    self.directory.clone(),
                    "The owned staging file could not be deleted by handle; the published output was preserved."
                        .into(),
                );
            }
        }
        let Some(directory_handle) = self.directory_handle.take() else {
            return CleanupOutcome::warning(
                self.directory.clone(),
                "The owned workspace handle was unavailable; the workspace was intentionally preserved."
                    .into(),
            );
        };
        if !delete_held(directory_handle) {
            return CleanupOutcome::warning(
                self.directory.clone(),
                "The owned staging workspace could not be deleted by handle; it was intentionally preserved."
                    .into(),
            );
        }
        CleanupOutcome::complete()
    }

    #[cfg(unix)]
    fn cleanup_owned(&mut self) -> CleanupOutcome {
        if self.verify_directory().is_err() {
            return CleanupOutcome::warning(
                self.directory.clone(),
                "The staging workspace identity changed; it was intentionally preserved.".into(),
            );
        }
        if path_is_occupied(&self.path) {
            let Some(_expected) = self.file_identity else {
                return CleanupOutcome::warning(
                    self.directory.clone(),
                    "The provider left an unclaimed staging object; the private workspace was intentionally preserved."
                        .into(),
                );
            };
            // Unix workspaces are atomically created mode 0700. Under the
            // documented same-account trust boundary, a still-claimed name
            // inside this private directory is owned by this operation.
            if std::fs::remove_file(&self.path).is_err() {
                return CleanupOutcome::warning(
                    self.directory.clone(),
                    "The claimed staging object could not be removed from the private workspace; it was intentionally preserved."
                        .into(),
                );
            }
        }
        self.protection.take();
        self.directory_handle.take();
        if std::fs::remove_dir(&self.directory).is_err() {
            return CleanupOutcome::warning(
                self.directory.clone(),
                "The private staging workspace could not be removed; it was intentionally preserved."
                    .into(),
            );
        }
        CleanupOutcome::complete()
    }

    #[cfg(not(any(windows, unix)))]
    fn cleanup_owned(&mut self) -> CleanupOutcome {
        CleanupOutcome::warning(
            self.directory.clone(),
            "This platform has no identity-atomic cleanup primitive; the staging workspace was intentionally preserved."
                .into(),
        )
    }
}

impl Drop for StageWorkspace {
    fn drop(&mut self) {
        if self.directory_handle.is_some() || self.protection.is_some() {
            let _ = self.cleanup_owned();
        }
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
    fn adapted_status_uses_freshly_validated_stage_for_identity_bound_publication() {
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
        assert!(calls.iter().any(|path| {
            path.parent()
                .and_then(Path::file_name)
                .is_some_and(|name| name.to_string_lossy().starts_with(".fitifact-stage-"))
        }));
        assert!(!calls.iter().any(|path| path == &output));
        assert_eq!(std::fs::read(&output).unwrap(), b"valid-output");
        drop(calls);
        std::fs::remove_file(output).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn provider_failure_preserves_ambiguous_partial_and_reports_cleanup_path() {
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
        assert_eq!(std::fs::read(&staged).unwrap(), b"partial");
        assert_eq!(
            result.cleanup_warning.as_ref().unwrap().path,
            staged.parent().unwrap()
        );
        assert_eq!(
            result.error.as_ref().unwrap().details["cleanup_warning"]["path"],
            serde_json::to_value(staged.parent().unwrap()).unwrap()
        );
        assert!(!output.exists());
        assert_eq!(std::fs::read(&input).unwrap(), b"original");
        std::fs::remove_dir_all(dir).unwrap();
    }

    struct PollutingInspector {
        input: PathBuf,
        source: Artifact,
        output: Artifact,
    }

    impl Inspector for PollutingInspector {
        fn inspect(&self, path: &Path) -> Result<Artifact> {
            if path == self.input {
                return Ok(self.source.clone());
            }
            if path
                .parent()
                .and_then(Path::file_name)
                .is_some_and(|name| name.to_string_lossy().starts_with(".fitifact-stage-"))
            {
                std::fs::write(path.parent().unwrap().join("foreign"), b"keep").unwrap();
            }
            Ok(self.output.clone())
        }
    }

    #[test]
    fn post_link_cleanup_failure_returns_published_output_and_structured_warning() {
        let dir = test_dir("post-link-cleanup");
        let input = dir.join("input.mov");
        let output = dir.join("requested.mp4");
        std::fs::write(&input, b"original").unwrap();
        let inspector = PollutingInspector {
            input: input.clone(),
            source: Artifact::media(Container::Mov, VideoCodec::H264, Some(AudioCodec::Aac), 8),
            output: Artifact::media(Container::Mp4, VideoCodec::H264, Some(AudioCodec::Aac), 12),
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
        assert_eq!(result.output, Some(output.clone()));
        assert_eq!(std::fs::read(&output).unwrap(), b"valid-output");
        let warning = result.cleanup_warning.unwrap();
        assert!(warning.path.starts_with(&dir));
        assert_eq!(
            std::fs::read(warning.path.join("foreign")).unwrap(),
            b"keep"
        );
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
        assert!(first.cleanup_owned().warning.is_none());
        assert!(second.cleanup_owned().warning.is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn cleanup_never_deletes_an_unclaimed_foreign_stage_file() {
        let dir = test_dir("foreign-stage");
        let output = dir.join("output.mp4");
        let mut stage = StageWorkspace::new(&output).unwrap();
        std::fs::write(stage.path(), b"foreign").unwrap();
        let cleanup = stage.cleanup_owned();
        assert!(cleanup.warning.is_some());
        assert_eq!(std::fs::read(stage.path()).unwrap(), b"foreign");
        drop(stage);
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
            assert!(cleanup.warning.is_some());
            assert_eq!(std::fs::read(stage.path()).unwrap(), b"foreign");
            drop(stage);
        } else {
            assert!(stage.publish(&output).unwrap());
            assert_eq!(std::fs::read(&output).unwrap(), b"validated");
            assert!(stage.cleanup_owned().warning.is_none());
            std::fs::remove_file(&output).unwrap();
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn windows_handle_cleanup_removes_only_stage_link_and_owned_workspace() {
        let dir = test_dir("handle-cleanup");
        let output = dir.join("output.mp4");
        let mut stage = StageWorkspace::new(&output).unwrap();
        let workspace = stage.directory().to_path_buf();
        std::fs::write(stage.path(), b"validated").unwrap();
        stage.claim_created_file().unwrap();
        assert!(stage.publish(&output).unwrap());
        assert!(stage.cleanup_owned().warning.is_none());
        assert_eq!(std::fs::read(&output).unwrap(), b"validated");
        assert!(!workspace.exists());
        std::fs::remove_file(output).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn claimed_stage_remains_readable_by_tools_that_do_not_share_delete_access() {
        use std::os::windows::fs::OpenOptionsExt;

        let dir = test_dir("external-reader-sharing");
        let output = dir.join("output.mp4");
        let mut stage = StageWorkspace::new(&output).unwrap();
        std::fs::write(stage.path(), b"validated").unwrap();
        stage.claim_created_file().unwrap();
        let reader = OpenOptions::new()
            .read(true)
            .share_mode(0x0000_0001)
            .open(stage.path())
            .unwrap();
        drop(reader);
        assert!(stage.cleanup_owned().warning.is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn unix_workspace_is_created_mode_0700_and_owned_cleanup_completes() {
        use std::os::unix::fs::PermissionsExt;

        let dir = test_dir("unix-private-workspace");
        let output = dir.join("output.mp4");
        let mut stage = StageWorkspace::new(&output).unwrap();
        assert_eq!(
            std::fs::metadata(stage.directory())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        std::fs::write(stage.path(), b"owned").unwrap();
        stage.claim_created_file().unwrap();
        assert!(stage.cleanup_owned().warning.is_none());
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
