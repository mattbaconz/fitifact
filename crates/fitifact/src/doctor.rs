use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

use crate::contract::DoctorSchema;
use crate::runtime::ProcessSpawner;

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
    pub healthy: bool,
    pub tools: Vec<DoctorTool>,
    pub capabilities: Vec<DoctorCapability>,
    pub workspaces: Vec<DoctorWorkspace>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorCapability {
    pub name: String,
    pub available: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorWorkspace {
    pub name: String,
    pub writable: bool,
    pub detail: String,
}

impl DoctorReport {
    pub fn new(tools: Vec<DoctorTool>) -> Self {
        Self {
            schema: DoctorSchema,
            healthy: tools.iter().all(|tool| tool.available),
            tools,
            capabilities: Vec::new(),
            workspaces: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

pub fn diagnose(
    spawner: &dyn ProcessSpawner,
    destination: &Path,
    temporary: &Path,
    timeout: Duration,
) -> DoctorReport {
    let ffprobe = tool_report(spawner, "ffprobe", timeout);
    let ffmpeg = tool_report(spawner, "ffmpeg", timeout);
    let ffmpeg_available = ffmpeg.available;
    let capabilities = if ffmpeg_available {
        vec![
            capability_report(
                spawner,
                "libx264",
                &["-hide_banner", "-encoders"],
                "libx264",
                timeout,
            ),
            capability_report(
                spawner,
                "mp4_muxer",
                &["-hide_banner", "-muxers"],
                " mp4 ",
                timeout,
            ),
        ]
    } else {
        vec![
            DoctorCapability {
                name: "libx264".into(),
                available: false,
                detail: "requires ffmpeg".into(),
            },
            DoctorCapability {
                name: "mp4_muxer".into(),
                available: false,
                detail: "requires ffmpeg".into(),
            },
        ]
    };
    let workspaces = vec![
        workspace_report("destination", destination),
        workspace_report("temporary", temporary),
    ];
    let mut warnings = Vec::new();
    if ffmpeg
        .version
        .as_deref()
        .and_then(ffmpeg_major)
        .is_some_and(|major| major < 6)
    {
        warnings.push(
            "Detected FFmpeg major version is older than 6; behavior may differ from CI-tested 7.x.".into(),
        );
    }
    let healthy = ffprobe.available
        && ffmpeg.available
        && capabilities.iter().all(|item| item.available)
        && workspaces.iter().all(|item| item.writable);
    DoctorReport {
        schema: DoctorSchema,
        healthy,
        tools: vec![ffprobe, ffmpeg],
        capabilities,
        workspaces,
        warnings,
    }
}

fn tool_report(spawner: &dyn ProcessSpawner, name: &str, timeout: Duration) -> DoctorTool {
    let args = vec!["-version".into()];
    match spawner.spawn(name, &args, timeout) {
        Ok(output) if output.success() && !output.stdout_truncated => {
            let version = output
                .stdout_str()
                .lines()
                .next()
                .map(|line| line.chars().take(160).collect::<String>());
            DoctorTool {
                name: name.into(),
                available: version.is_some(),
                version,
                detail: None,
            }
        }
        _ => DoctorTool {
            name: name.into(),
            available: false,
            version: None,
            detail: Some("not available on PATH or did not start successfully".into()),
        },
    }
}

fn capability_report(
    spawner: &dyn ProcessSpawner,
    name: &str,
    args: &[&str],
    marker: &str,
    timeout: Duration,
) -> DoctorCapability {
    let args = args
        .iter()
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();
    let available = spawner
        .spawn("ffmpeg", &args, timeout)
        .ok()
        .filter(|output| output.success() && !output.stdout_truncated)
        .is_some_and(|output| output.stdout_str().contains(marker));
    DoctorCapability {
        name: name.into(),
        available,
        detail: if available {
            "available".into()
        } else {
            "required FFmpeg capability is unavailable".into()
        },
    }
}

fn workspace_report(name: &str, directory: &Path) -> DoctorWorkspace {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let path = directory.join(format!(".fitifact-doctor-{}-{nonce}", std::process::id()));
    let opened = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path);
    let (writable, created) = match opened {
        Ok(mut file) => (
            std::io::Write::write_all(&mut file, b"fitifact").is_ok(),
            true,
        ),
        Err(_) => (false, false),
    };
    if created {
        let _ = std::fs::remove_file(path);
    }
    DoctorWorkspace {
        name: name.into(),
        writable,
        detail: if writable {
            "writable".into()
        } else {
            "not writable".into()
        },
    }
}

fn ffmpeg_major(version: &str) -> Option<u32> {
    let token = version.split_whitespace().nth(2)?;
    token
        .trim_start_matches(|character: char| !character.is_ascii_digit())
        .split('.')
        .next()?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Result;
    use crate::runtime::{ProcessSpawner, SpawnOutput};
    use std::time::Duration;

    struct HealthyTools;

    impl ProcessSpawner for HealthyTools {
        fn spawn(&self, program: &str, args: &[String], _timeout: Duration) -> Result<SpawnOutput> {
            let stdout = match (program, args.first().map(String::as_str)) {
                ("ffprobe", Some("-version")) => "ffprobe version 7.1\n",
                ("ffmpeg", Some("-version")) => "ffmpeg version 7.1\n",
                ("ffmpeg", _) if args.iter().any(|arg| arg == "-encoders") => {
                    " V..... libx264 H.264 encoder\n"
                }
                ("ffmpeg", _) if args.iter().any(|arg| arg == "-muxers") => " E mp4 MP4 muxer\n",
                _ => "",
            };
            Ok(SpawnOutput {
                status: 0,
                stdout: stdout.as_bytes().to_vec(),
                stderr: Vec::new(),
                stdout_truncated: false,
                stderr_truncated: false,
            })
        }
    }

    #[test]
    fn doctor_reports_tools_capabilities_and_both_write_locations() {
        let root = std::env::temp_dir().join(format!("fitifact-doctor-{}", std::process::id()));
        let destination = root.join("destination");
        let temporary = root.join("temporary");
        std::fs::create_dir_all(&destination).unwrap();
        std::fs::create_dir_all(&temporary).unwrap();
        let report = diagnose(
            &HealthyTools,
            &destination,
            &temporary,
            Duration::from_secs(5),
        );
        assert!(report.healthy);
        assert!(
            report
                .capabilities
                .iter()
                .any(|item| item.name == "libx264" && item.available)
        );
        assert!(
            report
                .capabilities
                .iter()
                .any(|item| item.name == "mp4_muxer" && item.available)
        );
        assert_eq!(report.workspaces.len(), 2);
        assert!(report.workspaces.iter().all(|item| item.writable));
        assert!(report.warnings.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
}
