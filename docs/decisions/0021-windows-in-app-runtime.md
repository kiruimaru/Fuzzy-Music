# 0021: Windows runtime setup inside the application

Date: 2026-11-24. Status: accepted for the next Windows build.

The 0.2.1 Linux AppImage exposes an explicit in-app dependency setup, while the
same Windows build still requires running `setup-windows-runtime.ps1` from a
terminal. The first project presentation flagged the platform difference: Linux
users see missing tools and an Install dependencies action; Windows users see
neither. The external script remains useful for offline and manual setup, but
the application must offer the same first-run option on Windows.

Windows x64 now shares the existing `RuntimeSetup` component. A Rust worker
downloads the same pinned releases as the PowerShell script: official yt-dlp
2026.10.26 and the shinchiro mpv 20261111 build. Both are verified with the
pinned SHA-256 before publication, and mpv.7z is extracted with the Windows
`tar` implementation into `tools` beside the executable. Upstream license
notices and a provenance JSON are kept with the tools. The HTTPS downloads use
the same environment and system proxy policy as yt-dlp and mpv, and nothing is
downloaded without an explicit click. No administrator approval is required;
the system-package checkbox remains Linux-only because Windows tools are
per-user files.

The app folder must be writable. A read-only installation reports an actionable
error instead of partially installing. Release builds resolve tools relative
to the executable, so a completed download is usable immediately. Debug builds
may have resolved a PATH fallback at startup; the UI therefore suggests a
restart if playback is still unavailable after installation. The external
PowerShell script stays in the portable ZIP and remains a supported manual
path. No runtime executable is redistributed in the application archive.
