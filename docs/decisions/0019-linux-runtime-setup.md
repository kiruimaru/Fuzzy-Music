# 0019: Linux runtime setup beside AppImage

Date: 2026-11-23. Status: accepted for the next AppImage build.

The published AppImage dispatches to host `mpv` and `yt-dlp`. Automatically
redistributing those programs inside the image would add platform and
corresponding-source obligations; the Windows distribution already acquires
external runtime tools from upstream at first setup instead.

The next Linux AppImage build will publish a setup script beside the image.
It downloads the upstream x86_64 `yt-dlp` release with a pinned SHA256 and
retains its upstream license notices in a sibling `tools` directory. It
installs `mpv` through pacman, apt, dnf or zypper, requesting elevation only
when needed. Optional `--ffmpeg` installs the FLAC converter. The AppImage
wrapper prefers the adjacent verified `yt-dlp`, then falls back to host PATH;
mpv remains a host package. No tool is silently installed at app startup.

The existing v0.1.0 AppImage and release assets are immutable and remain
unchanged. The release workflow must use a new application version to publish
the updated wrapper. Windows continues using its included PowerShell bootstrap.
