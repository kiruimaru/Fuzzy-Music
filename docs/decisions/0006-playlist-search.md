# 0006: Public playlist search through the SoundCloud web API

Date: 2026-11-17. Status: local implementation; contributor review pending.

`yt-dlp` can search tracks and expand a playlist URL, but it cannot search
playlists: SoundCloud search pages return 404 to it. Playlist discovery
therefore uses `api-v2.soundcloud.com/search/playlists` and `/playlists/{id}`,
which require a `client_id`. That id is embedded in SoundCloud's own public
JavaScript bundles, so it is discovered by fetching `https://soundcloud.com`,
collecting `a-v2.sndcdn.com` script URLs, and reading the first 32-character
`client_id` literal. No user account, token, cookie or manually obtained key is
involved, and nothing is persisted to disk.

HTTP uses `ureq` with rustls and a global timeout from `RequestOptions`. This
adds a second transport to `crates/soundcloud` alongside the `yt-dlp` process
path. Track search, resolve and playback are unchanged and still use `yt-dlp`;
the HTTP client cannot resolve audio streams.

The `client_id` is cached in process memory only. A 4xx response from the
cached-id request clears the cache, rediscovers the id once, and retries. Only
one retry is attempted: repeated rejection surfaces a safe error instead of
looping. Cancellation is checked before each request and after reading a body.
Raw HTTP and network text stays inside the adapter; the application maps
`SoundCloudError::Network` to `source_unavailable` with a fixed retryable
message.

Playlist search is limited to 1..50 results, defaults to 12 in the UI, and
returns no pagination token. Opening a playlist resolves its URL through
`/resolve`, then loads `/playlists/{id}`; that single response already contains
all tracks, so no per-track requests are made. Entries that are missing an id,
title or public permalink are dropped, and the response reports how many were
skipped relative to the advertised `track_count`.

Large playlists do not always include full metadata: some entries arrive as
bare stubs containing only `id`, `kind` and `policy`. Those are never dropped
silently; their ids are batched (up to 50 per request) into `/tracks?ids=...`
and merged back at the original playlist position, so order is preserved. A
failed metadata batch leaves the already parsed entries intact and only those
tracks are counted as skipped.

Track search limits follow `yt-dlp`: `scsearch` accepts larger pages, so the
adapter allows 1..100 instead of the earlier 1..20 probe limit. The UI requests
30 results for both tracks and playlists.

Only public third-party playlists are supported. Saved/local playlists,
likes and account data stay out of scope. The playlist id is used as a stable
identity but the public `permalink_url` is what crosses the boundary, matching
the existing rule that the UI never constructs source URLs.
