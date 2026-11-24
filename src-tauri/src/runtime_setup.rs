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
    platform: &'static str,
    yt_dlp: bool,
    mpv: bool,
    ffmpeg: bool,
    can_install_packages: bool,
}

pub fn status() -> RuntimeStatus {
    RuntimeStatus {
        supported: cfg!(any(
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "windows", target_arch = "x86_64")
        )),
        platform: if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else {
            "other"
        },
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
            "Automatic runtime setup is supported on Linux x86_64 and Windows x64 only.",
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
    #[cfg(target_os = "windows")]
    {
        let _ = (install_system, ffmpeg);
        install_windows()?;
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    let _ = (install_system, ffmpeg);
    Ok(status())
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn download_agent() -> ureq::Agent {
    // Use the same environment/system proxy policy as yt-dlp and mpv.
    let proxy = fuzzy_application::NetworkPolicy::detect()
        .https_proxy()
        .ok()
        .flatten();
    ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(180)))
        .proxy(proxy)
        .build()
        .into()
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
    let timeout = dependencies::find_on_path("timeout").ok_or_else(|| {
        failure("The system timeout utility is unavailable. Install packages manually.")
    })?;
    let mut command = Command::new(pkexec);
    command
        .arg(timeout)
        .args(["--signal=TERM", "--kill-after=10s", "600s"])
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
                reap_install(child);
                return Err(failure("Could not check package installation."));
            }
            Ok(None) => {}
        }
        if CANCELLED.load(Ordering::SeqCst) || Instant::now() >= deadline {
            reap_install(child);
            return Err(failure("Runtime setup was cancelled or timed out."));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(target_os = "linux")]
