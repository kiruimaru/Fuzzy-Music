# 0016: Playback resilience and diagnostic logging

Date: 2026-11-19. Status: stacked above the custom background work.

Long listening sessions could end with a playback error, and after a restart a
full-window animated background could make the whole machine feel slow. Both
symptoms need defensive behaviour in the shell and the renderer.

The startup cache now supports pinning. The shell pins the key of the track mpv
is reading, keeps that pin across gapless transitions, and releases the previous
key when the next track starts, when local playback begins, when playback stops
or when the application shuts down. Pinned entries are skipped by eviction and
survive expiry, so a long track can never lose the loopback segments that mpv
is still requesting; the link metadata cache stays independent of this.

Playback errors stop being terminal when a queue is being played. On an error
with automatic advance enabled, the renderer starts the next queue entry, or
replays the current entry at the end of the queue, using a fresh resolution so a
stale link cannot fail twice. At most two consecutive recovery attempts are
made; a successful playing state resets the counter. A track that fails
persistently is skipped instead of blocking the session.

Animated backgrounds are detected by file extension (GIF, WebP, AVIF) and the
root element gains `has-animated-background`. For those images the shell
surfaces drop the backdrop blur and use a more opaque tint: the tint still keeps
text readable, while the compositor no longer has to blur a changing backdrop
on every frame. Static backgrounds keep the frosted presentation.

mpv writes a session log to the application data directory (`mpv.log`), with the
verbose level limited to info and the file truncated when it exceeds two
megabytes. The log is not shown in the UI and contains no track metadata beyond
what mpv itself prints; it exists so a reported playback failure can be
diagnosed from the affected machine.
