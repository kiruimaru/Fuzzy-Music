# 0020: Explicit Linux runtime setup inside the application

Date: 2026-11-23. Status: accepted.

The owner requested Linux downloads that do not require a separate terminal
bootstrap. This extends decision 0019: the optional sidecar script remains,
but the AppImage itself detects missing runtime tools and provides setup.

The renderer receives only a typed boolean availability snapshot. A Rust worker
downloads pinned upstream x86_64 yt-dlp and its two license notices, with SHA-256
verification, 180-second per-file deadlines, bounded sizes and atomic publication.
Files live in the user's XDG data directory, not the read-only AppImage or a
potentially unwritable Downloads folder. An explicit FUZZY_YTDLP override is
never replaced. AppImage wrappers prefer managed yt-dlp and preserve clean host
library/Python environments.

mpv and optional ffmpeg remain distribution packages rather than bundled
binaries. A separate unchecked consent checkbox authorizes pkexec to invoke
the detected host package manager with fixed arguments. No passwords are read
by the app, and no arbitrary commands or arguments come from the frontend.
The desktop must provide a polkit authentication agent. A missing agent,
declined request, stale package indexes or interrupted install is reported for
retry or manual package-manager repair. Package-manager interruption, including
window close or the ten-minute timeout, may require distribution-specific
transaction recovery; the application does not attempt privileged repairs.

No silent download or privilege elevation occurs. Existing runtime overrides
and manual installation remain supported. The UI stays usable on failure.
Version 0.2.0 gets its own immutable tag and GitHub Actions AppImage build;
the old 0.1.0 assets are untouched. Linux publication waits for the matching
Windows/corresponding-source release.
