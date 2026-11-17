use std::path::{Path, PathBuf};

// Overrides remain available for development on every platform. A portable
// distribution must never depend on its working directory or the user's PATH.
pub fn executable(variable: &str, name: &str) -> PathBuf {
    if let Some(value) = std::env::var_os(variable) {
        return value.into();
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
