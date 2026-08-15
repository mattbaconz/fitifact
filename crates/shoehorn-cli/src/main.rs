use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use shoehorn::artifact::Artifact;
use shoehorn::ffmpeg::FfmpegProvider;
use shoehorn::inspect::FfprobeInspector;
use shoehorn::runtime::{ExecutionContext, SystemSpawner};
use shoehorn::{
    AdaptRequest, AdaptationStatus, ConstraintInput, ConstraintSet, PlanOutcome, adapt, check,
    compile, compile_from_yaml, explain_check, explain_plan, inspect, plan,
};

#[derive(Parser)]
#[command(
    name = "shoehorn",
    about = "Make a file fit a destination by changing as little as possible."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Inspect real file internals (not the extension).
    Inspect {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Check a file against typed destination constraints.
    Check {
        file: PathBuf,
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long)]
        json: bool,
    },
    /// Plan the minimum mutation without executing.
    Plan {
        file: PathBuf,
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long)]
        json: bool,
    },
    /// Adapt a file, then validate the output against the same constraints.
    Adapt {
        file: PathBuf,
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Report whether ffprobe/ffmpeg are available.
    Doctor,
}

#[derive(Args, Clone)]
struct TargetArgs {
    #[arg(long)]
    container: Option<String>,
    #[arg(long = "video-codec")]
    video_codec: Option<String>,
    #[arg(long = "audio-codec")]
    audio_codec: Option<String>,
    #[arg(long = "max-size", value_name = "BYTES")]
    max_size: Option<u64>,
    #[arg(long = "max-width")]
    max_width: Option<u32>,
    #[arg(long = "max-height")]
    max_height: Option<u32>,
    #[arg(long)]
    constraints: Option<PathBuf>,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{err}");
            if err.usage {
                ExitCode::from(64)
            } else {
                exit_for_error(err.code)
            }
        }
    }
}

fn run() -> Result<ExitCode, CliError> {
    let cli = Cli::parse();
    match cli.command {
        Command::Inspect { file, json } => inspect_cmd(&file, json),
        Command::Check { file, target, json } => check_cmd(&file, target, json),
        Command::Plan { file, target, json } => plan_cmd(&file, target, json, false),
        Command::Adapt {
            file,
            target,
            output,
            json,
            dry_run,
        } => {
            if dry_run {
                plan_cmd(&file, target, json, true)
            } else {
                adapt_cmd(&file, target, output, json)
            }
        }
        Command::Doctor => doctor_cmd(),
    }
}

fn inspect_cmd(file: &Path, json: bool) -> Result<ExitCode, CliError> {
    let inspector = FfprobeInspector::default();
    let artifact = inspect(file, &inspector).map_err(CliError::engine)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&artifact).map_err(CliError::json)?
        );
    } else {
        print_inspect(&artifact);
    }
    Ok(ExitCode::SUCCESS)
}

fn check_cmd(file: &Path, target: TargetArgs, json: bool) -> Result<ExitCode, CliError> {
    let constraints = load_constraints(&target)?;
    let inspector = FfprobeInspector::default();
    let artifact = inspect(file, &inspector).map_err(CliError::engine)?;
    let report = check(&artifact, &constraints);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(CliError::json)?
        );
    } else {
        let explanation = explain_check(&artifact, &report);
        println!("{}", explanation.summary);
        for line in explanation.details {
            println!("  {line}");
        }
    }
    if report.compatible {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(2))
    }
}

fn plan_cmd(
    file: &Path,
    target: TargetArgs,
    json: bool,
    from_dry_run: bool,
) -> Result<ExitCode, CliError> {
    let constraints = load_constraints(&target)?;
    let inspector = FfprobeInspector::default();
    let artifact = inspect(file, &inspector).map_err(CliError::engine)?;
    let report = check(&artifact, &constraints);
    let outcome = plan(&artifact, &constraints, &shoehorn::default_catalog());
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&outcome).map_err(CliError::json)?
        );
    } else {
        if from_dry_run {
            println!("Dry run: no files will be written.");
        }
        let explanation = explain_plan(&artifact, &report, &outcome);
        println!("{}", explanation.summary);
        for line in explanation.details {
            println!("  {line}");
        }
    }
    match outcome {
        PlanOutcome::Compatible => Ok(ExitCode::SUCCESS),
        PlanOutcome::Planned { .. } => Ok(ExitCode::SUCCESS),
        PlanOutcome::CannotSatisfy { .. } => Ok(ExitCode::from(3)),
    }
}

