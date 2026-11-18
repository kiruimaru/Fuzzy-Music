# Fuzzy Music Windows 0.1.0-alpha.1

RU: Распакуйте весь ZIP в доступный для записи каталог. Двойной щелчок по
`Fuzzy Music.exe` запускает поиск публичных SoundCloud треков, Play и Stop.
Перед первым запуском откройте PowerShell в распакованном каталоге и выполните
`powershell -NoProfile -ExecutionPolicy Bypass -File .\setup-windows-runtime.ps1`.
Это разовая загрузка проверенных зависимостей непосредственно у upstream;
параметр Bypass действует только на этот процесс. Не переносите EXE отдельно
от созданного tools. При сбое загрузки проверьте доступ к GitHub и повторите.
Интернет необходим. Это ранняя alpha:
есть управление воспроизведением и очередь треков открытого плейлиста;
избранное, сохранение настроек и отдельный редактор очереди ещё не реализованы.
Используется системный ручной прокси Windows; после его изменения перезапустите
приложение. PAC/WPAD не поддерживается.

EN: Extract the entire ZIP to a writable directory, then double-click
`Fuzzy Music.exe`. First run the PowerShell setup command above to download
digest-verified tools directly from upstream. Keep the tools directory alongside it. Public SoundCloud
search and streaming require internet access. Pause, seek, volume and playlist
queue playback are implemented; favourites, persisted settings and independent
queue editing remain unfinished. Windows manual system proxy is detected at
startup; restart after changes. PAC/WPAD is unsupported.

RU: Если SoundCloud недоступен напрямую, включите свой VPN/системный ручной
прокси до запуска приложения. Приложение не устанавливает VPN и не меняет
сетевые настройки Windows. Ошибка timeout может означать недоступность сервиса;
отсутствующие или повреждённые инструменты требуют повторной установки tools.

## Download and disk sizes

Measured locally with the same 9,180,160-byte release executable: old combined
ZIP 81,459,498 bytes, 556,969,597 bytes unpacked; split portable ZIP approximately
2,888,036 bytes (2.75 MiB), 9,572,175 bytes unpacked (9.13 MiB). Documentation
and provenance changes can slightly change these numbers; CI prints exact sizes
and checksums for each build. Corresponding source is about 78.3 MB compressed /
547.4 MB unpacked, downloaded separately only when needed.

First setup additionally downloads mpv 34,181,485 bytes and yt-dlp 23,575,744 bytes
plus notices (about 58.0 MB total). Measured installed tools occupy 185,836,978
bytes, including retained upstream mpv.7z; portable app + tools approximately
195,409,153 bytes (186.36 MiB). WebView2, if missing, is an additional separate
Microsoft download. These are file-byte totals, not filesystem allocation sizes.

## Requirements / Требования

Windows x64, Microsoft Edge WebView2 Evergreen Runtime (installed separately if
missing): https://developer.microsoft.com/microsoft-edge/webview2/
No Rust, Node, Python or developer toolchain is required at runtime.
Clean Windows/Sandbox verification is pending; do not interpret local developer
machine checks as proof of clean-machine compatibility. Runtime DLL imports and
minimum Windows version still need external validation before public release.

## Build / Сборка

From the repository root, with Rust MSVC, Windows SDK, Node/npm installed:
`npm ci`, then `powershell -NoProfile -File scripts/package-windows.ps1`.
Windows tar must support 7z extraction. Downloads are pinned and SHA256 checked.
Output is under ignored `artifacts/windows-alpha` (outside Vite's cleared dist).
Download the **portable** Actions artifact for running the app. The separate
**corresponding-source** artifact on the same workflow run supplies the matching
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

Release builds discover tools relative to their executable (`tools/yt-dlp.exe`,
`tools/mpv.exe`), independently of cwd/PATH. Development overrides `FUZZY_YTDLP`
and `FUZZY_MPV` remain supported on Windows and Linux.
