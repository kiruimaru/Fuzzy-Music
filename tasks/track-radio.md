# Task: Track radio

## Goal

Let any SoundCloud track start a continuous radio queue: the seed plays at once,
related tracks are appended automatically, and playback keeps moving to the next
queue entry without manual intervention.

## Scope

### In scope

- Related-tracks web endpoint for a SoundCloud track id, bounded to 25 entries.
- Radio action on every SoundCloud track row in search results, artist pages,
  playlists, Favourites and the music library.
- Queue extension near the end of the queue, with deduplication by source and id
  and a retry throttle for empty or failed pages.
- Radio lifecycle: starting another list, selecting a session-queue entry or
  pressing Stop ends radio mode and cancels pending requests.
- Tests for the initial page and for queue extension beyond the first page.

### Out of scope

- Persisting radio sessions, saved stations, personalisation or listening
  history.
- Radio for local files; the action is hidden for them.
- Additional recommendation sources beyond SoundCloud related tracks.

## Context

- Relevant files: `crates/soundcloud/src/lib.rs`, `crates/soundcloud/src/web.rs`,
  `crates/application/src/lib.rs`, `src-tauri/src/lib.rs`, `src/api.ts`,
  `src/App.tsx`, `src/style.css`.
- Related decision: `docs/decisions/0017-track-radio.md`.
- The endpoint `/tracks/{id}/related` was verified live through the local proxy
  before implementation.
- Queue rows keep using the existing playlist transitions, prefetch budget and
  playback recovery.

## Verification

- Frontend 40 tests passed, including starting radio and queue growth.
- Rust workspace tests passed: 63 passed, 4 ignored.
- rustfmt and strict clippy clean.
- TypeScript/Vite build and Tauri debug build passed (CSS 21.52 kB, JS 275.82 kB).
- Manual check: start radio from a search result and from a playlist; let the
  queue run across the first page and confirm the next page is appended.
