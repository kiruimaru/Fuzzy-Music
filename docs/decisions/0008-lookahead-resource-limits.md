# 0008. Lookahead resource limits

## Context

Long listening sessions made the whole system lag: FPS drops across the
machine while a playlist played. Investigation showed the first lookahead
cycle resolved and warmed all forty neighbours at the same moment, so their
cached links expired together. Every thirty minutes a storm of up to forty
extractor processes and hundreds of segment downloads ran through the local
proxy, and the RAM audio cache held over a hundred MiB at steady state. Live
probes of signed links showed they stay valid for roughly two hours, which
the thirty-minute cap ignored.

## Decision

SoundCloud signs stream links for about two hours, so cached links now stay
fresh for up to fifty-five minutes instead of thirty. That alone halves the
number of storms. Extractor launches are additionally bounded to sixteen per
lookahead cycle; tracks beyond the budget defer to the next cycle, which
spreads resolution over time instead of piling it into one burst. Only the
three nearest neighbours keep audio startup segments in RAM; the rest of the
lookahead stores links alone, so the cache costs a few MiB instead of a
hundred. mpv demuxer caches get explicit byte caps so prefetch-playlist
cannot grow without bound. Expired cache entries are dropped on lookup
instead of lingering until eviction.

## Consequences

Refresh work becomes small, spread-out background activity rather than
periodic storms, and steady-state RAM stays tens of MiB lower. Startup for
tracks beyond the nearest three falls back to a plain network stream, which
is the same behaviour as before the RAM cache existed and is acceptable
because gapless playback only needs the next track ready. Deferred
resolutions retry on the next forty-five second cycle, so a slow network
delays link refresh rather than dropping it.
