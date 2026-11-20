# Fuzzy Music Windows 0.1.0

RU: Распакуйте весь ZIP в доступный для записи каталог. Двойной щелчок по
`Fuzzy Music.exe` запускает поиск публичных SoundCloud треков, Play и Stop.
Перед первым запуском откройте PowerShell в распакованном каталоге и выполните
`powershell -NoProfile -ExecutionPolicy Bypass -File .\setup-windows-runtime.ps1`.
Это разовая загрузка проверенных зависимостей непосредственно у upstream;
параметр Bypass действует только на этот процесс. Не переносите EXE отдельно
от созданного tools. При сбое загрузки проверьте доступ к GitHub и повторите.
Интернет необходим для SoundCloud. Реализованы управление воспроизведением,
очередь, избранные треки и плейлисты, настройки, загрузки и локальная медиатека.
Для преобразования в FLAC отдельно установите `ffmpeg` в PATH.
Используется системный ручной прокси Windows; после его изменения перезапустите
приложение. PAC/WPAD не поддерживается.

EN: Extract the entire ZIP to a writable directory, then double-click
`Fuzzy Music.exe`. First run the PowerShell setup command above to download
digest-verified tools directly from upstream. Keep the tools directory alongside it. Public SoundCloud
search and streaming require internet access. Pause, seek, volume and playlist
queue playback, favourites, persisted settings, downloads and a local library
are implemented. FLAC conversion additionally requires `ffmpeg` on PATH.
Windows manual system proxy is detected at
startup; restart after changes. PAC/WPAD is unsupported.

RU: Если SoundCloud недоступен напрямую, включите свой VPN/системный ручной
прокси до запуска приложения. Приложение не устанавливает VPN и не меняет
сетевые настройки Windows. Ошибка timeout может означать недоступность сервиса;
отсутствующие или повреждённые инструменты требуют повторной установки tools.

## Downloads

Use `Fuzzy-Music-0.1.0-windows-x64.zip` to run the app. A separate
`Fuzzy-Music-0.1.0-windows-x64.exe` is also published with
`setup-windows-runtime.ps1`; it is the same EXE from the ZIP, not a standalone
installer. Keep both files together while setting up the tools. The matching
`Fuzzy-Music-0.1.0-corresponding-source.zip` is a separate source download,
not needed for normal playback. Exact sizes and SHA256 checksums are emitted
by the workflow and published with the release.

First setup additionally downloads mpv 34,181,485 bytes and yt-dlp 23,575,744 bytes
plus notices (about 58.0 MB total). Measured installed tools occupy 185,836,978
bytes, including retained upstream mpv.7z, in addition to the portable app.
WebView2, if missing, is an additional separate Microsoft
download. Runtime figures are historical file-byte measurements, not a current
application size or filesystem allocation guarantee.

## Requirements / Требования

Windows x64, Microsoft Edge WebView2 Evergreen Runtime (installed separately if
missing): https://developer.microsoft.com/microsoft-edge/webview2/
No Rust, Node, Python or developer toolchain is required at runtime.
Clean Windows/Sandbox verification is pending; do not interpret local developer
machine checks as proof of clean-machine compatibility. Runtime DLL imports and
minimum Windows version remain unverified in this first public release.
The portable executable is unsigned; there is no installer.

## Build / Сборка

From the repository root, with Rust MSVC, Windows SDK, Node/npm installed:
`npm ci`, then `powershell -NoProfile -File scripts/package-windows.ps1`.
Windows tar must support 7z extraction. Downloads are pinned and SHA256 checked.
Output is under ignored `artifacts/windows-alpha` (outside Vite's cleared dist).
Download the **portable** release ZIP or Actions artifact for running the app. The separate
**corresponding-source** download on the same release/workflow run supplies the matching
application/build scripts, locked Rust vendor sources/licenses and frontend
runtime sources. Keep both available together when redistributing the app.
Each ZIP has a SHA256 sidecar. Both contain SOURCE-PROVENANCE.json identifying
the Git commit/tree, working-tree dirty flag and matching executable digest.
Staging must not already exist. External runtime binaries are acquired separately
(see THIRD-PARTY.md). Actions adds a small outer ZIP containing ZIP + checksum;
the portable artifact never contains the source archive.
`-SkipBuild` packages the already built release EXE.
ZIP ordering and timestamps are fixed; native compiler output is not claimed
bit-for-bit reproducible across toolchains. No installer/MSI is used.

The `Windows build and release` workflow (`windows-alpha.yml`, retained filename)
checks PRs and manual builds. A `vMAJOR.MINOR.PATCH` tag whose version matches
npm, Tauri and Rust triggers a full build and publishes a GitHub Release only
after both archives and their checksums pass validation. Release notes live in
`docs/releases/<tag>.md`. Do not move a published tag or replace its assets.

Release builds discover tools relative to their executable (`tools/yt-dlp.exe`,
`tools/mpv.exe`), independently of cwd/PATH. Development overrides `FUZZY_YTDLP`
and `FUZZY_MPV` remain supported on Windows and Linux.
