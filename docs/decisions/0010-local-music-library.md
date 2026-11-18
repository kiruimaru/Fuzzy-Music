# 0010: Favourite downloads and local music library

Date: 2026-11-19. Status: local implementation; contributor review pending.

The user explicitly expanded the prior MVP boundary to download favourited
tracks and index a user-selected music folder. This overlaps #21's ownership
of favourites/settings and must be coordinated with its contributor.

One SQLite database under Tauri application data owns schema version, settings,
favourite metadata and the local index. No audio data or signed stream URL is
stored in SQLite. SQLite errors are surfaced without replacing the database;
streaming still works if the library cannot be opened. Settings are partial
updates to avoid volume changes overwriting quality or folder.

An indexed local item has an opaque local id, never a UI-supplied filesystem
path. Rust plays only paths found in its index, through the existing mpv owner.
The configured music directory is scanned on startup and on manual refresh,
without following symlinks, with traversal bounds. Audio metadata uses Lofty.
Changing directories does not delete or move previously indexed files.

SoundCloud download logic remains in `crates/soundcloud`, uses the existing
managed process-group cancellation and network policy, never a shell, and
downloads to a hidden temporary staging directory on disk. One worker and a
bounded queue prevent a fan-out. Downloads use a 10-minute timeout, a 512 MiB
limit, and no-overwrite atomic publication into the music folder. The best
available source audio is the default; bitrate presets prefer <=320/192/128/64
kbit/s and fall back to available audio without transcoding. Explicit FLAC
output uses ffmpeg and does not recover quality lost by the source. Schema v2
adds a persisted light/dark theme with dark as the default for existing data.

Favourite state is independent from download success. Unliking cancels pending
work but does not delete published files; failed/missing files expose retry.
Shutdown cancels and joins the download worker. Interrupted pending jobs are
resumed on startup. Existing user files are not overwritten.

Further contributor review, real GUI listening, actual reboot/restart acceptance,
Windows packaging and ffmpeg/source-format availability remain separate checks.
