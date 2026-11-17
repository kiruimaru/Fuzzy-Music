# Fuzzy Music Project Context

**Last reviewed:** 2026-11-17  
**Status:** project foundation

## Product

Fuzzy Music is a Windows and Linux desktop player for searching and streaming
publicly available SoundCloud tracks without requiring the user to manually
obtain an API key.

## MVP

The first release must support:

- search by track title and author;
- title, author, cover art, and duration metadata;
- play, pause, seek, and volume;
- a playback queue and track switching;
- local favourites;
- persisted settings.

Playlists, history, and additional themes are post-MVP.

## Architecture

The application is a local modular desktop app:

```text
React + TypeScript UI
          |
      Tauri 2 commands
          |
Rust application core
  |        |          |
source   playback   persistence
adapter  engine     and settings
```

There is no application server planned for the first release. The current
technical choices are Rust, Tauri 2, React, TypeScript, CSS, yt-dlp, mpv,
SQLite, Git, and GitHub.

The frontend/Rust contract is documented in
[`architecture/contracts.md`](architecture/contracts.md). Keep source-specific
response formats inside the source adapter.

## Known state

- AI context assembly has offline regression coverage in
  `scripts/test-ai-context.sh`; see `tasks/ai-context-11.md` for scope and limits.
- The repository and local Rust environment were prepared.
- A Windows Rust prototype reportedly completed SoundCloud search, stream
  discovery, timeout handling, and cancellation checks.
- The reported prototype suite had eight passing automated tests.
- Playback integration and the Tauri shell have not yet been added to this
  repository.

These prototype results are historical project input. They must be re-run or
ported into this repository before being treated as CI evidence.

## Agreed work split

The source-integration owner is responsible for SoundCloud search, normalized
metadata, stream discovery, source errors, and later local favourites.

The application-shell owner is responsible for the Tauri 2 shell, React +
TypeScript interface, mpv integration, and build preparation.

Before connecting the modules, both owners must agree on the contracts in
[`architecture/contracts.md`](architecture/contracts.md).

## Delivery order

1. Confirm source connectivity and keep the source adapter isolated.
2. Create the minimal Tauri application and search states.
3. Validate mpv playback without a separate window.
4. Connect search results to playback.
5. Add queue, favourites, settings, and cross-platform checks.
6. Test and prepare Windows and Linux builds.

## Risks

- SoundCloud internals or available stream formats can change.
- Individual tracks may be unavailable or region-restricted.
- Search and stream discovery may be slow.
- mpv process behavior can differ between Windows and Linux.
- yt-dlp and mpv are external runtime dependencies and need a packaging
  decision before release.

## Open decisions

- Final playback integration API between Rust and mpv.
- How yt-dlp and mpv are distributed or detected on each platform.
- SQLite schema and migration strategy.
- Minimum supported Windows and Linux versions.
- CI matrix and release artifact format.

When an open decision is resolved, add a short record under
[`decisions/`](decisions/), then update this file.
