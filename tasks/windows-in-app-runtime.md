# Task: Windows in-app runtime setup

## Goal

Give Windows the same explicit first-run dependency option as Linux, without
redistributing external executables inside the application archive.

## Scope

### In scope

- Report the host platform and support Windows x64 in `RuntimeStatus`.
- Download pinned yt-dlp.exe and shinchiro mpv.7z with SHA-256 verification.
- Extract mpv with the system `tar` into `tools` beside the executable.
- Keep yt-dlp license notices and a provenance JSON with the tools.
- Use the existing network proxy policy for downloads.
- Show the shared dependency panel and Install dependencies action on Windows.
- Keep the system-package consent option Linux-only.
- UI and live-download tests.

### Out of scope

- Bundling or redistributing mpv, yt-dlp or ffmpeg in the ZIP or AppImage.
- Installing WebView2, codecs or optional ffmpeg automatically on Windows.
- An installer, service, updater or silent background installation.
- Removing the existing `setup-windows-runtime.ps1` manual path.

## Context

- Relevant files: `src-tauri/src/runtime_setup.rs`, `src/RuntimeSetup.tsx`,
  `src/api.ts`, `src/runtime-setup.test.tsx`, `scripts/setup-windows-runtime.ps1`.
- Related decision: `docs/decisions/0021-windows-in-app-runtime.md`.
- Pinned Windows releases and hashes match the PowerShell setup script.

## Verification

- Frontend 46 tests passed, including the Windows panel and install call.
- Rust runtime tests passed; the live Windows download test downloaded,
  verified and extracted mpv and yt-dlp successfully.
- rustfmt and strict clippy clean.
- Tauri debug build passed.
- Manual check: launch a Windows build without tools, install from Settings,
  confirm `tools/mpv.exe` and `tools/yt-dlp.exe`, then play a track.
