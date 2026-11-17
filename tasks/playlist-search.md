# Task: Public playlist search

## Scope

Search public SoundCloud playlists, open one, and play any of its tracks
through the existing single-track player. No saved/local playlists, no
account features, no queue.

## Implementation

- `crates/soundcloud/src/web.rs`: `client_id` discovery with in-memory cache and
  single 4xx refresh, playlist search, playlist track loading, URL-safe query
  encoding, cancellation checks and safe errors.
- `crates/soundcloud`: `Playlist`, `PlaylistSearchResults`, `PlaylistTracks`
  types plus `search_playlists` and `playlist_tracks` methods; probe CLI gains
  `playlists` and `playlist` actions.
- `crates/application`: playlist DTOs with camelCase wire shape, mapping,
  `search_playlists`, `playlist_tracks`, skipped-track count.
- `src-tauri`: `search_playlists`, `playlist_tracks`, `cancel_playlists` with a
  separate generation operation so playlist and track searches never cancel
  each other.
- `src`: Tracks/Playlists tabs, playlist grid, playlist header with back
  navigation, unchanged footer player.
- Dark Spotify-style shell: sidebar, rounded search, results table, pinned
  player; responsive at 320px with the player stacked last.

## Checks

- Live probe: `playlists "lofi chill"` returned playlists with ids, counts and
  `permalink_url`; `playlist <set url>` returned 51 tracks with titles, authors
  and permalinks.
- Large playlist check: a 416-track set returned all 416 entries after stubs
  were refetched via `/tracks?ids=`.
- Track search probe: limits 10, 25 and 50 returned exactly that many tracks.
- Rust workspace tests: 25 passed, 2 ignored; strict Clippy and rustfmt pass.
- Frontend: 8 tests pass, including playlist search/open and safe errors.
- Browser: 3 tests pass (tracks 1000/320 px, playlists 1000 px).
- TypeScript/Vite build and debug Tauri build pass.

## Remaining

- Linux transport not exercised for this feature.
- `client_id` discovery depends on SoundCloud's bundle shape and may need
  updating if their site changes.
- Human listening and real playlist playback on Windows await the user check.
