# Task: Basic playback controls

## Scope

Local implementation requested by the user: a default player UI with genuine
pause/resume, seek, and volume. No queue, favourites, persisted settings, source
adapter rewrite, packaging, issue closure, or publication.

## Implementation

- `src-tauri/src/playback.rs`: owned mpv JSON IPC transport, property observers,
  pause/seek/volume commands and safe status/error handling.
- `src-tauri/src/lib.rs`: expanded status and generation-checked update command.
- `src/App.tsx`, `src/api.ts`, `src/style.css`: dark baseline player panel,
  keyboard-accessible timeline and volume, actual paused state.
- Offline fixture IPC tests, UI regressions, optional real-mpv local-audio test.
- Playwright uses its installed browser unless CHROMIUM_PATH is explicitly set.

## Checks and remaining work

- Frontend: six tests passed; TypeScript/Vite build passed.
- Browser: two fixture smoke tests passed at 1000px and 320px including controls.
- Real mpv 0.42 local WAV check (`--ignored` opt-in): pause froze position,
  seek to 5s applied while paused, volume 25 applied, resume continued in the
  same owned process.
- Native debug Tauri build passed.
- The unrelated live SoundCloud test failed only because yt-dlp was absent from
  PATH; it was not treated as playback evidence.
- Locked workspace tests: 25 passed, 2 ignored (live SoundCloud, real mpv).
- Strict all-target Clippy and rustfmt: pass.
- Total: `cargo test` 25 passed, 2 ignored; frontend 6, browser 2.
- Linux native transport and human listening must be checked separately.
- Existing issue #22 overlaps this partial implementation: coordinate before
  publication; queue is not implemented and the issue must not be closed.
- No commit/push/PR is authorized by this task.
