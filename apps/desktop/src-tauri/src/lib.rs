mod clipboard;
mod engine;

use engine::{
    adapt_path_cancellable, check_path, doctor_report, inspect_path, open_user_file, plan_path,
    read_header as read_header_bytes, read_limited_image, CommandError, TargetSpec,
};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

#[derive(Default)]
struct Jobs {
    active: Mutex<HashMap<String, fitifact::CancellationToken>>,
    revealable: Mutex<HashSet<std::path::PathBuf>>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    operation_id: String,
    stage: String,
}

fn emit_progress(app: &tauri::AppHandle, operation_id: &str, stage: &str) {
    let _ = app.emit(
        "fitifact-progress",
        ProgressEvent {
            operation_id: operation_id.into(),
            stage: stage.into(),
        },
    );
}

async fn join_blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, CommandError> + Send + 'static,
) -> Result<T, CommandError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| CommandError::new("EXECUTION_FAILED", error.to_string()))?
}

#[tauri::command]
async fn inspect(app: tauri::AppHandle, path: String) -> Result<fitifact::Artifact, CommandError> {
    let path = open_user_file(&path)?;
    app.state::<Jobs>()
        .revealable
        .lock()
        .unwrap()
        .insert(path.clone());
    join_blocking(move || inspect_path(&path)).await
}

#[tauri::command]
async fn check(
    path: String,
    target: TargetSpec,
) -> Result<fitifact::CompatibilityReport, CommandError> {
    join_blocking(move || check_path(&open_user_file(&path)?, &target)).await
}

#[tauri::command]
async fn plan(path: String, target: TargetSpec) -> Result<fitifact::PlanOutcome, CommandError> {
    join_blocking(move || plan_path(&open_user_file(&path)?, &target)).await
}

#[tauri::command]
async fn adapt(
    app: tauri::AppHandle,
    path: String,
    target: TargetSpec,
    operation_id: String,
) -> Result<fitifact::AdaptationResult, CommandError> {
    if operation_id.is_empty() || operation_id.len() > 100 {
        return Err(CommandError::new(
            "INPUT_INVALID",
            "Invalid operation identifier.",
        ));
    }
    let path = open_user_file(&path)?;
    let token = fitifact::CancellationToken::default();
    {
        let jobs = app.state::<Jobs>();
        let mut active = jobs.active.lock().unwrap();
        if !active.is_empty() {
            return Err(CommandError::new(
                "EXECUTION_FAILED",
                "Another file is still processing.",
            ));
        }
        active.insert(operation_id.clone(), token.clone());
        jobs.revealable.lock().unwrap().insert(path.clone());
    }
    let worker_app = app.clone();
    let job_id = operation_id.clone();
    let result = join_blocking(move || {
        emit_progress(&worker_app, &job_id, "Adapting on this device");
        adapt_path_cancellable(&path, &target, &token)
    })
    .await;
    app.state::<Jobs>()
        .active
        .lock()
        .unwrap()
        .remove(&operation_id);
    if let Ok(result) = &result {
        if let Some(output) = &result.output {
            app.state::<Jobs>()
                .revealable
                .lock()
                .unwrap()
                .insert(output.canonicalize().unwrap_or_else(|_| output.clone()));
        }
        if matches!(result.status, fitifact::AdaptationStatus::Adapted)
            && result
                .validation
                .as_ref()
                .is_some_and(|v| v.status == fitifact::validate::ValidationStatus::Pass)
        {
            emit_progress(&app, &operation_id, "Output validated");
        }
    }
    result
}

#[tauri::command]
fn cancel(app: tauri::AppHandle, operation_id: String) -> Result<bool, CommandError> {
    let jobs = app.state::<Jobs>();
    let active = jobs.active.lock().unwrap();
    // An already-finished job keeps its result; never fabricate cancellation.
    Ok(active
        .get(&operation_id)
        .is_some_and(|token| token.cancel()))
}

/// A file the user chose or Fitifact produced in this session.
fn known_file(
    app: &tauri::AppHandle,
    path: &str,
    action: &str,
) -> Result<std::path::PathBuf, CommandError> {
    let path = open_user_file(path)?;
    if !app
        .state::<Jobs>()
        .revealable
        .lock()
        .unwrap()
        .contains(&path)
    {
        return Err(CommandError::new(
            "INPUT_INVALID",
            format!("Select or produce this file before {action} it."),
        ));
    }
    Ok(path)
}

#[tauri::command]
fn reveal(app: tauri::AppHandle, path: String) -> Result<(), CommandError> {
    let path = known_file(&app, &path, "revealing")?;
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|error| CommandError::new("EXECUTION_FAILED", error.to_string()))
}

#[tauri::command]
fn copy_file(app: tauri::AppHandle, path: String) -> Result<(), CommandError> {
    let path = known_file(&app, &path, "copying")?;
    clipboard::copy_file(&path)
        .map_err(|error| CommandError::new("EXECUTION_FAILED", error.to_string()))
}

#[tauri::command]
async fn doctor() -> Result<fitifact::doctor::DoctorReport, CommandError> {
    join_blocking(|| Ok(doctor_report())).await
}

#[tauri::command]
fn compile_target(target: TargetSpec) -> Result<fitifact::ConstraintSet, CommandError> {
    engine::load_target(&target)
}

#[tauri::command]
async fn read_header(path: String) -> Result<Vec<u8>, CommandError> {
    join_blocking(move || read_header_bytes(&open_user_file(&path)?)).await
}

#[tauri::command]
async fn read_limited_file(path: String) -> Result<tauri::ipc::Response, CommandError> {
    let bytes = join_blocking(move || read_limited_image(&open_user_file(&path)?)).await?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Jobs::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let mut tool_dirs = Vec::new();
            if let Ok(exe) = std::env::current_exe() {
                if let Some(dir) = exe.parent() {
                    tool_dirs.push(dir.to_path_buf());
                }
            }
            if let Ok(resource) = app.path().resource_dir() {
                let profiles = resource.join("profiles");
                if profiles.is_dir() {
                    std::env::set_var("FITIFACT_PROFILES", &profiles);
                }
                tool_dirs.push(resource);
            }
            let refs: Vec<&std::path::Path> =
                tool_dirs.iter().map(std::path::PathBuf::as_path).collect();
            fitifact::bind_media_tools_from_dirs(refs);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            inspect,
            check,
            plan,
            adapt,
            cancel,
            reveal,
            copy_file,
            doctor,
            compile_target,
            read_header,
            read_limited_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running Fitifact desktop");
}
