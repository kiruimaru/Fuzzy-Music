# Fuzzy Music

Fuzzy Music is a university desktop music player project for Windows and Linux.
The first release is intended to search public SoundCloud tracks and stream
available audio without requiring users to obtain API keys manually.

The repository includes a Rust SoundCloud adapter, a typed application API,
and a Tauri 2 + React/TypeScript shell for search and mpv playback.
See [desktop setup and verification](docs/shell-development.md).

The player supports pause/resume, position/seek and volume through mpv JSON IPC. See
[`tasks/playback-controls.md`](tasks/playback-controls.md) for verification and
remaining limitations. Playlist playback supports previous/next and automatic
transitions. A separate session queue supports adding search results, selecting
and removing entries, Previous/Next and automatic advancement without repeat.
Removing the current entry stops playback and leaves the next choice to the user.
Queue persistence is outside the MVP; durable volume storage awaits #21.

A dark Spotify-style shell also adds public SoundCloud **playlist search**:
search playlists, open one and play its tracks through the same player. It uses
the SoundCloud web API with an auto-discovered `client_id`, because `yt-dlp`
cannot search playlists. See decision 0006 and
[`tasks/playlist-search.md`](tasks/playlist-search.md). Saved playlists are not
implemented. Integration checks and accepted review follow-ups are recorded in
[`tasks/pr20-pr23-integration.md`](tasks/pr20-pr23-integration.md).
Session queue behavior and #22 acceptance are recorded in
[`tasks/playback-queue-22.md`](tasks/playback-queue-22.md).

## Project goals

- Search by track title and author.
- Show title, author, cover art, and duration.
- Play, pause, seek, and control volume.
- Manage a playback queue and switch tracks.
- Store local favourites and application settings.

Saved/custom playlists, history, and additional themes are post-MVP work.

## Architecture and stack

The application is a local desktop app with no application server in the
first release:

| Technology | Responsibility | Status |
|---|---|---|
| Rust | Source integration, request control, typed application API | Implemented |
| yt-dlp | SoundCloud search, metadata, and stream discovery | Used by prototype |
| Cargo, rustfmt, Clippy | Build, tests, formatting, and linting | Available |
| Git and GitHub | Version control and Pull Request review | In use |
| Tauri 2 | Typed commands between interface and Rust | Search, playlists and playback controls implemented |
| React, TypeScript, CSS | Search and minimal playback interface | Implemented |
| mpv | Audio playback | Play/Stop, pause, seek, volume and playlist transitions implemented |
| SQLite | Local favourites and settings | Planned |

SoundCloud-specific code is isolated in `crates/soundcloud`. The planned
frontend/Rust boundary uses explicit, source-agnostic contracts in
[`docs/architecture/contracts.md`](docs/architecture/contracts.md); prototype
types are not that boundary. `crates/application` implements typed search/resolve
DTOs and safe errors without Tauri. `src-tauri` dispatches source calls to blocking
workers and owns cancellation and mpv. See the contracts and
`crates/application/examples/search.rs` for the runtime-independent API.
Team responsibilities are recorded in
[`docs/team.md`](docs/team.md).

## Repository layout

- `crates/soundcloud`: source adapter, extractor process control, and probe CLI.
- `crates/application`: serializable search/resolve DTOs, safe errors, offline
  boundary tests, and a blocking-worker example.
- `src-tauri`: Tauri commands, request control, and owned mpv playback.
- `src`: minimal React + TypeScript interface and frontend tests.
- `tests`: browser smoke tests with fixture IPC.
- `scripts`: shared agent context checks and Windows extractor setup/probes.
- `docs/architecture` and `docs/decisions`: contracts and durable decisions.
- `docs/project-context.md`, `docs/ai-log.md`, and `tasks`: current state and
  contributor handoffs.
- `.github`: issue/PR templates and Windows alpha build/artifact workflow.

Keep Tauri commands above `crates/application`; the frontend must use
the documented DTOs. Persistence and the full playback API remain planned.

## Quick start on Windows x64

For the portable alpha artifact and first-run runtime setup, see
[Windows alpha](docs/windows-alpha.md). The player includes playlist playback
and controls; the full MVP and clean-machine verification remain unfinished.
Choose the **portable** artifact for the small user download; matching GPL
application and vendored dependency sources are a separate **corresponding-source**
artifact on the same build, with checksums and Git/executable provenance.

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
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
bash scripts/test-ai-context.sh
bash -n scripts/ai-context.sh scripts/test-ai-context.sh
```

Normal tests run without SoundCloud access. The source/API checks previously
passed on Windows and Linux; the shell, frontend and live playback checks passed
locally on Linux with the environment limits documented in
[shell development](docs/shell-development.md). These are not CI results;
Windows shell validation also passed for the network-policy follow-up: search,
fresh resolve, Playing, proxy connection and Stop were observed. The user confirmed
working search; audible playback was confirmed on an earlier build, not reverified
by listening on this build. See [Windows network policy](docs/decisions/0004-windows-network-policy.md)
for startup refresh and PAC/bypass limitations. These changes have not been
retested on Linux; non-Windows routing uses inherited proxy settings or normal routing.

For the desktop shell:

```text
npm ci
npm run tauri -- dev
npm test
npm run test:ui
npm run build
```

## License

This project is licensed under the GNU General Public License version 3. See
[`LICENSE`](LICENSE).
