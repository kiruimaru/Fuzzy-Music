# Fuzzy Music Project Context

**Last reviewed:** 2026-11-19
**Status:** playback controls, playlist transitions and Windows alpha integrated; review follow-ups remain

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

Saved/custom playlists, history, and additional themes are post-MVP.
Public source playlist discovery and playback are already implemented.

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

- Windows alpha packaging/discovery from PR #20 is integrated; see
  `../tasks/windows-alpha.md`, decision 0005-portable-alpha and `windows-alpha.md`. Live portable
  search/Playing/Stop passed with Windows-only PATH and different cwd. Public
  Portable ZIP supplies npm notices; a separate matching corresponding-source
  artifact supplies application/locked Rust sources with Git/executable provenance. External
  mpv/yt-dlp are acquired directly via digest-verified first-run bootstrap because
  exact transitive mpv source was not available as an upstream bundle. Clean
  Windows runtime verification remains external. Linux alpha is assigned in #19.
- PR #23 adds public SoundCloud playlist discovery, a playlist playback queue,
  previous/next, automatic transitions, link lookahead and HLS startup caching,
  along with mpv JSON IPC pause/resume, seek and volume. Saved playlists,
  independent queue editing, favourites and persisted settings remain unfinished.
  Issue #22 is not closed by integration.
- PRs #20 (Windows alpha) and #23 (playback controls, public playlists and
  transitions) were reviewed at heads `82bafc8` and `cfa6c69`. Their normal
  Rust/frontend checks pass, but additional reproductions expose cache limits,
  worker ownership, HTTP cancellation/deadlines, Linux release tool discovery
  and missing-runtime UI guidance defects. See
  `../tasks/pr-review-2026-11-19.md` and its regression patches.
  The owner authorized integration after manual testing, accepting the remaining
  findings as follow-up work. See `../tasks/pr20-pr23-integration.md`.
- Combined Linux verification: 44 Rust tests passed / 4 ignored, 15 frontend
  tests, four browser fixture tests, strict Clippy and rustfmt passed.
  The original PR #23 manual run had no observed crash/OOM or journal error;
  total service memory started near 269 MiB, was about 537 MiB at 12.5 minutes,
  and peaked at 677 MiB. This does not establish absence of a memory leak.
- AI context assembly has offline regression coverage in
  `scripts/test-ai-context.sh`; see `tasks/ai-context-11.md` for scope and limits.
- The repository and local Rust environment were prepared.
- `crates/application` adds synchronous typed search/resolve DTOs, safe errors,
  and explicit request control without a Tauri/runtime dependency. See decision
  0002 and `tasks/application-api.md`. PR #15 is merged; the shell uses its
  policies unchanged. Joint human owner review remains pending.
- `crates/soundcloud` contains the standalone Rust search and stream probe,
  including request timeout, cancellation, and process-group cleanup.
- The source/application boundary previously passed 14 tests on Windows/Linux.
  The shell workspace passed 20 offline tests on Linux, with child fixture and
  opt-in live audio test ignored during normal discovery.
- Strict Clippy and rustfmt checks passed for the integrated workspace.
- `src-tauri` and `src` implement minimal search, fresh resolve, owned mpv Play/
  Stop, cancellation, generation guards and exit cleanup. Source code is unchanged.
- Five frontend tests, two browser fixture smoke tests (1000px/320px), and a
  complete Tauri debug build passed. Live source/resolve/mpv play/Stop/replay
  passed using an existing local proxy; the real window returned ten tracks,
  reached Playing with an active audio-device stream and stopped its mpv child.
   Those Linux checks did not include human listening or Windows verification.
- The Windows network follow-up snapshots native manual proxy settings for both
  yt-dlp and mpv. Windows search returned ten results, fresh resolve reached Playing,
  mpv connected through the configured proxy, and Stop cleaned up the child.
  The user confirmed working search; listening was confirmed on an earlier build,
  not repeated on this build. See `../tasks/windows-network-integration.md` and
  decision 0004 for checks, startup restart requirements and PAC/bypass limitations.

The Linux evidence above consists of local checks. Native rendering needed
`WEBKIT_DISABLE_DMABUF_RENDERER=1`; direct SoundCloud access timed out.
See [`shell-development.md`](shell-development.md) and
[`../tasks/tauri-shell-14.md`](../tasks/tauri-shell-14.md) for setup and limits.
Windows alpha PR/manual build and artifact workflow is configured.

## Agreed work split

The source-integration owner is responsible for SoundCloud search, normalized
metadata, stream discovery, source errors, and later local favourites.

The application-shell owner is responsible for the Tauri 2 shell, React +
TypeScript interface, mpv integration, and build preparation.

Before connecting the modules, both owners must agree on the contracts in
[`architecture/contracts.md`](architecture/contracts.md).
The prototype's Rust types are internal source-adapter contracts, not Tauri
DTOs. Mapping and safe errors now live in `crates/application`; review the new
contract policies together during contributor review. Worker dispatch, local
stale-response guards and cancellation IPC now live in `src-tauri`.

## Delivery order

PR #12 (source), PR #15 (application API, retargeted to main), and PR #13
(context checks) are merged into main. The journal conflict in #13 was resolved
by preserving both histories. Old source branches need no separate code merge:
their implementation is already integrated, including the later guide edits.
Issue #14 shell PR #17 and Windows network PR #18 are merged into main at
`faa334a`. Their task contexts retain platform/manual acceptance limitations.
PRs #20/#23 are integrated with merge conflict resolutions preserving portable
dependency errors and mpv IPC controls. Next work is resolving the accepted
review findings, contributor contract review, and release/tool packaging decisions.

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

- Independent queue editing and remaining full playback API work (issue #22).
- Linux release dependency discovery/setup; #20 currently requires beside-app
  tools for release builds on Linux too, with explicit FUZZY overrides as a workaround.
- SQLite schema and migration strategy.
- Minimum supported Windows and Linux versions.
- CI matrix and release artifact format.

When an open decision is resolved, add a short record under
[`decisions/`](decisions/), then update this file.
