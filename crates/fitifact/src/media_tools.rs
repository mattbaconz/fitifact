use std::path::{Path, PathBuf};

/// Resolved ffmpeg/ffprobe programs.
///
/// Desktop prefers tools shipped next to the app. CLI and tests fall back to
/// PATH names. `FITIFACT_FFMPEG` / `FITIFACT_FFPROBE` override both when they
/// point at a real file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaToolPaths {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

impl MediaToolPaths {
    pub fn path_names() -> Self {
        Self {
            ffmpeg: PathBuf::from("ffmpeg"),
            ffprobe: PathBuf::from("ffprobe"),
        }
    }

    pub fn ffmpeg_program(&self) -> String {
        self.ffmpeg.to_string_lossy().into_owned()
    }

    pub fn ffprobe_program(&self) -> String {
        self.ffprobe.to_string_lossy().into_owned()
    }

    pub fn ffmpeg_is_bundled(&self) -> bool {
        is_explicit_path(&self.ffmpeg)
    }

    pub fn ffprobe_is_bundled(&self) -> bool {
        is_explicit_path(&self.ffprobe)
    }
}

pub fn resolve_media_tools() -> MediaToolPaths {
    MediaToolPaths {
        ffmpeg: resolve_tool("FITIFACT_FFMPEG", "ffmpeg"),
        ffprobe: resolve_tool("FITIFACT_FFPROBE", "ffprobe"),
    }
}

/// Prefer ffmpeg/ffprobe files in these directories over PATH.
///
/// The desktop host calls this with the executable directory (and the Tauri
/// resource directory) so a packaged app never depends on a system install.
pub fn bind_media_tools_from_dirs<'a>(dirs: impl IntoIterator<Item = &'a Path>) {
    let folders: Vec<&Path> = dirs.into_iter().collect();
    for (key, name) in [
        ("FITIFACT_FFMPEG", "ffmpeg"),
        ("FITIFACT_FFPROBE", "ffprobe"),
    ] {
        if env_override(key).is_some() {
            continue;
        }
        if let Some(path) = folders.iter().find_map(|dir| find_tool_in_dir(dir, name)) {
            // Startup only, before the desktop host runs inspect/adapt.
            #[allow(unused_unsafe)]
            unsafe {
                std::env::set_var(key, path);
            }
        }
    }
}

fn resolve_tool(env_key: &str, name: &str) -> PathBuf {
    if let Some(path) = env_override(env_key) {
        return path;
    }
    if let Some(path) = bundled_tool(name) {
        return path;
    }
    PathBuf::from(name)
}

fn env_override(env_key: &str) -> Option<PathBuf> {
    let value = std::env::var_os(env_key)?;
    let path = PathBuf::from(value);
    path.is_file().then_some(path)
}

fn bundled_tool(name: &str) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    sidecar_search_dirs(dir)
        .into_iter()
        .find_map(|folder| find_tool_in_dir(&folder, name))
}

fn sidecar_search_dirs(exe_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![
        exe_dir.to_path_buf(),
        exe_dir.join("binaries"),
        exe_dir.join("resources"),
    ];
    if let Some(src_tauri_binaries) = tauri_src_binaries(exe_dir) {
        dirs.push(src_tauri_binaries);
    }
    dirs
}

fn tauri_src_binaries(exe_dir: &Path) -> Option<PathBuf> {
    let folder = exe_dir.file_name()?;
    if folder != "debug" && folder != "release" {
        return None;
    }
    let target = exe_dir.parent()?;
    if target.file_name()? != "target" {
        return None;
    }
    Some(target.parent()?.join("binaries"))
}

/// Only `<name>` or the Tauri sidecar name `<name>-<target triple>`, as a real executable.
fn find_tool_in_dir(dir: &Path, name: &str) -> Option<PathBuf> {
    let suffix = std::env::consts::EXE_SUFFIX;
    [
        format!("{name}{suffix}"),
        format!("{name}-{}{suffix}", env!("FITIFACT_TARGET_TRIPLE")),
    ]
    .into_iter()
    .map(|file_name| dir.join(file_name))
    .find(|path| is_executable_file(path))
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

fn is_explicit_path(path: &Path) -> bool {
    path.is_absolute() || path.components().count() > 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_names_stay_bare_commands() {
        let tools = MediaToolPaths::path_names();
        assert_eq!(tools.ffmpeg_program(), "ffmpeg");
        assert_eq!(tools.ffprobe_program(), "ffprobe");
        assert!(!tools.ffmpeg_is_bundled());
    }

    #[test]
    fn absolute_or_nested_paths_count_as_shipped_tools() {
        let tools = MediaToolPaths {
            ffmpeg: PathBuf::from("/opt/fitifact/ffmpeg"),
            ffprobe: PathBuf::from("ffprobe"),
        };
        assert!(tools.ffmpeg_is_bundled());
        assert!(!tools.ffprobe_is_bundled());
    }

    fn scratch_dir(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "fitifact-media-tools-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        ));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_executable(path: &Path) {
        std::fs::write(path, b"not-a-real-binary").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    #[test]
    fn finds_tauri_triple_named_sidecar() {
        let root = scratch_dir("triple");
        let sidecar = root.join(format!(
            "ffmpeg-{}{}",
            env!("FITIFACT_TARGET_TRIPLE"),
            std::env::consts::EXE_SUFFIX
        ));
        write_executable(&sidecar);
        assert_eq!(
            find_tool_in_dir(&root, "ffmpeg").as_deref(),
            Some(sidecar.as_path())
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ignores_lookalike_files_and_other_triples() {
        let root = scratch_dir("lookalike");
        for name in [
            "ffmpeg-notes.txt".to_string(),
            "ffmpeg-readme".to_string(),
            format!("ffmpeg-not-this-triple{}", std::env::consts::EXE_SUFFIX),
        ] {
            write_executable(&root.join(name));
        }
        std::fs::create_dir_all(root.join(format!("ffmpeg{}", std::env::consts::EXE_SUFFIX)))
            .unwrap();
        assert_eq!(find_tool_in_dir(&root, "ffmpeg"), None);
        #[cfg(unix)]
        {
            let plain = root.join(format!("ffmpeg-{}", env!("FITIFACT_TARGET_TRIPLE")));
            std::fs::write(&plain, b"no execute bit").unwrap();
            assert_eq!(find_tool_in_dir(&root, "ffmpeg"), None);
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tauri_debug_layout_includes_src_tauri_binaries() {
        let dirs = sidecar_search_dirs(Path::new("/app/src-tauri/target/debug"));
        assert!(
            dirs.iter()
                .any(|dir| dir.ends_with("src-tauri/binaries")
                    || dir.ends_with("src-tauri\\binaries"))
        );
    }
}
