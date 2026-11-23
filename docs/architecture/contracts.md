# Module Contracts

This document defines the first stable boundary between the React interface
and the Rust core. It is intentionally source-agnostic: SoundCloud response
objects must be normalized before they cross this boundary.

## Application boundary status

`crates/soundcloud` is a standalone synchronous adapter/probe, not a Tauri
command implementation. Its public Rust API is preserved from PR #1:

- `Track` uses `uploader`, `duration_seconds`, and `webpage_url`, with optional
  uploader metadata; the planned DTO uses `author`, `durationMs`, `streamUrl`,
  and `source` with camelCase serialization.
- `SearchResults` contains `query` and `tracks`, without pagination metadata.
- `ResolvedStream` keeps the temporary stream URL and format separate from
  track metadata.
- `SoundCloudError` is not `AppError`; extractor diagnostics may contain raw
  stderr and must not be forwarded directly to the UI.

`crates/application` now implements the DTO boundary independently of Tauri.
PR #15 was reviewed and merged with the policies below. The shell uses them
unchanged in issue #14; decision 0003 records its responsibilities. Joint human
owner review remains pending; implementation does not establish an earlier
agreement. The source adapter's public API remains unchanged.

## Track

```ts
type Track = {
  id: string;
  title: string;
  author: string;
  artworkUrl: string | null;
  durationMs: number | null;
  streamUrl: string | null;
  source: "soundcloud" | "local";
  sourceUrl: string;
};
```

`id` is stable for the source. `durationMs` and `streamUrl` may be null when
the source does not provide them yet. The UI must not construct source URLs.

For SoundCloud, `sourceUrl` is the public source permalink supplied by the adapter. This is an
explicit extension to the initial contract: source ID alone cannot be resolved
by the existing adapter. Pass it unchanged to resolve; it is not an audio URL.
Missing or whitespace-only author becomes `Unknown artist`; present author is
preserved. Duration is rounded to nearest millisecond; absent, negative,
nonfinite, or greater than JavaScript's safe integer range becomes null.
Search and resolved metadata always have `streamUrl: null`; audio URLs live
only in the separate resolve response. Do not persist temporary audio URLs.
Local tracks have opaque `id` and `sourceUrl: "local:<id>"`; their filesystem
paths stay in Rust. They are played only through `play_library_track`.

## Search

```ts
type SearchTracksRequest = {
  query: string;
  limit?: number;
};

type SearchTracksResponse = {
  tracks: Track[];
  nextPageToken: string | null;
};
```

The command must expose loading, empty, error, and cancellation states. A
stale response must not overwrite a newer search.

Track limits must be 1..100. The extractor application method defaults to 10;
the production web-search method and UI default to 30. `nextPageToken` is always null:
pagination is unsupported, not an indication of all available source results.

## Resolve

```ts
type ResolveTrackRequest = { source: "soundcloud"; trackUrl: string };
type ResolveTrackResponse = { track: Track; streamUrl: string };
```

Resolve immediately before playback, using `Track.sourceUrl` as `trackUrl`.
The response URL is temporary, with no guaranteed expiry time; re-resolve if
playback can no longer use it. No playback engine is implemented here.

## Playlists

Public playlist discovery uses the SoundCloud web API, not `yt-dlp`, because
the extractor cannot search playlists. Decision 0006 records transport,
`client_id` handling and limits. Tracks still resolve and play through the
unchanged single-track path.

```ts
type Playlist = {
  id: string;
  title: string;
  author: string;
  artworkUrl: string | null;
  trackCount: number | null;
  durationMs: number | null;
  source: "soundcloud";
  sourceUrl: string;
};

type SearchPlaylistsRequest = { query: string; limit?: number };
type SearchPlaylistsResponse = {
  playlists: Playlist[];
  nextPageToken: string | null;
};

type PlaylistTracksRequest = { source: "soundcloud"; playlistUrl: string };
type PlaylistTracksResponse = {
  playlist: Playlist;
  tracks: Track[];
  skippedTrackCount: number;
};
```

