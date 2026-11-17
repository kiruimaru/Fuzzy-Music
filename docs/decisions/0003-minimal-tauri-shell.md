# 0003: Minimal Tauri shell and owned mpv process

Date: 2026-11-17. Status: implemented for issue #14; contributor review pending.

Use the application DTOs and policies merged in PR #15 unchanged. The shell
owner handles worker dispatch, IPC cancellation, generation guards, playback,
and dependency discovery. The source adapter stays unchanged. This records the
implementation choice, not a claim that both human owners reviewed it together.

Keep Tauri in `src-tauri` and React/TypeScript in `src`. Commands run all source
work, process creation, polling and process waits in Tauri blocking workers.
Search and playback each have one cancellation token and one monotonic intent
generation. Reject equal/older generations on Rust entry as well as completion:
Stop can reach a worker before an older Play has even started. React also guards
all asynchronous completion and status polling. New webview sessions start from
a timestamp-based generation within JavaScript's safe integer range.

Every Play resolves `Track.sourceUrl` afresh through `fuzzy-application`.
Rust starts mpv directly with the returned URL, disables its video/window,
configuration and extractor, and owns its process group/job. No shell, downloaded
track file, cached stream URL, or frontend process access is involved. A fixed
stdout marker from `--term-playing-msg` confirms playback initialization;
process exit and a 30-second startup deadline produce stopped/error states.
The marker does not prove that a listener heard audio or that an output is unmuted.
Diagnostics are discarded; only fixed safe errors reach the UI.

For this slice, expose `search_tracks`, `cancel_search`, `play_track`,
`stop_playback`, and a small `playback_status` poll every 250 ms. This status
is not the future full position/volume PlaybackSnapshot. mpv JSON IPC, pause,
seek, volume and queue are deferred. Stop kills and reaps the owned process
group/job and cancels resolve under the same lock used to accept its completion.
Closing the window cancels both operations, waits on a worker for source cleanup,
and stops owned playback before exiting. Exit also runs cleanup.

Find tools on PATH, with `FUZZY_YTDLP` and `FUZZY_MPV` executable overrides.
No automatic downloads, packaging, persisted settings or credentials are added.
Linux GTK/WebKit dependencies are system libraries; Windows needs WebView2 and
MSVC tools. Packaging and Windows validation remain separate release concerns.
