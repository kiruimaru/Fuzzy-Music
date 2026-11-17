# Fuzzy Music Windows 0.1.0-alpha.1

RU: Распакуйте весь ZIP в доступный для записи каталог. Двойной щелчок по
`Fuzzy Music.exe` запускает поиск публичных SoundCloud треков, Play и Stop.
Перед первым запуском откройте PowerShell в распакованном каталоге и выполните
`powershell -NoProfile -ExecutionPolicy Bypass -File .\setup-windows-runtime.ps1`.
Это разовая загрузка проверенных зависимостей непосредственно у upstream;
параметр Bypass действует только на этот процесс. Не переносите EXE отдельно
от созданного tools. При сбое загрузки проверьте доступ к GitHub и повторите.
Интернет необходим. Это ранняя alpha:
очередь, избранное, пауза, перемотка и настройки ещё не реализованы.
Используется системный ручной прокси Windows; после его изменения перезапустите
приложение. PAC/WPAD не поддерживается.

EN: Extract the entire ZIP to a writable directory, then double-click
`Fuzzy Music.exe`. First run the PowerShell setup command above to download
digest-verified tools directly from upstream. Keep the tools directory alongside it. Public SoundCloud
search, Play and Stop require internet access. Queue, favourites, pause, seek
and settings are not implemented. Windows manual system proxy is detected at
startup; restart after changes. PAC/WPAD is unsupported.

RU: Если SoundCloud недоступен напрямую, включите свой VPN/системный ручной
прокси до запуска приложения. Приложение не устанавливает VPN и не меняет
сетевые настройки Windows. Ошибка timeout может означать недоступность сервиса;
отсутствующие или повреждённые инструменты требуют повторной установки tools.

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
Output is under ignored `artifacts/windows-alpha` (outside Vite's cleared dist), with a SHA256 sidecar. Staging must not
already exist. ZIP includes corresponding application/Rust sources and notices;
external runtime binaries are acquired separately (see THIRD-PARTY.md).
`-SkipBuild` packages the already built release EXE.
ZIP ordering and timestamps are fixed; native compiler output is not claimed
bit-for-bit reproducible across toolchains. No installer/MSI is used.

Release builds discover tools relative to their executable (`tools/yt-dlp.exe`,
`tools/mpv.exe`), independently of cwd/PATH. Development overrides `FUZZY_YTDLP`
and `FUZZY_MPV` remain supported on Windows and Linux.