Limit defaults to 10 and must be 1..50. `nextPageToken` is always null.
`sourceUrl` is the public set permalink; the UI never builds playlist or track
URLs. `trackCount` is the source-advertised count, while `tracks` may be
shorter when entries lack an id, title or public permalink; the difference is
reported as `skippedTrackCount`. Playlist search and playlist loading use a
generation stream separate from track search, so either search can be replaced
or cancelled without touching the other. Playback selection keeps its own
generation.

## Shell integration

The shell snapshots a shared immutable network policy at startup and explicitly
applies it to extractor and player child Commands. Existing source constructors
remain compatible. See decision 0004 for Windows system proxy/env precedence,
restart behavior and PAC/bypass limitations. No frontend contract changes.

`Application::search_tracks` and `resolve_track` are synchronous blocking calls.
An async Tauri command MUST dispatch them to its blocking worker pool and
await completion; merely declaring a command async does not make these calls
nonblocking. No Tauri bridge or runtime is provided by this crate; `src-tauri`
now supplies the bridge, workers, cancellation IPC and safe worker errors.
See the compilable CLI example `crates/application/examples/search.rs`:
`cargo check -p fuzzy-application --example search --locked`.

Supply a fresh `RequestOptions`/cancellation token per operation; keep a clone
with the owning view/controller, cancel on replacement or disposal. Timeout
(45 seconds by default) and process-group cancellation pass through unchanged.
Cancellation can race with successful completion: each search/resolve owner
must increment a local generation on every new intent (including clearing the
query), capture it when starting work, and apply success/error/loading updates
only if it still matches. Search and playback selection need separate local
generations. No global registry is needed. Cancel IPC and worker-panic translation
are implemented by the shell.

For issue #14, Stop must invalidate the playback generation and cancel pending
resolve before stopping mpv; a late resolve must never restart playback. Window
close must cancel requests and clean up shell-owned playback processes. See
`tasks/tauri-shell-14.md` and `docs/shell-development.md` for verification.

The minimal shell commands are:

```ts
search_tracks({ generation, request: SearchTracksRequest }): Promise<SearchTracksResponse>
cancel_search({ generation }): Promise<void>
play_track({ generation, request: ResolveTrackRequest, fresh?: boolean, volume?: number }): Promise<void>
stop_playback({ generation }): Promise<void>
playback_status(): Promise<{
  generation: number;
  state: "idle" | "loading" | "playing" | "stopped" | "error";
  trackId: string | null;
  error: string | null;
}>
```

Each intent includes a strictly increasing safe-integer generation, independently
for search and playback. Old/equal command generations are rejected even when
worker scheduling reorders them. Error results use AppError; status.error is
also a fixed safe message. Search never changes playback. play_track completes
after fresh resolution and process creation; loading continues until mpv reports
playback initialization, or its startup deadline/exit reports error. UI labels
distinguish source resolving from player startup. This small status is specific
to #14, not the full future PlaybackSnapshot below.

## Playback

Local controls follow decision 0005 (supersedes the Play/Stop-only limitation
of decision 0003). `playback_status` additionally returns `positionMs: number`,
`durationMs: number | null`, `volume: number` (0..100), `seekable: boolean`, and
supports `state: "paused"`. `generation` remains part of the status.

```ts
update_playback({ generation, action:
  | { type: "pause"; paused: boolean }
  | { type: "seek"; position_ms: number }
  | { type: "volume"; volume: number }
}): Promise<void>
```

Controls use the current playback generation without creating a new intent.
Stale generations are rejected. Pause/resume preserve the player and stream.
Seek requires known duration and seekability; position is clamped to duration.
The action's `position_ms` field is explicitly snake_case on this IPC command;
status fields remain camelCase. Successful command submission is not an
optimistic state change: observed mpv properties drive the next snapshot.
Volume must be finite and within 0..100. `play_track` accepts the preferred
volume and passes it to mpv at process startup; omitted volume uses the last
snapshot volume (100 initially). Stop preserves volume. Volume is saved through
the SQLite settings port.

