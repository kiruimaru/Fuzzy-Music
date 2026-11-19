# 0011: Playlist bookmarks and optional playlist downloads

Date: 2026-11-19. Status: stacked on PR #25; contributor review pending.

Favourite playlists live in the same SQLite database as tracks but in their own
`playlists` table; schema v3 adds it together with a `playlist_tracks` junction.
A bookmark stores playlist metadata only: no signed URLs and no audio. Liking a
playlist never starts downloads. The row stays after unliking so downloaded
tracks keep their association and a later like restores the previously saved
state.

Downloading a playlist is an explicit second action. Rust resolves the track
list through the existing application boundary, inserts every track as a normal
media row without marking it favourite, and enqueues the same single download
worker used for favourite tracks. The download queue is bounded at 1024 ids and
one playlist request is capped at 1000 tracks. Tracks shared between playlists
and favourites are deduplicated by source id. Existing files are never
overwritten; download state, retry and offline playback behave like favourite
downloads.

The UI exposes hearts on playlist cards and in the playlist header, a Favourites
section for bookmarked playlists with per-playlist progress, and a Download
action that can be repeated to retry failed tracks. While a playlist download is
running the same action becomes Cancel: pending and in-flight tracks stop, their
state returns to idle and the bookmark with its full track list is preserved, so
Download resumes the work later. Downloading a playlist never marks its tracks
as favourite; they appear in Favourites only if the listener hearts them
explicitly. Removing a bookmark keeps downloaded files, matching track favourite
behaviour. Unliking a track in the Favourites list removes its row immediately
while the store write runs in the background, and a failed write restores the
entry on the next refresh. Playlist tracks also stay reachable from the Music
library once downloaded.

Real SoundCloud playlist downloads, restart acceptance and Windows GUI checks
remain separate verification work.
