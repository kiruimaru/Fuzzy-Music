# Task: Album view

## Goal

Make listening to albums feel intentional: when an album opens, present it with
a large cover and calm album-focused typography instead of the compact playlist
header.

## Scope

### In scope

- Expose the SoundCloud `is_album` flag through the playlist contract.
- Album banner with blurred artwork backdrop, large cover, album eyebrow,
  artist link, track count and total duration.
- Actions inside the banner: Play album, favourite, download or cancel, offline
  progress.
- Regular playlists keep the existing compact header.
- Responsive layout: banner stacks and the cover shrinks on narrow windows.
- Backward-compatible reads of playlists already stored in SQLite.

### Out of scope

- Track list redesign; albums reuse the existing rows and transition machinery.
- Album metadata beyond what the playlist response already carries.
- Backend download or playback changes.

## Context

- Relevant files: `crates/soundcloud/src/lib.rs`, `crates/soundcloud/src/web.rs`,
  `crates/application/src/lib.rs`, `src/api.ts`, `src/App.tsx`, `src/style.css`.
- Related decision: `docs/decisions/0014-album-view.md`.
- Albums were previously indistinguishable from playlists once opened; the
  artist page only classified them in its own tabs.

## Verification

- Frontend 37 tests passed, including album region, «Play album» action and
  duration rendering.
- Rust workspace tests passed: 60 passed, 4 ignored.
- TypeScript/Vite build and Tauri debug build passed.
- Manual check: open an album from an artist page and from search; the banner
  shows the large cover, blurred backdrop, artist link and Play album.