```ts
type PlaybackState =
  | "idle"
  | "loading"
  | "playing"
  | "paused"
  | "stopped"
  | "error";

type PlaybackSnapshot = {
  generation: number;
  state: PlaybackState;
  trackId: string | null;
  positionMs: number;
  durationMs: number | null;
  volume: number;
  seekable: boolean;
  error: string | null;
  transitions: number;
};
```

Playback commands should be small and explicit: load a track, play, pause,
seek, set volume, and stop. The UI consumes snapshots/events and does not
manage the mpv process directly.

## Session queue

The frontend owns a temporary ordered sequence of `{ key, track: Track }`
entries; duplicate Tracks have different entry keys. No queue or source-specific
response shape crosses IPC. Add appends without playing; Play on a track-search
result appends and selects. Search and browsing do not clear the queue.
Selection and Previous/Next have no wraparound. Stop preserves list/selection.
Deleting the current entry stops/cancels playback and clears selection; another
entry must be selected explicitly. Removing another entry preserves playback.

Session queue play commands set `fresh: true`: each new track resolves at launch,
ignoring prepared/cached stream links and HLS audio. Rust stops the old player
before resolving the replacement, cancels old preparation/lookahead, and rejects
late work through playback generations. No audio warmers are spawned in this
mode. EOF selects the next entry once, or remains stopped after the last.
An error disables automatic advancement and does not retry/skip without user
action. Existing public playlist playback keeps its own prepared transition
sequence; adding to the session queue does not edit that source sequence.
See decision 0009.

## Settings Port For #21

The following TypeScript contract is implemented by an injectable frontend port:

```ts
type AppSettings = {
  volume: number;
  downloadQuality: "best" | "high" | "medium" | "standard" | "small" | "flac";
  musicFolder: string;
  theme: "dark" | "light";
};
interface SettingsStore {
  getSettings(): Promise<AppSettings>;
  updateSettings(settings: Partial<AppSettings>): Promise<AppSettings>;
}
```

Default volume is 100; values must be finite within 0..100. Updates return the
committed settings; updates patch only supplied fields. Tauri mapping:
`get_settings(): Promise<AppSettings>` and
`update_settings({ settings: AppSettings }): Promise<AppSettings>`.
These commands use one versioned SQLite database in the Tauri application data
directory. UI waits for its initial read before Play, serializes volume writes,
shows safe load/save failures, and falls back to 100 on read failure without
writing over stored data. Contributor review/coordination with #21 remains.
No temporary stream URL belongs in settings.

## Favourites And Local Library

`library_entries()` returns typed entries containing stable Track metadata,
favourite state, download status and offline availability; filesystem paths
are not sent to the UI. `set_favourite({track, liked})` persists a like and
enqueues a SoundCloud download. `retry_download({id})` retries a failed or
missing file. `scan_library()` imports supported local audio from the configured
folder. `play_library_track({generation,id,volume})` opens only an indexed file
and uses the existing playback generation/owned mpv lifecycle.

`choose_music_folder()` opens the native folder chooser; `update_settings`
requires an existing absolute writable directory. A startup scan imports
supported files using embedded title/artist/duration metadata where available;
a manual refresh detects files added later. Scan does not follow symlinks, is
bounded to 20,000 entries and 16 directory levels, and reports unreadable items.
Changing folders does not move or delete existing indexed files.

