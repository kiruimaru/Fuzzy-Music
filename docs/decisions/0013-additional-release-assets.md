# 0013: Add platform assets without moving published tags

The owner requested Linux AppImage and a separately downloadable Windows EXE
after v0.1.0 was published. Published tags and existing release assets remain
unchanged. A manual workflow checks out the application tag separately from its
own packaging scripts and adds assets only after validation.

The Windows EXE is extracted from the existing portable ZIP, not rebuilt or
represented as an installer. Its hash must match the original clean tagged
provenance. Its external runtime bootstrap remains a separate download.

The Linux overlay adds wrappers under usr/bin/tools, preserving decision 0005
and the exact tagged application code. These wrappers run host mpv and yt-dlp
with AppImage library/Python paths cleared. No external media-tool binaries are
redistributed. Ubuntu 22.04 x86_64 is the build baseline, not a claim of universal
Linux compatibility. Explicit FUZZY_MPV/FUZZY_YTDLP overrides remain supported.

AppImage provenance distinguishes application and packaging commits. Matching
application sources remain available in the existing corresponding-source ZIP;
packaging sources, system-package coordinates and notices are additional assets.
Uploads are idempotent only for identical bytes and never use --clobber. A
rebuild that differs from an already-published additional asset fails rather
than replacing it. A new version is required for application-code fixes.
