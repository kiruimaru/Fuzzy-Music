#!/bin/sh
set -eu
# The AppImage's WebKit/GTK libraries must not leak into host media programs.
unset LD_LIBRARY_PATH LD_PRELOAD PYTHONHOME PYTHONPATH
tool=$(basename "$0")
case "$tool" in
  mpv|yt-dlp) ;;
  *) echo "Unknown Fuzzy Music runtime tool: $tool" >&2; exit 126 ;;
esac
if ! command -v "$tool" >/dev/null 2>&1; then
  echo "Fuzzy Music requires $tool installed on the host PATH; see the Linux release instructions." >&2
  exit 127
fi
exec "$tool" "$@"