fn reap_install(mut child: command_group::GroupChild) {
    // A user process cannot always signal the elevated child. The root-owned
    // timeout still bounds it; reaping must not block the UI worker or shutdown.
    let _ = child.kill();
    std::thread::spawn(move || {
        let _ = child.wait();
    });
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
    let agent: ureq::Agent = download_agent();
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

#[cfg(target_os = "windows")]
const WINDOWS_YTDLP: (&str, &str, u64) = (
    "https://github.com/yt-dlp/yt-dlp/releases/download/2026.10.26/yt-dlp.exe",
    "f5389d3698ecde4ef06f4e6b1ce0294fe9ac1396228c8990b8d3416d6d177a7d",
    128 * 1024 * 1024,
);
#[cfg(target_os = "windows")]
const WINDOWS_MPV: (&str, &str, u64) = (
    "https://github.com/shinchiro/mpv-winbuild-cmake/releases/download/20261111/mpv-x86_64-20261111-git-bcde6b93cc.7z",
    "f259b5fd4b5c395617c948a4fadcdf55bba1c5282fb7c13e88626b3f71072baf",
    128 * 1024 * 1024,
);

#[cfg(target_os = "windows")]
fn install_windows() -> Result<(), AppError> {
    use std::fs;
    let tools = windows_tools_directory()?;
    fs::create_dir_all(&tools)
        .map_err(|_| failure("Could not create the tools directory beside the application."))?;
    let agent = download_agent();
    if !status().yt_dlp {
        download_verified(
            &agent,
            WINDOWS_YTDLP.0,
            &tools.join("yt-dlp.exe"),
            WINDOWS_YTDLP.1,
            WINDOWS_YTDLP.2,
        )?;
    }
    if !status().mpv {
        let archive = tools.join("mpv.7z");
        if archive.is_file() {
            verify_file(&archive, WINDOWS_MPV.1, WINDOWS_MPV.2)?;
        } else {
            download_verified(
                &agent,
                WINDOWS_MPV.0,
                &archive,
                WINDOWS_MPV.1,
                WINDOWS_MPV.2,
            )?;
        }
        extract_mpv(&archive, &tools)?;
        if !tools.join("mpv.exe").is_file() {
            return Err(failure("The mpv archive did not contain mpv.exe."));
        }
    }
    // Keep the upstream notices next to the locally acquired tools.
    for (name, url, hash, limit) in [
        (
            "yt-dlp-LICENSE",
            "https://raw.githubusercontent.com/yt-dlp/yt-dlp/2026.10.26/LICENSE",
            "7e12e5df4bae12cb21581ba157ced20e1986a0508dd10d0e8a4ab9a4cf94e85c",
            1024 * 1024u64,
        ),
        (
            "yt-dlp-THIRD_PARTY_LICENSES.txt",
            "https://raw.githubusercontent.com/yt-dlp/yt-dlp/2026.10.26/THIRD_PARTY_LICENSES.txt",
            "472aefe951c7db35e1657c1d13fd337140511ed6f2b329205105ad441c5a02b7",
            4 * 1024 * 1024u64,
        ),
    ] {
        let target = tools.join(name);
        if !target.is_file() {
            download_verified(&agent, url, &target, hash, limit)?;
        }
    }
    let provenance = serde_json::json!({
        "mpv": { "version": "20261111-bcde6b93cc", "sha256": WINDOWS_MPV.1 },
        "ytDlp": { "version": "2026.10.26", "sha256": WINDOWS_YTDLP.1 },
    });
    fs::write(
        tools.join("runtime-provenance.json"),
        serde_json::to_vec_pretty(&provenance)
            .map_err(|_| failure("Could not write runtime provenance."))?,
    )
    .map_err(|_| failure("Could not write runtime provenance."))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_tools_directory() -> Result<std::path::PathBuf, AppError> {
    let executable =
        std::env::current_exe().map_err(|_| failure("Could not locate the application."))?;
    let folder = executable
        .parent()
        .ok_or_else(|| failure("Could not locate the application folder."))?;
    Ok(folder.join("tools"))
}

#[cfg(target_os = "windows")]
fn download_verified(
    agent: &ureq::Agent,
    url: &str,
    target: &std::path::Path,
    hash: &str,
    limit: u64,
) -> Result<(), AppError> {
    use std::io::{Read, Write};
    if target.is_file() {
        return verify_file(target, hash, limit);
    }
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
    let parent = target
        .parent()
        .ok_or_else(|| failure("Could not locate the tools directory."))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| failure("Could not stage the runtime download."))?;
    temporary
        .write_all(&bytes)
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|_| failure("Could not save the runtime download."))?;
    temporary
        .persist(target)
        .map_err(|_| failure("Could not publish the runtime download."))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn verify_file(path: &std::path::Path, expected: &str, limit: u64) -> Result<(), AppError> {
    let bytes = std::fs::read(path).map_err(|_| failure("Could not read the runtime file."))?;
    verify(&bytes, expected, limit)
}

#[cfg(target_os = "windows")]
fn extract_mpv(archive: &std::path::Path, tools: &std::path::Path) -> Result<(), AppError> {
    use std::os::windows::process::CommandExt;
    let mut command = std::process::Command::new("tar");
    command
        .args(["-xf"])
        .arg(archive)
        .arg("-C")
        .arg(tools)
        .creation_flags(0x08000000); // CREATE_NO_WINDOW
    let status = command
        .status()
        .map_err(|_| failure("Windows tar is unavailable. Extract mpv.7z manually."))?;
    if !status.success() {
        return Err(failure(
            "Could not extract mpv.7z. Extract it manually into the tools folder.",
        ));
    }
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "windows", test))]
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
    fn runtime_status_reports_the_host_platform() {
        let status = status();
        assert_eq!(
            status.platform,
            if cfg!(windows) {
                "windows"
            } else if cfg!(target_os = "linux") {
                "linux"
            } else {
                "other"
            }
        );
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        assert!(status.supported);
    }

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

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    #[test]
    #[ignore = "Downloads pinned upstream mpv and yt-dlp; requires network"]
    fn live_windows_runtime_download() {
        install_windows().unwrap();
        let tools = windows_tools_directory().unwrap();
        assert!(tools.join("yt-dlp.exe").is_file());
        assert!(tools.join("mpv.exe").is_file());
        assert!(tools.join("yt-dlp-LICENSE").is_file());
        assert!(tools.join("runtime-provenance.json").is_file());
    }
}
