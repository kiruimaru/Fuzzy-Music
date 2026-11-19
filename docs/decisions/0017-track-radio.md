# 0017: Track radio

Date: 2026-11-19. Status: stacked above the custom background work.

A SoundCloud track row can start a radio queue. The seed track plays
immediately while the client asks the web API for tracks related to that id,
then appends the deduplicated recommendations to the same queue. Radio reuses
the playlist path, so gapless preparation, lookahead and playback recovery
behave exactly like playlist playback; the only difference is where the queue
entries come from.

The queue keeps growing while radio mode is active. When fewer than five tracks
remain after the current one, the client requests another page of related tracks
for the last queued entry and appends only entries not already present, keyed by
source and track id. A successful append clears the retry timer, so a short page
is topped up on the next status poll. An empty or failed request waits twenty
seconds before retrying, which keeps an unavailable recommendation service from
turning the two-hundred-fifty-millisecond status poll into a request storm.

Radio ends when the user starts another list, plays an entry from the session
queue, or presses Stop; pending requests are cancelled through their own
generation. A failed first page does not interrupt the seed: playback continues
and a short message appears in the player.

Only SoundCloud tracks can start radio. The source adapter parses the id as a
positive integer before any network access, and the application boundary rejects
local tracks. Each request is limited to twenty-five recommendations, uses the
same parsing and client-id refresh as artist tracks, and adds no dependencies.
Radio is a playback mode, not a saved entity: it is not persisted and does not
mark tracks as favourites or download them.
