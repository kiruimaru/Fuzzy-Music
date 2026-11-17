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
PR #15 was reviewed and merged with the policies below. Both module owners
must still confirm their shell integration responsibilities before wiring IPC;
this merge does not establish an earlier joint agreement. The source adapter's
public API remains unchanged.

## Track

```ts
type Track = {
  id: string;
  title: string;
  author: string;
  artworkUrl: string | null;
  durationMs: number | null;
  streamUrl: string | null;
  source: "soundcloud";
  sourceUrl: string;
};
```

`id` is stable for the source. `durationMs` and `streamUrl` may be null when
the source does not provide them yet. The UI must not construct source URLs.

`sourceUrl` is the public source permalink supplied by the adapter. This is an
explicit extension to the initial contract: source ID alone cannot be resolved
by the existing adapter. Pass it unchanged to resolve; it is not an audio URL.
Missing or whitespace-only author becomes `Unknown artist`; present author is
preserved. Duration is rounded to nearest millisecond; absent, negative,
nonfinite, or greater than JavaScript's safe integer range becomes null.
Search and resolved metadata always have `streamUrl: null`; audio URLs live
only in the separate resolve response. Do not persist temporary audio URLs.

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

Limit defaults to 10 and must be 1..20. `nextPageToken` is always null:
pagination is unsupported, not an indication of all available source results.

## Resolve

```ts
type ResolveTrackRequest = { source: "soundcloud"; trackUrl: string };
type ResolveTrackResponse = { track: Track; streamUrl: string };
```

Resolve immediately before playback, using `Track.sourceUrl` as `trackUrl`.
The response URL is temporary, with no guaranteed expiry time; re-resolve if
playback can no longer use it. No playback engine is implemented here.

## Shell integration

`Application::search_tracks` and `resolve_track` are synchronous blocking calls.
A future async Tauri command MUST dispatch them to its blocking worker pool and
await completion; merely declaring a command async does not make these calls
nonblocking. No Tauri bridge or runtime is provided by this crate.
See the compilable CLI example `crates/application/examples/search.rs`:
`cargo check -p fuzzy-application --example search --locked`.

Supply a fresh `RequestOptions`/cancellation token per operation; keep a clone
with the owning view/controller, cancel on replacement or disposal. Timeout
(45 seconds by default) and process-group cancellation pass through unchanged.
Cancellation can race with successful completion: each search/resolve owner
must increment a local generation on every new intent (including clearing the
query), capture it when starting work, and apply success/error/loading updates
only if it still matches. Search and playback selection need separate local
generations. No global registry is needed. A concrete cancel IPC command and
worker-panic translation remain shell responsibilities.

For issue #14, Stop must invalidate the playback generation and cancel pending
resolve before stopping mpv; a late resolve must never restart playback. Window
close must cancel requests and clean up shell-owned playback processes. See
`tasks/application-api.md` for dependency order and the integration checklist.

## Playback

```ts
type PlaybackState =
  | "idle"
  | "loading"
  | "playing"
  | "paused"
  | "stopped"
  | "error";

type PlaybackSnapshot = {
  state: PlaybackState;
  trackId: string | null;
  positionMs: number;
  durationMs: number | null;
  volume: number;
  error: string | null;
};
```

Playback commands should be small and explicit: load a track, play, pause,
seek, set volume, and stop. The UI consumes snapshots/events and does not
manage the mpv process directly.

## Error shape

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
