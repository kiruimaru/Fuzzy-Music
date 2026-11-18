# 0005: Executable-relative portable alpha dependencies

Windows alpha uses a native Tauri release executable and a portable ZIP, without
MSI/NSIS. Stable identifier remains io.github.kiruimaru.fuzzy-music. Shell/Tauri
version is 0.1.0-alpha.1; internal source/application crates retain 0.1.0.
No Windows installer numeric-version mapping is needed for this no-bundle build.

Shared Linux convention: tools beside the current executable, independent of
cwd. FUZZY_YTDLP/FUZZY_MPV override discovery on every platform. Debug builds
retain PATH fallback; release builds use the packaged path even if missing,
preventing accidental execution of an unrelated PATH binary.

Pinned, digest-verified shinchiro baseline x86_64 mpv and official yt-dlp are
acquired directly by the user with setup-windows-runtime.ps1. The distributable
ZIP excludes external tools: the upstream mpv release does not supply a complete
matching transitive source archive, and moving recipe HEADs cannot substitute.
This adds a documented first-run setup step. The previous all-in-one developer
candidate is preserved locally. Application and vendored Rust sources and npm
notices accompany the distributed executable. Binary artifacts remain ignored.
ZIP timestamps/order are deterministic for fixed inputs, not a claim about
cross-toolchain reproducibility. WebView2 remains an external prerequisite.

Linux implementation/format validation is delegated via issue #19, linked to
the broader release automation issue #8. GitHub Actions builds/uploads ZIP+SHA
on PR/manual events with read-only contents permission; no release/tag is created.
See third-party-windows.md for provenance and redistribution boundaries.
