# mpv playlist transitions

The shell appends one resolved successor to its owned mpv process and enables mpv playlist prefetch and gapless audio. A transition counter in playback status allows the frontend to update its queue without issuing a second play command. The existing explicit play path remains the fallback when preparation arrives after EOF. Stream URLs remain temporary and are not persisted.

mpv prefetch behavior depends on stream type; this design does not guarantee a gapless transition, especially for HLS or interrupted networks. Pause and backward seek may cause a queued signed URL to expire, requiring future refresh/recovery work. Live audio timing is still required.

## RAM startup segments

For supported HLS media playlists, keep at most initialization data and three initial segments per track in RAM and serve a rewritten manifest on loopback to mpv. Bound each segment to 2 MiB and retain at most 21 tracks. A 50-second lifetime limits use of expiring signed links; stop and close invalidate entries. Requests use the existing proxy policy, while mpv bypasses it for loopback. Unsupported playlists or failed downloads retain the original direct-stream path. This improves startup after successful prefetch but does not guarantee gapless or instant playback; live measurement remains required.

## Expanded lookahead

Retain up to forty neighbouring tracks plus the current track, with up to twelve audio segments and 6 MiB of audio per track. Publish a usable manifest after three segments and extend it in the background. Retained audio is capped at 246 MiB; transient transfers, manifests, and runtime allocations are additional. Memory is not allocated merely to reach a target value.

## Lookahead scheduling and memory headroom

Use eight workers drawing nearby tracks from an ordered queue instead of waiting for each fixed batch. Reuse valid links to retry missing audio and check cancellation between segment transfers. Limit retained audio to 3 MiB per track (up to 123 MiB across 41 tracks); manifests, in-flight transfers, runtime overhead, and the separate mpv process use additional memory. This is not a hard cap on total memory consumption.

## Repeated playlist navigation

A manual selection does not cancel the active playlist lookahead. Cancellation remains on stop, close, or replacement by a new lookahead. To avoid discarding prepared tracks after one minute, signed stream and HLS segment expiry is respected with a two-minute safety margin and a maximum retention of fifteen minutes. Sources without a numeric expiry use the earlier short conservative lifetime. Real playback still depends on the source continuing to honour the signed link and on completed prefetch.

## Longer cache retention

The maximum retention of signed links and HLS entries is 30 minutes, still constrained by the earliest actual expiry with a two-minute safety margin. Unknown expiry continues to use the short fallback.
