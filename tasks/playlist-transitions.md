# Playlist transitions

Prepared streams are appended to mpv while the current file plays. mpv receives gapless-audio and prefetch-playlist options, and advances without waiting for a UI poll. The shell reports a transition counter and the UI updates the current queue entry without issuing another play command. If preparation finishes after EOF, normal explicit playback remains the fallback. Failed preparation retries with a delay.

Checks: shell unit tests including fixture-driven EOF transition; frontend tests including queue synchronization and retry; frontend production build. Real mpv and SoundCloud audio were not available on PATH for this session and audible gaps have not been measured.

Remaining: verify with real SoundCloud streams, pause and long seeks, unavailable tracks, and manual switching. mpv prefetch only opens the next entry once the current URL is fully read and has limitations for HLS. Signed stream URLs can expire after a long pause or backward seek; implement refresh/recovery for this case. Test actual network buffering and measure the transition before claiming gapless playback. Search performance needs separate benchmarking.

## Early lookahead for manual skips

While a playlist plays, resolve up to six neighbouring tracks sequentially in a cancellable background worker. Keep at most six temporary stream links in memory, refresh lookahead after 45 seconds, and use entries only for 60 seconds; stop and close clear them. Selecting a prepared track can bypass a second yt-dlp invocation. The current 30-second mpv queue preparation can also consume the cache. This caches links, not audio bytes; first playback, network buffering, a skip before prefetch completes, and expired or unavailable streams can still be slow. Automatic transition and memory use should be checked with real playlists. Search performance is unchanged.

## Bidirectional lookahead

Resolve up to three successors and three predecessors, alternating by distance so the nearest next and previous tracks are prioritized. The bounded six-entry cache is restricted to the current neighbourhood, and cached playback links survive manual track changes until expiry. Near-end automatic preparation only consumes links under 30 seconds old without discarding other fresh entries. The cache contains stream links, not buffered audio; immediately switching before the background worker reaches a neighbour may still require a fresh resolve. Live timing remains to be measured.


## Twenty-track link lookahead

The UI requests up to twenty nearest playlist neighbours, alternating forward and backward until the window is full, and the shell bounds its cache at twenty. Resolution remains sequential, cancellable and short-lived. This is link preparation only: no audio segments are cached yet. A live SoundCloud probe returned m3u8_native HLS, so an in-memory HLS-aware playback/cache design is still needed. It must handle segment URLs, proxy policy, expiry, seeks, cancellation, memory limits and fallback before claiming audio is preloaded.

## Parallel link preparation

The lookahead worker resolves batches of up to four tracks concurrently and stores each result as soon as it completes, while checking the current playback generation before insertion. Stop and selection cancel the shared request token. This improves link readiness only; it does not buffer HLS audio or guarantee immediate playback. The user's requirement for twenty rapidly selectable prebuffered audio tracks remains unimplemented.

## In-memory HLS startup cache

The shell downloads the first three segments of supported media playlists for up to twenty neighbours plus the current track. A loopback HTTP server supplies the rewritten manifest and cached bytes to mpv; remaining segments use their original HTTPS addresses. Audio bytes stay in RAM (up to 168 MiB of segment payload plus overhead and concurrent downloads), and stop/close clears entries. The network client uses the shell's proxy policy and mpv bypasses the proxy for loopback. Cache entries expire after 50 seconds; unsupported HLS forms or failed downloads fall back to the original stream. Link entries now survive selection, including the current track, to support repeated back-and-forth skips. Prefetch is asynchronous and is not guaranteed to finish before a skip. Real SoundCloud/mpv playback and latency remain to be measured; encrypted, master and byte-range playlists are not prebuffered.

## Larger lookahead and startup buffer

Lookahead covers up to forty neighbours and the current track. The RAM cache stores up to twelve audio segments per track and at most 6 MiB of audio data per track, with a 41-track limit (up to 246 MiB of retained audio, plus manifests, concurrent transfers and overhead). An initial manifest becomes available after three segments, before the remaining segments finish downloading. Stop clears the cache and invalidates in-flight work. Actual memory use depends on available tracks, stream sizes, cache expiry, and how quickly background requests finish; it is not padded to reach a fixed number. Signed links and entries remain short-lived and unsupported formats fall back to direct playback.

## Faster parallel preparation

The lookahead uses eight dynamic workers instead of fixed four-track batches; each worker starts the next nearest request as soon as it finishes. A cached link can be reused to retry missing audio without another extraction. Cancellation is checked between audio segment transfers. The retained audio limit is 3 MiB per track across at most 41 tracks (123 MiB), leaving headroom for network buffers, manifests, the shell and mpv; this is not a strict limit on total process or system memory. Playback timing and real-world memory usage still need measurement.

## Repeated skips and signed-link lifetime

Manual changes within a playlist no longer cancel the active lookahead; it can finish preparing tracks while the listener moves forward or backward. Stop and a replacement lookahead still cancel old work. Cached link and HLS audio lifetimes now use a numeric `expires` value when present, capped at 15 minutes with a two-minute safety margin; HLS entries use the earliest expiry among the playlist and its segments. Unknown expiry retains the previous short fallback. Fixture tests cover a prefetch that survives a manual skip, repeated forward/backward reuse, and expiry handling. Actual switching latency on the user's playlist remains to be verified.

## Thirty-minute cache lifetime

The upper bound for signed stream links and cached HLS audio is now 30 minutes. The source-provided expiry and two-minute safety margin still take precedence; links without a numeric expiry retain their short fallback. User confirmed the preceding playback behavior works well. Longer listening, signed-link expiry and memory behavior still depend on the source and should be checked in actual use.
