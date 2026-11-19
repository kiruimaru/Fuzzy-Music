# Task: Playlist bookmarks and optional playlist downloads

## Goal

Bookmark SoundCloud playlists like tracks, and optionally download an entire bookmarked playlist for offline playback.

## Scope

### In scope

- Persist favourite playlists in the existing SQLite library (schema v3).
- Heart actions on playlist cards, the playlist header and the Favourites view.
- A separate Download action that queues every track of a playlist through the existing single download worker.
- Per-playlist progress in Favourites and offline playback via the music library.

### Out of scope

- User-created/custom playlists and their editing.
- Automatic download when liking a playlist.
- Cancelling a whole playlist download from the UI.
- Provider work beyond public SoundCloud playlists.

## Context

- Relevant files: `src-tauri/src/library.rs`, `src-tauri/src/lib.rs`, `src/App.tsx`, `src/api.ts`.
- Related decision: `docs/decisions/0011-playlist-favourites.md`; base feature in `docs/decisions/0010-local-music-library.md`.
- Dependencies: stacked on PR #25 (`feat/favourites-local-library`), which itself follows PR #24 (`feat/issue-22-session-queue`).

## Plan

1. Add `playlists` and `playlist_tracks` tables with a v3 migration.
2. Add library/service commands for bookmarks and playlist download queueing.
3. Expose Tauri commands and frontend API bindings.
4. Add hearts, the Favourites playlist section and download progress to the UI.
5. Cover the flows with Rust and frontend tests; update docs.

## Progress

### Completed

- Schema v3, bookmark persistence, playlist counts and download queueing.
- Single-worker integration: playlist tracks reuse favourite download states, retry and offline playback; queue capacity 1024; 1000-track request cap.
- Tauri commands `playlist_entries`, `set_playlist_favourite`, `download_playlist`.
- UI: playlist card hearts, header actions, Favourites playlist section, download progress, open-from-favourites.
- Tests: Rust bookmark/download flow; frontend bookmark/download flow.

### Remaining

- Manual GUI acceptance on Windows: like, download progress, restart, offline playback.
- Real playlist download against SoundCloud through the user's proxy.
- Coordinate favourites/settings overlap with #21 owner (pre-existing PR #25 note).

## Verification

Commands and results:

```text
cargo fmt --all
clean
cargo test --workspace --all-targets --locked
54 passed, 4 ignored
npm test
32 passed
npm run tauri -- build --debug --no-bundle
build passed
```

## Handoff

- Current branch: `feat/playlist-favourites` (based on `feat/favourites-local-library` / PR #25).
- Files changed: schema/migration and playlist methods in `library.rs`; commands in `src-tauri/src/lib.rs`; `Clone` on the application `Playlist` DTO; API types and methods; App UI and styles; tests; docs.
- Known risks: playlist downloads share the single download worker, so large playlists take a long time; unlike keeps files and progress; duplicates across playlists collapse by source id.
- Next action: manual Windows GUI check, then review and publishing decision.
