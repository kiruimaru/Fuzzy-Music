#!/bin/sh
set -eu
# The AppImage's WebKit/GTK libraries must not leak into host media programs.
unset LD_LIBRARY_PATH LD_PRELOAD PYTHONHOME PYTHONPATH
tool=$(basename "$0")
case "$tool" in
  mpv|yt-dlp) ;;
  *) echo "Unknown Fuzzy Music runtime tool: $tool" >&2; exit 126 ;;
esac
if [ "$tool" = yt-dlp ]; then
  data_home=${XDG_DATA_HOME:-${HOME:-}/.local/share}
  managed_tool="$data_home/io.github.kiruimaru.fuzzy-music/runtime/tools/yt-dlp"
  if [ -x "$managed_tool" ]; then
    exec "$managed_tool" "$@"
  fi
fi
if [ -n "${APPIMAGE:-}" ]; then
  app_dir=$(dirname -- "$APPIMAGE")
  adjacent_tool="$app_dir/tools/$tool"
  if [ -x "$adjacent_tool" ]; then
    exec "$adjacent_tool" "$@"
  fi
fi
if ! command -v "$tool" >/dev/null 2>&1; then
  echo "Fuzzy Music requires $tool installed on the host PATH; see the Linux release instructions." >&2
  exit 127
fi
exec "$tool" "$@"
