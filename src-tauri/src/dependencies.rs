use std::path::{Path, PathBuf};

// Overrides remain available for development on every platform. A portable
// distribution must never depend on its working directory or the user's PATH.
pub fn executable(variable: &str, name: &str) -> PathBuf {
    if let Some(value) = std::env::var_os(variable) {
        return value.into();
    }
    #[cfg(target_os = "linux")]
    if std::env::var_os("APPIMAGE").is_some()
        && let Ok(app) = std::env::current_exe()
        && let Some(parent) = app.parent()
    {
        let packaged = beside(parent, name);
        if packaged.is_file() {
            return packaged;
        }
    }
    #[cfg(target_os = "linux")]
    {
        if name == "yt-dlp"
            && let Some(directory) = runtime_directory()
        {
            let managed = directory.join(name);
            if managed.is_file() || find_on_path(name).is_none() {
                return managed;
            }
        }
        if let Some(host) = find_on_path(name) {
            return host;
        }
        // A missing player can be installed from the running application's UI.
        if name == "mpv" {
            return name.into();
        }
    }
    if let Ok(app) = std::env::current_exe()
        && let Some(parent) = app.parent()
    {
        let packaged = beside(parent, name);
        if packaged.is_file() || cfg!(not(debug_assertions)) {
            return packaged;
        }
    }
    name.into()
}

pub fn find_on_path(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?).find_map(|directory| {
        let path = directory.join(name);
        if is_executable(&path) {
            Some(path)
        } else {
            None
        }
    })
}

pub fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    true
}

#[cfg(target_os = "linux")]
pub fn runtime_directory() -> Option<PathBuf> {
    let root = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
        })?;
    Some(root.join("io.github.kiruimaru.fuzzy-music/runtime/tools"))
}

fn beside(parent: &Path, name: &str) -> PathBuf {
    parent.join("tools").join(if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_layout_preserves_spaces() {
        let path = beside(Path::new("a directory/app"), "mpv");
        assert_eq!(path.parent().unwrap(), Path::new("a directory/app/tools"));
        assert_eq!(path.file_stem().unwrap(), "mpv");
    }
}
