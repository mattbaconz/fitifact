use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{Error, ErrorCode, Result};
use crate::plan::Plan;

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub timeout: Duration,
    pub temp_dir: Option<std::path::PathBuf>,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120),
            temp_dir: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnOutput {
    pub status: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl SpawnOutput {
    pub fn stdout_str(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    pub fn stderr_str(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }

    pub fn success(&self) -> bool {
        self.status == 0
    }
}

pub trait ProcessSpawner {
    fn spawn(&self, program: &str, args: &[String], timeout: Duration) -> Result<SpawnOutput>;
}

#[derive(Debug, Default)]
pub struct SystemSpawner;

impl ProcessSpawner for SystemSpawner {
    fn spawn(&self, program: &str, args: &[String], timeout: Duration) -> Result<SpawnOutput> {
        spawn_system(program, args, timeout)
    }
}

/// Records program names so tests can prove no-op never starts ffmpeg.
#[derive(Debug)]
pub struct RecordingSpawner<S> {
    inner: S,
    pub calls: std::sync::Mutex<Vec<String>>,
}

impl<S: ProcessSpawner> RecordingSpawner<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn programs(&self) -> Vec<String> {
        self.calls.lock().expect("call log").clone()
    }

    pub fn ffmpeg_spawn_count(&self) -> usize {
        self.programs()
            .iter()
            .filter(|p| program_is_ffmpeg(p))
            .count()
    }

    pub fn ffprobe_spawn_count(&self) -> usize {
        self.programs()
            .iter()
            .filter(|p| program_is_ffprobe(p))
            .count()
    }
}

impl<S: ProcessSpawner + ?Sized> ProcessSpawner for &S {
    fn spawn(&self, program: &str, args: &[String], timeout: Duration) -> Result<SpawnOutput> {
        (**self).spawn(program, args, timeout)
    }
}

impl<S: ProcessSpawner> ProcessSpawner for RecordingSpawner<S> {
    fn spawn(&self, program: &str, args: &[String], timeout: Duration) -> Result<SpawnOutput> {
        self.calls
            .lock()
            .expect("call log")
            .push(program.to_string());
        self.inner.spawn(program, args, timeout)
    }
}

pub fn program_is_ffmpeg(program: &str) -> bool {
    let name = Path::new(program)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(program);
    name.eq_ignore_ascii_case("ffmpeg")
}

pub fn program_is_ffprobe(program: &str) -> bool {
    let name = Path::new(program)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(program);
    name.eq_ignore_ascii_case("ffprobe")
}

pub trait TransformProvider {
    fn execute(
        &self,
        input: &Path,
        output: &Path,
        plan: &Plan,
        ctx: &ExecutionContext,
    ) -> Result<()>;
}

pub fn execute(
    provider: &dyn TransformProvider,
    input: &Path,
    output: &Path,
    plan: &Plan,
    ctx: &ExecutionContext,
) -> Result<()> {
    if plan.steps.is_empty() {
        return Err(Error::new(
            ErrorCode::ExecutionFailed,
            "empty plan cannot be executed",
        ));
    }
    if same_path(input, output) {
        return Err(Error::new(
            ErrorCode::SecurityBlocked,
            "refusing to overwrite the original file",
        ));
    }
    provider.execute(input, output, plan, ctx)
}

fn same_path(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn spawn_system(program: &str, args: &[String], timeout: Duration) -> Result<SpawnOutput> {
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Duration as StdDuration, Instant};

    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| map_spawn_error(program, err))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let out_handle =
        thread::spawn(move || stdout.map(|mut s| read_all(&mut s)).unwrap_or_default());
    let err_handle =
        thread::spawn(move || stderr.map(|mut s| read_all(&mut s)).unwrap_or_default());

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = out_handle.join().unwrap_or_default();
                let stderr = err_handle.join().unwrap_or_default();
                let code = status.code().unwrap_or(-1);
                return Ok(SpawnOutput {
                    status: code,
                    stdout,
                    stderr,
                });
            }
            Ok(None) if start.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Error::new(
                    ErrorCode::ExecutionLimit,
                    format!("{program} exceeded timeout of {}s", timeout.as_secs()),
                ));
            }
            Ok(None) => thread::sleep(StdDuration::from_millis(40)),
            Err(err) => {
                return Err(Error::new(
                    ErrorCode::ExecutionFailed,
                    format!("failed waiting for {program}: {err}"),
                ));
            }
        }
    }
}

fn read_all(reader: &mut dyn std::io::Read) -> Vec<u8> {
    let mut buf = Vec::new();
    let _ = std::io::Read::read_to_end(reader, &mut buf);
    buf
}

fn map_spawn_error(program: &str, err: std::io::Error) -> Error {
    if err.kind() == std::io::ErrorKind::NotFound {
        Error::new(
            ErrorCode::ProviderMissing,
            format!("{program} was not found on PATH"),
        )
    } else {
        Error::new(
            ErrorCode::ExecutionFailed,
            format!("failed to start {program}: {err}"),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnLog {
    pub programs: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::TransformId;
    use crate::constraints::Field;
    use crate::plan::{PlanStep, StepParam};

    struct ForbiddenSpawner;

    impl ProcessSpawner for ForbiddenSpawner {
        fn spawn(
            &self,
            program: &str,
            _args: &[String],
            _timeout: Duration,
        ) -> Result<SpawnOutput> {
            panic!("should not spawn {program}");
        }
    }

    #[test]
    fn execute_refuses_overwrite() {
        let plan = Plan {
            steps: vec![PlanStep {
                id: "step-1".into(),
                transform: TransformId::Remux,
                params: vec![StepParam {
                    field: Field::MediaContainer,
                    value: "mp4".into(),
                }],
                reason: vec!["container".into()],
            }],
            preserved: Vec::new(),
        };
        let provider = crate::ffmpeg::FfmpegProvider::new(ForbiddenSpawner);
        let err = execute(
            &provider,
            Path::new("in.mp4"),
            Path::new("in.mp4"),
            &plan,
            &ExecutionContext::default(),
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::SecurityBlocked);
    }
}