fn adapt_cmd(
    file: &Path,
    target: TargetArgs,
    output: Option<PathBuf>,
    json: bool,
) -> Result<ExitCode, CliError> {
    let constraints = load_constraints(&target)?;
    let inspector = FfprobeInspector::default();
    let provider = FfmpegProvider::<SystemSpawner>::default();
    let result = adapt(AdaptRequest {
        input: file,
        constraints,
        output,
        catalog: None,
        inspector: &inspector,
        provider: &provider,
        execution: ExecutionContext::default(),
    })
    .map_err(CliError::engine)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(CliError::json)?
        );
    } else {
        println!("{}", result.explanation.summary);
        for line in &result.explanation.details {
            println!("  {line}");
        }
        match result.status {
            AdaptationStatus::Compatible => {}
            AdaptationStatus::Adapted => {
                if let Some(path) = &result.output {
                    println!("Wrote {}", path.display());
                }
            }
            AdaptationStatus::CannotSatisfy | AdaptationStatus::Failed => {
                if let Some(err) = &result.error {
                    println!("{}: {}", err.code, err.message);
                }
            }
        }
    }
    Ok(match result.status {
        AdaptationStatus::Compatible | AdaptationStatus::Adapted => ExitCode::SUCCESS,
        AdaptationStatus::CannotSatisfy => ExitCode::from(3),
        AdaptationStatus::Failed => match result.error.as_ref().map(|e| e.code) {
            Some(shoehorn::ErrorCode::ValidationFailed) => ExitCode::from(6),
            Some(shoehorn::ErrorCode::ProviderMissing) => ExitCode::from(5),
            Some(shoehorn::ErrorCode::SecurityBlocked) => ExitCode::from(7),
            _ => ExitCode::from(5),
        },
    })
}

fn doctor_cmd() -> Result<ExitCode, CliError> {
    let ffprobe = tool_version("ffprobe");
    let ffmpeg = tool_version("ffmpeg");
    match &ffprobe {
        Ok(v) => println!("ffprobe: {v}"),
        Err(err) => println!("ffprobe: missing ({err})"),
    }
    match &ffmpeg {
        Ok(v) => println!("ffmpeg: {v}"),
        Err(err) => println!("ffmpeg: missing ({err})"),
    }
    if ffprobe.is_ok() && ffmpeg.is_ok() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(5))
    }
}

fn tool_version(name: &str) -> Result<String, String> {
    let output = std::process::Command::new(name)
        .arg("-version")
        .output()
        .map_err(|err| err.to_string())?;
    if !output.status.success() {
        return Err(format!("exit {}", output.status));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.lines().next().unwrap_or("ok").to_string())
}

fn load_constraints(target: &TargetArgs) -> Result<ConstraintSet, CliError> {
    if let Some(path) = &target.constraints {
        let text = std::fs::read_to_string(path)
            .map_err(|err| CliError::usage(format!("cannot read {}: {err}", path.display())))?;
        return compile_from_yaml(&text).map_err(CliError::engine);
    }
    if target.container.is_none()
        && target.video_codec.is_none()
        && target.audio_codec.is_none()
        && target.max_size.is_none()
        && target.max_width.is_none()
        && target.max_height.is_none()
    {
        return Err(CliError::usage(
            "provide destination constraints via flags (e.g. --container mp4 --video-codec h264) or --constraints FILE.yaml",
        ));
    }
    Ok(compile(ConstraintInput {
        container: target.container.clone().map(|v| vec![v]),
        video_codec: target.video_codec.clone().map(|v| vec![v]),
        audio_codec: target.audio_codec.clone().map(|v| vec![v]),
        max_bytes: target.max_size,
        max_width: target.max_width,
        max_height: target.max_height,
        ..ConstraintInput::default()
    }))
}

fn print_inspect(artifact: &Artifact) {
    let name = artifact
        .path
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    println!("{name}");
    println!(
        "Container       {}",
        artifact
            .container
            .as_ref()
            .map(|c| c.as_str().to_ascii_uppercase())
            .unwrap_or_else(|| "unknown".into())
    );
    println!(
        "Video           {}",
        artifact
            .video
            .as_ref()
            .and_then(|v| v.codec.as_ref())
            .map(|c| c.as_str().to_ascii_uppercase())
            .unwrap_or_else(|| "none".into())
    );
    println!(
        "Audio           {}",
        artifact
            .audio
            .as_ref()
            .and_then(|a| a.codec.as_ref())
            .map(|c| c.as_str().to_ascii_uppercase())
            .unwrap_or_else(|| "none".into())
    );
    if let Some(video) = &artifact.video
        && let (Some(w), Some(h)) = (video.width, video.height)
    {
        println!("Resolution      {w}×{h}");
    }
    println!("Size            {}", format_bytes(artifact.byte_length));
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

struct CliError {
    message: String,
    code: Option<shoehorn::ErrorCode>,
    usage: bool,
}

impl CliError {
    fn engine(err: shoehorn::Error) -> Self {
        Self {
            message: err.to_string(),
            code: Some(err.code),
            usage: false,
        }
    }

    fn json(err: serde_json::Error) -> Self {
        Self {
            message: err.to_string(),
            code: None,
            usage: false,
        }
    }

    fn usage(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: None,
            usage: true,
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

fn exit_for_error(code: Option<shoehorn::ErrorCode>) -> ExitCode {
    match code {
        Some(shoehorn::ErrorCode::InputInvalid)
        | Some(shoehorn::ErrorCode::InspectionUnsupported)
        | Some(shoehorn::ErrorCode::InspectionLimit) => ExitCode::from(4),
        Some(shoehorn::ErrorCode::ProviderMissing)
        | Some(shoehorn::ErrorCode::ExecutionFailed)
        | Some(shoehorn::ErrorCode::ExecutionLimit) => ExitCode::from(5),
        Some(shoehorn::ErrorCode::ValidationFailed) => ExitCode::from(6),
        Some(shoehorn::ErrorCode::SecurityBlocked) => ExitCode::from(7),
        Some(shoehorn::ErrorCode::NoValidPlan) => ExitCode::from(3),
        _ => ExitCode::from(64),
    }
}
