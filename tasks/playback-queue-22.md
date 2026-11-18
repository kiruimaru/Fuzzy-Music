# Task: Playback controls and session queue (#22)

## Goal

Complete the session queue and remaining playback integration on top of merged
PRs #20/#23. Preserve source playlist playback and request generations.

## Scope

- Add search results to an editable in-memory queue; select, remove, Previous/Next.
- Advance at EOF without wrapping, stop after the last entry, preserve the list
  on Stop/search, and wait for user choice after errors.
- Stop/cancel on deleting current, with stable keys for duplicate tracks.
- Resolve fresh at each session entry launch; reject late resolve/status/EOF.
- Load/save preferred volume through the typed #21 settings port. Until that API
  exists, use an injectable memory adapter without another persistence store.
- Preserve actual mpv pause/resume, seekability, status and existing source
  playlist transitions. No packaging, favourites, SQLite or cache redesign.

## Implementation

- Branch: `feat/issue-22-session-queue`.
- `src/App.tsx`: editable session queue separate from public source sequences,
  generation guards, no automatic retry after preparation/track error.
- `src/settings.ts`: typed settings port; memory adapter pending #21.
- `src/api.ts`, Rust shell/player: fresh launch and preferred-volume parameters;
  fresh selection drops old player before resolve; startup volume is an mpv arg.
- Decision 0009 and contracts record queue semantics and proposed #21 IPC names.

## Verification

- Locked Rust workspace/all-target tests: 47 passed, 4 ignored.
- Strict all-target Clippy: passed.
- Frontend tests: 26 passed; TypeScript/Vite build passed.
- Real mpv local WAV with null audio output: initial volume 35, pause/seek/volume/
  resume, EOF and same-process replay passed. This does not prove audibility.
- Chromium fixture UI tests: six passed, including queue/EOF/removal at 1000px
  and 320px. Screenshots inspected; no horizontal overflow or player overlap.
- Native Tauri debug no-bundle build passed; context smoke, Bash syntax,
  rustfmt and diff whitespace checks passed.
- Playwright initially lacked its browser; installed its headless Chromium,
  then `npm run test:ui` passed without a browser-path override.

## Remaining Acceptance

- Replace memory settings port when #21 provides persistence. Proposed commands
  and ownership are in the contracts; contributor agreement is not claimed.
- Human audio scenario: search, add three, select, pause/resume, seek, volume,
  Next/Previous, EOF and Stop; confirm no mpv remains after Stop/window close.
- Windows verification of this change remains separate from Linux evidence.
- Publish focused PR and obtain Maroi9p review. Keep #22 open until outstanding
  acceptance is recorded; avoid an automatic closing keyword in the PR.
- Earlier cache/worker/HTTP and Linux packaging findings remain tracked in
  `pr20-pr23-integration.md`; this task does not represent them as fixed.
