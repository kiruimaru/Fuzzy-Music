# Minimal Desktop Shell (Issue #14)

This slice uses Tauri 2 + React/TypeScript and the existing typed application API.
It supports search, fresh stream resolution on every Play, headless mpv audio,
Stop, retry and request cancellation. It does not implement the rest of the MVP.

## Windows Setup

Install Git, Rust stable with the MSVC toolchain, Microsoft C++ Build Tools
including the Windows SDK, Node.js 22.12+ (or a supported newer LTS), and
Microsoft Edge WebView2 Runtime. Use the existing extractor setup:

```powershell
.\scripts\setup-ytdlp.ps1
$env:FUZZY_YTDLP = (Resolve-Path .\tools\yt-dlp.exe).Path
```

Install mpv from a Windows build and put its directory on PATH, or set
`$env:FUZZY_MPV = 'C:\Tools\mpv\mpv.exe'`. Do not point it at a shell command
or include arguments. The same rule applies to `FUZZY_YTDLP`.

From the repository root:

```powershell
npm ci
npm run tauri -- info
npm run tauri -- dev
```

For a standalone development binary without a dev server:

```powershell
npm run tauri -- build --debug --no-bundle
.\target\debug\fuzzy-shell.exe
```

The tools are external runtime dependencies, not bundled into this binary.
Windows build, process/job behavior and sound are **not verified** in this session.

## Linux Setup

Install a C/C++ toolchain, Rust/Cargo, Node.js/npm, pkg-config, GTK 3,
WebKitGTK 4.1, librsvg, OpenSSL, yt-dlp and mpv using your distribution packages.
On Arch Linux, the relevant packages include `base-devel rust nodejs npm
pkgconf gtk3 webkit2gtk-4.1 librsvg openssl yt-dlp mpv`. On Debian/Ubuntu,
development headers include `libgtk-3-dev libwebkit2gtk-4.1-dev librsvg2-dev
libssl-dev build-essential pkg-config`. Then:

```bash
npm ci
npm run tauri -- info
npm run tauri -- dev
```

`FUZZY_YTDLP` and `FUZZY_MPV` optionally select executable paths; otherwise
`yt-dlp` and `mpv` are found on PATH. Overrides are read once at app startup.
`npm run dev` starts only a browser frontend at `http://127.0.0.1:5173`;
real source/playback commands require the native Tauri window.

On the tested Hyprland/GTK/WebKit environment, default rendering failed with
`Gdk ... Error 71 (Protocol error)`; forcing X11 alone produced GBM buffer errors.
The following process-local workaround successfully displayed the native window:

```bash
WEBKIT_DISABLE_DMABUF_RENDERER=1 npm run tauri -- dev
```

For the standalone debug binary:

```bash
npm run tauri -- build --debug --no-bundle
WEBKIT_DISABLE_DMABUF_RENDERER=1 ./target/debug/fuzzy-shell
```

Public SoundCloud access still depends on local network availability. The tested
machine timed out on direct connections. A pre-existing local HTTP proxy enabled
search and playback when passed through `HTTPS_PROXY`, `https_proxy` and
`http_proxy` in the app/test process environment. This is optional local setup,
not an application proxy setting; no proxy address or credentials are persisted.

## Manual Acceptance

1. Launch the native window. Search `lofi hip hop`. Confirm real titles/authors,
   Searching, and a responsive input. Edit/replace a query during search and
   clear it: an old result must not appear. Cancel must end Searching.
2. Play a result. Confirm Resolving, Starting audio, then Playing, audio from
   your output device, and no extra mpv window. Stop must terminate sound.
   Play again must perform another resolve and work without restarting.
3. Press Stop immediately after Play; wait longer than the resolve would take.
   No late response may start playback. Rapidly select different tracks.
4. Try an unavailable track, offline source/timeout, and a missing mpv executable
   (set `FUZZY_MPV` before launch). Loading must end with a safe error. Restore
   setup and relaunch if needed to change executable overrides, then retry.
   Within unchanged setup, another search/Play remains available after errors.
5. Close during Searching, Resolving and Playing. Confirm owned yt-dlp/mpv
   processes terminate and the audio stops. Check only this app's process tree;
   unrelated user-owned mpv processes must remain untouched.
6. Check empty search results when the source returns none; backend fixtures
   cover this case deterministically because live result counts vary.

## Automated Checks And Actual Results

```text
npm test
npm run test:ui
npm run build
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
bash scripts/test-ai-context.sh
bash -n scripts/ai-context.sh scripts/test-ai-context.sh
```

Browser smoke tests use fixture IPC, not Tauri or the internet. Set
`CHROMIUM_PATH` to a Chromium executable if it is not `/usr/bin/chromium`.
Screenshots are written under ignored `test-results/`.

An explicitly opt-in live test can produce sound and requires the real tools:

```bash
cargo test -p fuzzy-shell --lib live_soundcloud_play_stop_replay --locked -- --ignored --nocapture
```

On 2026-11-17, Arch Linux x86_64 checks passed with Rust/Cargo 1.99.1,
Node 26.12.0, npm 12.2.1, GTK 3.24.52, WebKitGTK 2.54.1, yt-dlp 2026.08.19,
mpv 0.41.0, Tauri Rust 2.12.6 / CLI+API 2.12.5 and React 19.3.1.
Offline Rust checks passed (20 tests; child fixture and opt-in live test ignored);
five frontend tests and browser smoke at 1000px/320px passed. Full Tauri debug
build passed. The live test passed search and two fresh resolve/play/Stop cycles
using the existing proxy. Native window search returned ten real tracks;
Play reached Playing and created an active, unmuted mpv audio-device stream;
Stop removed the owned mpv process. No human listening verification is claimed.
Closing the native window during replay also removed the app and its mpv child.
Windows was not available and remains untested.
