# Task: Artist pages and list playback

## Goal

Open an artist's page from any track row, listen to their public tracks in order
without manual switching, and make every visible track list auto-advance.

## Scope

### In scope

- Resolve an artist URL through the SoundCloud web API and list their public tracks.
- Link artist names in track rows; add an artist view with Play artist and Back to search.
- Play any clicked row together with its visible list so the next track starts automatically.
- Apply the same list behavior to search results, playlists, Favourites and the music library.

### Out of scope

- Artist pagination beyond the first 50 tracks.
- Following artists, artist likes/reposts or account features.
- Provider work beyond public SoundCloud.

## Context

- Relevant files: `crates/soundcloud/src/lib.rs`, `crates/soundcloud/src/web.rs`,
  `crates/application/src/lib.rs`, `src-tauri/src/lib.rs`, `src/App.tsx`, `src/api.ts`.
- Related decision: `docs/decisions/0012-artist-pages.md`; playback list behavior
  builds on `docs/decisions/0007-playlist-transitions.md`.
- Dependencies: stacked on `feat/playlist-favourites` (playlist bookmarks, PR #25 base).

## Plan

1. Add artist resolution and track loading to the SoundCloud client and web transport.
2. Expose the application boundary DTO, Tauri commands and frontend bindings.
3. Render the artist view and make row playback start the visible list.
4. Cover validation, parsing, UI navigation and auto-advance with tests; update docs.

## Progress

### Completed

- `Artist`/`ArtistTracks` in the SoundCloud crate, `/resolve` + `/users/{id}/tracks`
  loading with array and collection response shapes.
- Artist URL validation rejects track, playlist, query and non-HTTPS URLs before
  any network access.
- Application DTOs `Artist`, `ArtistTracksRequest`, `ArtistTracksResponse`;
  commands `artist_tracks` and `cancel_artists` with generation-scoped cancellation.
- Artist names are links in track rows; the artist view reuses playlist header
  styling and keeps search results for Back to search.
- Row playback uses one list queue, so search results, artist pages, playlists,
  Favourites and the music library auto-advance through the existing transition
  machinery.
- Tests: artist URL validation, artist metadata/track parsing, frontend artist
  navigation and auto-advance, existing suite unchanged.

### Remaining

- Manual GUI acceptance against real SoundCloud through the user's proxy.
- Pagination for artists with more than 50 tracks (future work).

## Verification

Commands and results:

```text
cargo fmt --all
clean
cargo test --workspace --all-targets --locked
58 passed, 4 ignored
npm test
34 passed
cargo clippy --workspace --all-targets --locked -- -D warnings
clean
npm run tauri -- build --debug --no-bundle
build passed
```

## Handoff

- Current branch: `feat/artist-pages` (based on `feat/playlist-favourites`).
- Files changed: SoundCloud client and web transport; application DTOs; shell
  commands; API bindings; App view, rows and styles; tests; docs.
- Known risks: artist track lists are capped at 50; the artist view reuses the
  playlist hero styles; list playback now depends on the playlist transition path
  for every list, including local library tracks.
- Next action: manual Windows GUI check, then review and publishing decision.
