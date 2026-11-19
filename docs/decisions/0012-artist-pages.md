# 0012: Artist pages and list playback

Date: 2026-11-19. Status: stacked on the playlist bookmarks branch; review pending.

Artist pages are resolved through the existing SoundCloud web transport: the app
resolves the artist URL to a user id and loads the first page of the artist's
public tracks and public sets from api-v2. The artist view separates the
catalog into Tracks, Albums and Playlists tabs with counts: sets flagged
`is_album` by SoundCloud are classified as albums, the rest as playlists. The UI turns every visible artist name into a link to that page: track rows in
search, artist lists, Favourites, the music library and the queue, plus playlist
cards, playlist headers and the now-playing panel. The artist view has its own
presentation rather than reusing playlist chrome: a wide banner filled with a
blurred copy of the artist avatar, a circular portrait, the name, a summary of
tracks, albums and playlists, and a Play artist action. Album and playlist cards
open through the existing playlist view and keep their favourite toggles; leaving
such a playlist returns to the artist page. The previous search results stay
intact for Back to search. Only the first 50 tracks and 50 sets are loaded in
this iteration; artists without public tracks show an empty state.

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
