# Fuzzy Music

Fuzzy Music is a desktop music player for Windows and Linux. The first
release is intended to search public SoundCloud tracks and stream available
audio without requiring users to obtain API keys manually.

The repository is currently in the project-foundation stage. The next
implementation step is a minimal Tauri 2 application with a React +
TypeScript interface and a Rust core.

## Project goals

- Search by track title and author.
- Show title, author, cover art, and duration.
- Play, pause, seek, and control volume.
- Manage a playback queue and switch tracks.
- Store local favourites and application settings.

Custom playlists, history, and additional themes are post-MVP work.

## Planned architecture

The application is a local desktop app with no application server in the
first release:

- Tauri 2 connects the web interface to the Rust core.
- React, TypeScript, and CSS provide the interface.
- Rust owns source integration, request cancellation and timeouts, playback
  control, and local persistence.
- yt-dlp is the current planned helper for SoundCloud search and stream
  discovery.
- mpv is the current planned audio engine.
- SQLite is the current planned local storage.

The boundary between the interface and Rust must use explicit, documented
track, search-result, and audio-stream contracts. See
[`docs/architecture/contracts.md`](docs/architecture/contracts.md).

## Development workflow

1. Read [`AGENTS.md`](AGENTS.md) and [`docs/project-context.md`](docs/project-context.md)
   before changing code.
2. Create one branch per task.
3. Keep changes focused and record meaningful decisions in
   [`docs/decisions/`](docs/decisions/).
4. Run the checks listed in the relevant task context.
5. Add a concise handoff entry to [`docs/ai-log.md`](docs/ai-log.md).
6. Open a Pull Request and request review from a second contributor.

The repository is designed to be usable by multiple coding agents. Agent
instructions are kept in [`AGENTS.md`](AGENTS.md), while the current project
state is kept in [`docs/project-context.md`](docs/project-context.md).

## Current status

The repository and Rust environment have been prepared. A Windows prototype
has already checked SoundCloud search, stream discovery, timeout handling, and
request cancellation; the reported prototype test suite contains eight
passing automated tests. Audio playback and the Tauri application shell are
the next implementation steps.

## License

This project is licensed under the GNU General Public License version 3. See
[`LICENSE`](LICENSE).