One background worker queues at most 64 downloads and applies the source's
best available audio by default. High/medium/standard/small prefer source
formats at or below 320/192/128/64 kbit/s respectively, falling back to the
best available audio if none matches. FLAC converts the best available audio
with ffmpeg; it cannot recover quality lost at the source and may require
substantially more temporary disk space. Output size is checked after conversion.
Changing the preference does not replace existing downloads. Downloads have a 10-minute
deadline and 512 MiB file limit, use disk staging and atomic no-overwrite
publication. Removing a like cancels pending work but never deletes user files.
Like and failure persist independently; retry is explicit. Temporary signed
links are neither persisted nor exposed to the interface. See decision 0010.

## Linux Runtime Setup

`runtime_status(): Promise<RuntimeStatus>` checks executable availability off
the UI thread. `install_runtime({installSystem: boolean, ffmpeg: boolean})`
returns the same status. `RuntimeStatus` contains `supported`, `ytDlp`, `mpv`,
`ffmpeg`, and `canInstallPackages`, all booleans; it never exposes local paths.
Only Linux x86_64 supports in-app installation. Missing tools are shown on
startup and status remains available in Settings.

Installation requires an explicit frontend action. yt-dlp and its notices
are downloaded from fixed upstream HTTPS URLs, with bounded bytes/deadlines,
pinned SHA-256 verification and atomic publication into the user data directory.
System packages require a separate explicit consent checkbox; only fixed
mpv/ffmpeg package arguments are allowed, through host pkexec and its desktop
authentication agent. No shell or user-supplied command crosses IPC. Package
installation has a root-owned ten-minute timeout. Window close cancels setup
and attempts process-group cleanup without blocking on an elevated child;
an already authorized transaction may finish in the background.
AppImage child media tools clear bundled library/Python overrides.
See decision 0020. Windows setup is unchanged.

## Error Shape

Errors crossing the Tauri boundary should be safe to display and useful for
diagnostics:

```ts
type AppError = {
  code:
    | "invalid_input"
    | "not_found"
    | "source_unavailable"
    | "timeout"
    | "cancelled"
    | "playback_failed"
    | "storage_failed"
    | "internal";
  message: string;
  retryable: boolean;
};
```

Do not expose cookies, tokens, command-line arguments containing secrets, or
full external-service responses in user-facing errors.

The application mapper discards all source diagnostic strings, including for
Debug/Display/serialization. Invalid input and cancellation are not retryable;
timeout, extractor failure, and invalid response are retryable. Process/tool
failures are `source_unavailable`, not retryable until local setup is repaired.
No classification is inferred from stderr; `not_found` is reserved, because
the adapter cannot reliably distinguish it. Existing source CLI diagnostics
are internal and may still print stderr; do not forward them to UI/telemetry.

The shell can append one prepared successor to mpv. Playback status includes a
monotonic `transitions` count for the current playback generation; the UI uses
it to advance its queue without issuing another play command. See decision 0007.

Lookahead resolves up to forty neighbouring links with eight workers and at most
sixteen extractor launches per cycle. The three nearest requests also warm HLS
startup audio. Manual Play can warm the selected track separately. Temporary
links remain internal and are not persisted; misses use fresh resolution.

Supported HLS manifests are rewritten for a loopback HTTP server. Up to twelve
segments and 3 MiB of segment payload are retained per track, with an initial
manifest published after three segments. Unsupported forms fall back to the
original stream. Both link and audio caches retain up to 41 tracks; old warm
neighbourhoods are not yet trimmed to current-plus-three. Retained segment payload
can therefore reach 123 MiB. Manifests, concurrent downloads, runtime allocations
and mpv use additional memory; manual warm workers lack a global concurrency cap.

Manual selection preserves playlist lookahead. Numeric expiry uses a two-minute
safety margin and a maximum age of 55 minutes; unknown expiry uses short
fallback lifetimes. Stop clears entries and invalidates publication, but in-flight
manual HTTP work is not synchronously cancelled or joined. These lifecycle and
budget defects remain accepted follow-ups in `tasks/pr20-pr23-integration.md`.
Immediate or gapless playback and bounded total process memory are not guaranteed.
