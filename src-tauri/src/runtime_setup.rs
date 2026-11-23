use crate::dependencies;
use fuzzy_application::{AppError, ErrorCode};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};

static INSTALLING: AtomicBool = AtomicBool::new(false);
static CANCELLED: AtomicBool = AtomicBool::new(false);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    supported: bool,
    yt_dlp: bool,
    mpv: bool,
    ffmpeg: bool,
    can_install_packages: bool,
}

pub fn status() -> RuntimeStatus {
    RuntimeStatus {
        supported: cfg!(all(target_os = "linux", target_arch = "x86_64")),
        yt_dlp: available("FUZZY_YTDLP", "yt-dlp"),
        mpv: available("FUZZY_MPV", "mpv"),
        ffmpeg: dependencies::find_on_path("ffmpeg").is_some(),
        can_install_packages: cfg!(target_os = "linux")
            && dependencies::find_on_path("pkexec").is_some()
            && package_manager().is_some(),
    }
}

fn available(variable: &str, name: &str) -> bool {
    #[cfg(target_os = "linux")]
    if std::env::var_os(variable).is_none() {
        return dependencies::find_on_path(name).is_some()
            || (name == "yt-dlp"
                && dependencies::runtime_directory()
                    .is_some_and(|directory| dependencies::is_executable(&directory.join(name))));
    }
    let selected = dependencies::executable(variable, name);
    if selected.components().count() == 1 {
        dependencies::find_on_path(name).is_some()
    } else {
        dependencies::is_executable(&selected)
    }
}

fn failure(message: &str) -> AppError {
    AppError {
        code: ErrorCode::SourceUnavailable,
        message: message.into(),
        retryable: true,
    }
}

pub fn cancel() {
    CANCELLED.store(true, Ordering::SeqCst);
}

struct InstallGuard;
impl Drop for InstallGuard {
    fn drop(&mut self) {
        INSTALLING.store(false, Ordering::SeqCst);
    }
}

pub fn install(install_system: bool, ffmpeg: bool) -> Result<RuntimeStatus, AppError> {
    if !status().supported {
        return Err(failure(
            "Automatic runtime setup is supported on Linux x86_64 only.",
        ));
    }
    if INSTALLING.swap(true, Ordering::SeqCst) {
        return Err(failure("Runtime setup is already running."));
    }
    let _guard = InstallGuard;
    CANCELLED.store(false, Ordering::SeqCst);
    #[cfg(target_os = "linux")]
    {
        if !status().yt_dlp {
            download_extractor()?;
        }
        if install_system {
            install_packages(ffmpeg)?;
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = (install_system, ffmpeg);
    Ok(status())
}

fn package_manager() -> Option<&'static str> {
    ["pacman", "apt-get", "dnf", "zypper"]
        .into_iter()
        .find(|name| dependencies::find_on_path(name).is_some())
}

#[cfg(any(target_os = "linux", test))]
fn package_arguments(manager: &str, mpv: bool, ffmpeg: bool) -> Vec<&'static str> {
    let mut arguments = match manager {
        "pacman" => vec!["-S", "--needed", "--noconfirm"],
        "apt-get" => vec!["install", "-y", "--no-install-recommends"],
        "dnf" => vec!["install", "-y"],
        "zypper" => vec!["--non-interactive", "install"],
        _ => vec![],
    };
    if mpv {
        arguments.push("mpv");
    }
    if ffmpeg {
        arguments.push("ffmpeg");
    }
    arguments
}

