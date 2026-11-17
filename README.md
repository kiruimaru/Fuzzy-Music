# Fuzzy Music

Fuzzy Music is a university desktop music player project for Windows and Linux.
The first release is intended to search public SoundCloud tracks and stream
available audio without requiring users to obtain API keys manually.

The repository includes a standalone Rust SoundCloud prototype. The Tauri 2
application with a React + TypeScript interface remains a next step.

## Project goals

- Search by track title and author.
- Show title, author, cover art, and duration.
- Play, pause, seek, and control volume.
- Manage a playback queue and switch tracks.
- Store local favourites and application settings.

Custom playlists, history, and additional themes are post-MVP work.

## Architecture and stack

The application is a local desktop app with no application server in the
first release:

| Technology | Responsibility | Status |
|---|---|---|
| Rust | Source integration and request control | Implemented prototype |
| yt-dlp | SoundCloud search, metadata, and stream discovery | Used by prototype |
| Cargo, rustfmt, Clippy | Build, tests, formatting, and linting | Available |
| Git and GitHub | Version control and Pull Request review | In use |
| Tauri 2 | Commands and events between interface and Rust | Planned |
| React, TypeScript, CSS | Search, library, and playback interface | Planned |
| mpv | Audio playback, position, volume, and queue control | Planned |
| SQLite | Local favourites and settings | Planned |

SoundCloud-specific code is isolated in `crates/soundcloud`. The planned
frontend/Rust boundary uses explicit, source-agnostic contracts in
[`docs/architecture/contracts.md`](docs/architecture/contracts.md); prototype
types are not that boundary. `crates/application` implements typed search/resolve
DTOs and safe errors without Tauri. See the contracts for worker/cancellation
integration and the compilable `crates/application/examples/search.rs` example.
Team responsibilities are recorded in
[`docs/team.md`](docs/team.md).

## Quick start on Windows x64

Install Git, Rust stable, and Microsoft C++ Build Tools with Windows SDK.
From the repository root:

```powershell
.\scripts\check-soundcloud.ps1 -Query 'lofi hip hop' -Limit 3
.\scripts\check-soundcloud.ps1 -Mode resolve -Query 'https://soundcloud.com/hephestosmusic/super-mario-world-game-over'
```

The script downloads pinned yt-dlp into `tools` and verifies its SHA-256.
Internet access is required; a SoundCloud account, cookies, and a manual API
key are not required for public tracks. Search returns JSON metadata; resolve
returns a temporary stream URL without playing or downloading the full track.
Availability and result order can change.

Requests have a 45-second timeout by default. To exercise timeout or cancellation:

```powershell
.\scripts\check-soundcloud.ps1 -Query 'lofi hip hop' -TimeoutSeconds 1
.\scripts\check-soundcloud.ps1 -Query 'lofi hip hop' -CancelAfterSeconds 0.25
```

An unfinished request exits with `timed out` or `cancelled` and code 1;
requests that finish earlier may succeed. Cancellation stops the extractor
process group. Initial compilation and tool download are outside the timeout.
See [`docs/soundcloud-development.md`](docs/soundcloud-development.md) for
full usage and Linux instructions.

## Development workflow

1. Read [`AGENTS.md`](AGENTS.md) and [`docs/project-context.md`](docs/project-context.md).
2. Create one branch per task.
3. Keep changes focused and record meaningful decisions in `docs/decisions/`.
4. Run the checks listed in the relevant task context.
5. Add a concise handoff entry to [`docs/ai-log.md`](docs/ai-log.md).
6. Open a Pull Request and request review from a second contributor.

```text
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
```

Tests run without SoundCloud access. Live connectivity, audio playback, the
Tauri shell, and Linux execution require separate verification.

## License

This project is licensed under the GNU General Public License version 3. See
[`LICENSE`](LICENSE).
