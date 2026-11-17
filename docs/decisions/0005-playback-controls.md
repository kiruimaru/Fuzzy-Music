# 0005: Basic playback controls through mpv JSON IPC

Date: 2026-11-17. Status: local implementation; contributor review pending.

Extend the existing Play/Stop slice, without queue or persistence. Own one mpv
process per selected track. Connect to a unique Windows named pipe or Unix
socket using `--input-ipc-server`. Never expose the endpoint or stream URL to
React. Disable terminal output and discard diagnostics.

A single Windows pipe handle cannot be read and written concurrently: a blocked
read holds the pipe and the next command write deadlocks. Windows therefore
drains currently available bytes with `PeekNamedPipe` before each snapshot,
without a reading thread. Unix sockets use a reader thread and bounded channel.
On both platforms, a blocked write could still exceed the 3-second connect
deadline; the deadline covers connection setup, not command acknowledgment.

Observe `time-pos`, `duration`, `pause`, `volume`, and `seekable`. Actual mpv
events drive the 250ms status snapshot; controls do not optimistically mark
playback paused/playing. A non-null time position also confirms initialization
when file-loaded occurred before the IPC connection. Startup has a deadline.
IPC connection setup is bounded to three seconds on a blocking worker. It
currently holds the playback lock, so Stop/close can wait up to that deadline.

The native Windows pipe is byte-oriented (message mode is not enabled), so JSON
objects are delimited by newline and may arrive split or combined. Framing
keeps a pending buffer and only parses complete lines.

`update_playback` accepts the current playback generation (does not increment
it), and a tagged pause/seek/volume action. A stale generation cannot control a
new process. Pause/resume never resolves or restarts the track. Seek requires
known duration and seekability, clamps to duration, and uses milliseconds.
Volume is 0..100, default 100. Neither volume nor position is persisted yet.

Reader messages use a bounded channel. mpv command failures become a fixed safe
playback error, not raw diagnostics. Stop/close kill and reap the process; Unix
socket cleanup belongs to Player drop. Queue/EOF advance remains separate work.

The UI uses native accessible sliders, commits on pointer release or navigation
key release, and disables seek for unsupported streams. It preserves the
existing English interface strings; no personal names are added.
