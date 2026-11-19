# 0012: Artist pages and list playback

Date: 2026-11-19. Status: stacked on the playlist bookmarks branch; review pending.

Artist pages are resolved through the existing SoundCloud web transport: the app
resolves the artist URL to a user id and loads the first page of the artist's
public tracks from api-v2. The UI turns the artist name in every track row into a
link; the artist view reuses the playlist header styling, keeps the previous
search results intact and exposes a Play artist action. Only the first 50 tracks
are loaded in this iteration; artists without public tracks show an empty state.

Track rows now behave as lists everywhere. Playing any row starts the visible
list at that index through the existing playlist transition machinery, so mpv
appends the successor and the interface advances automatically. This applies to
search results, artist pages, playlists, Favourites and the music library; the
separate session queue keeps its existing semantics. The change removes the old
single-track search behavior, which played a result in isolation.

Artist requests are validated before any network access and are generation
scoped like playlist loads, so switching searches or views cancels obsolete
work. Errors reuse the safe application messages and never expose source
diagnostics. Pagination, following artists and account features remain future
work.
