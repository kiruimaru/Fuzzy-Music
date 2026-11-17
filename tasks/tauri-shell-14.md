# Task: Minimal Tauri Shell (#14)

## Goal And Scope

Implement the issue's search -> fresh resolve -> mpv -> Stop slice using Tauri 2,
React/TypeScript and the merged application DTOs. No source adapter rewrites,
design work, queue, favourites, persistence, download, packaging or CI.

## Progress

- Read issue #14 and handoff; #12/#15/#13 are already merged.
- Added `src-tauri`, `src`, npm/build config, dependency locks and minimal icons.
- Commands use blocking workers, typed DTOs and safe errors. React knows no
  source response format and never builds source or audio URLs.
- Search and playback have independent monotonic generations and cancellation.
  Both Rust and React reject stale completions; Rust also rejects reordered
  command starts. Stop invalidates resolve before killing/reaping owned mpv.
- Window close cancels both operations, waits for extractor cleanup on a worker
  and stops owned playback before exit.
- mpv uses no video/window, config or extractor; each Play resolves afresh.
  A fixed ready marker, exit status and startup deadline drive safe states.
- Added meaningful offline process/race tests, frontend regressions, browser
  smoke/screenshots and opt-in live search/play/Stop/replay test.
- Recorded decision 0003, shell command contract, setup/manual acceptance and
  actual Linux results. Source adapter/application contracts remain unchanged.

## Checks And Evidence

- `cargo test --workspace --all-targets --locked`: 20 passed, 2 ignored.
- Strict all-target Clippy, rustfmt, frontend TypeScript/build: pass.
- `npm test`: 5 passed; `npm run test:ui`: 2 passed, fixture IPC only.
- `npm run tauri -- build --debug --no-bundle`: pass on Linux.
- Context smoke and Bash syntax: pass.
- Direct live search and opt-in live test initially failed due to network timeout.
  Through the pre-existing local proxy, the opt-in live test passed real search
  and two fresh resolve/mpv play/Stop cycles.
- Native Wayland rendering initially failed with GDK protocol error; X11 alone
  produced GBM errors. `WEBKIT_DISABLE_DMABUF_RENDERER=1` displayed the real app.
- Native window search returned ten real titles/authors. Play reached Playing,
  a child mpv and active/unmuted audio-device stream were observed, and Stop
  removed the mpv process. Closing the native window during replay also removed
  both the app and its mpv child. No human listening verification is claimed.

## Remaining Acceptance And Handoff

- Windows environment was unavailable: build, WebView2, mpv window/job behavior
  and audio are not verified. Follow `docs/shell-development.md` on Windows.
- Human module owners must review contract/responsibility decisions together;
  no earlier joint approval is inferred from merged PR #15 or this implementation.
- Confirm sound by listening on the target machine. mpv's ready marker and an
  active audio-device stream do not prove a listener heard sound.
- Changes are on `feat/issue-14-tauri-shell`. No issue closure or external
  messages are performed; the above pending checks prevent claiming full
  acceptance. No source integration work is blocked.
