# Module Contracts

This document defines the first stable boundary between the React interface
and the Rust core. It is intentionally source-agnostic: SoundCloud response
objects must be normalized before they cross this boundary.

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
};
```

`id` is stable for the source. `durationMs` and `streamUrl` may be null when
the source does not provide them yet. The UI must not construct source URLs.

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
