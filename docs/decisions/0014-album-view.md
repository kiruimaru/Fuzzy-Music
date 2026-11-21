# 0014: Album view

Date: 2026-11-19. Status: stacked on the player motion branch; review pending.

SoundCloud playlists and albums share one API shape, so the app treated an
opened album exactly like a playlist. People who listen to albums need a calmer,
cover-first context. The web client already knows the `is_album` flag, but it
was dropped when playlists crossed into the application contract.

The playlist contract now carries `isAlbum` end to end: the SoundCloud client
reads the flag, the application DTO exposes it, and the frontend type mirrors
it. The field is optional on deserialization so playlist JSON stored in SQLite
by earlier versions still loads; those rows simply render as playlists until
they are resolved again.

When the flag is set, the playlist page renders an album banner instead of the
compact hero: the artwork is used twice, once as a blurred backdrop and once as
a large square cover, and the information block carries an Album eyebrow, a
larger title, the linked artist, track count with total duration, and the
playback, favourite and download actions. Regular playlists keep the existing
header untouched. Track rows, auto-advance and prefetch behaviour are shared, so
an album plays exactly like any other list.

The layout stacks vertically below 560 px and shrinks the cover, mirroring the
artist banner rules. No new dependencies and no playback or storage changes are
involved. Album detection depends on the source response; playlists cached
before this change are treated as playlists until refreshed.