#[cfg(target_os = "linux")]
fn install_packages(ffmpeg: bool) -> Result<(), AppError> {
    use command_group::CommandGroup;
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let current = status();
    let mpv = !current.mpv;
    let ffmpeg = ffmpeg && !current.ffmpeg;
    if !mpv && !ffmpeg {
        return Ok(());
    }
    let manager = package_manager()
        .ok_or_else(|| failure("Install mpv with your distribution's package manager."))?;
    let executable = dependencies::find_on_path(manager)
        .ok_or_else(|| failure("Package manager is unavailable."))?;
    let pkexec = dependencies::find_on_path("pkexec")
        .ok_or_else(|| failure("Administrator approval is unavailable. Install mpv with your distribution's package manager."))?;
    let mut command = Command::new(pkexec);
    command
        .arg(executable)
        .args(package_arguments(manager, mpv, ffmpeg))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for variable in ["LD_LIBRARY_PATH", "LD_PRELOAD", "PYTHONHOME", "PYTHONPATH"] {
        command.env_remove(variable);
    }
    let mut child = command
        .group_spawn()
        .map_err(|_| failure("Could not request administrator approval."))?;
    let deadline = Instant::now() + Duration::from_secs(600);
    loop {
        match child.try_wait() {
            Ok(Some(exit)) if exit.success() => return Ok(()),
            Ok(Some(_)) => {
                return Err(failure(
                    "Package installation was declined or failed. Retry or install mpv with your package manager.",
                ));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(failure("Could not check package installation."));
            }
            Ok(None) => {}
        }
        if CANCELLED.load(Ordering::SeqCst) || Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(failure("Runtime setup was cancelled or timed out."));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(target_os = "linux")]
fn download_extractor() -> Result<(), AppError> {
    let directory = dependencies::runtime_directory()
        .ok_or_else(|| failure("User data directory is unavailable."))?;
    // Explicit overrides belong to the user; never replace them during setup.
    if std::env::var_os("FUZZY_YTDLP").is_some() {
        return Err(failure(
            "The configured FUZZY_YTDLP executable is unavailable. Repair or remove that override.",
        ));
    }
    download_extractor_to(&directory)
}

#[cfg(target_os = "linux")]
fn download_extractor_to(directory: &std::path::Path) -> Result<(), AppError> {
    use std::{
        fs,
        io::{Read, Write},
        os::unix::fs::PermissionsExt,
        time::Duration,
    };
    fs::create_dir_all(directory)
        .map_err(|_| failure("Could not create the runtime directory."))?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(180)))
        .build()
        .into();
    let files = [
        (
            "yt-dlp-LICENSE",
            "https://raw.githubusercontent.com/yt-dlp/yt-dlp/2026.10.26/LICENSE",
            "7e12e5df4bae12cb21581ba157ced20e1986a0508dd10d0e8a4ab9a4cf94e85c",
            1024 * 1024,
        ),
        (
            "yt-dlp-THIRD_PARTY_LICENSES.txt",
            "https://raw.githubusercontent.com/yt-dlp/yt-dlp/2026.10.26/THIRD_PARTY_LICENSES.txt",
            "472aefe951c7db35e1657c1d13fd337140511ed6f2b329205105ad441c5a02b7",
            4 * 1024 * 1024,
        ),
        (
            "yt-dlp",
            "https://github.com/yt-dlp/yt-dlp/releases/download/2026.10.26/yt-dlp_linux",
            "767ba9dce7052aa196f8b12652f433d76de128ba168966fa84b6394d5e6c6795",
            128 * 1024 * 1024,
        ),
    ];
    for (name, url, hash, limit) in files {
        if CANCELLED.load(Ordering::SeqCst) {
            return Err(failure("Runtime setup was cancelled."));
        }
        let mut response = agent
            .get(url)
            .call()
            .map_err(|_| failure("Runtime download failed. Check your connection and retry."))?;
        let mut bytes = Vec::new();
        let mut reader = response.body_mut().as_reader();
        let mut chunk = [0u8; 64 * 1024];
        loop {
            if CANCELLED.load(Ordering::SeqCst) {
                return Err(failure("Runtime setup was cancelled."));
            }
            let count = reader
                .read(&mut chunk)
                .map_err(|_| failure("Runtime download was interrupted. Retry setup."))?;
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..count]);
            if bytes.len() as u64 > limit {
                return Err(failure("Runtime download exceeded its size limit."));
            }
        }
        verify(&bytes, hash, limit)?;
        if CANCELLED.load(Ordering::SeqCst) {
            return Err(failure("Runtime setup was cancelled."));
        }
        let mut temporary = tempfile::NamedTempFile::new_in(directory)
            .map_err(|_| failure("Could not stage the runtime download."))?;
        temporary
            .write_all(&bytes)
            .and_then(|()| temporary.as_file().sync_all())
            .map_err(|_| failure("Could not save the runtime download."))?;
        fs::set_permissions(
            temporary.path(),
            fs::Permissions::from_mode(if name == "yt-dlp" { 0o755 } else { 0o644 }),
        )
        .map_err(|_| failure("Could not set runtime permissions."))?;
        temporary
            .persist(directory.join(name))
            .map_err(|_| failure("Could not publish the runtime download."))?;
    }
    Ok(())
}

#[cfg(any(target_os = "linux", test))]
fn verify(bytes: &[u8], expected: &str, limit: u64) -> Result<(), AppError> {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if bytes.len() as u64 > limit || digest != expected {
        return Err(failure(
            "Runtime download failed its integrity check. Nothing was installed.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrity_rejects_corrupt_and_oversized_downloads() {
        let hash = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(verify(b"abc", hash, 3).is_ok());
        assert!(verify(b"abcd", hash, 4).is_err());
        assert!(verify(b"abc", hash, 2).is_err());
    }

    #[test]
    fn packages_are_fixed_and_optional() {
        assert_eq!(
            package_arguments("pacman", true, false),
            ["-S", "--needed", "--noconfirm", "mpv"]
        );
        assert_eq!(
            package_arguments("apt-get", false, true),
            ["install", "-y", "--no-install-recommends", "ffmpeg"]
        );
        assert!(!package_arguments("dnf", true, false).contains(&"ffmpeg"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "Downloads pinned upstream yt-dlp and notices; requires network"]
    fn live_verified_runtime_download() {
        let directory = tempfile::tempdir().unwrap();
        download_extractor_to(directory.path()).unwrap();
        assert!(directory.path().join("yt-dlp-LICENSE").is_file());
        assert!(
            directory
                .path()
                .join("yt-dlp-THIRD_PARTY_LICENSES.txt")
                .is_file()
        );
        let output = std::process::Command::new(directory.path().join("yt-dlp"))
            .arg("--version")
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            "2026.10.26"
        );
    }
}
