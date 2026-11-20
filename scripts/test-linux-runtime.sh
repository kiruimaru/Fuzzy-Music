#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "$0")/.." && pwd)
stage=$(mktemp -d)
trap 'rm -rf -- "$stage"' EXIT
mkdir -p "$stage/app with spaces/tools" "$stage/host"
for tool in mpv yt-dlp; do
  install -m755 "$root/scripts/linux-runtime-tool.sh" "$stage/app with spaces/tools/$tool"
  printf '%s\n' '#!/bin/sh' 'test -z "${LD_LIBRARY_PATH:-}${PYTHONHOME:-}${PYTHONPATH:-}" || exit 99' 'printf "%s\n" "$@"' > "$stage/host/$tool"
  chmod +x "$stage/host/$tool"
  actual=$(PATH="$stage/host:$PATH" LD_LIBRARY_PATH=/appimage/lib PYTHONHOME=/appimage/python PYTHONPATH=/appimage/python \
    "$stage/app with spaces/tools/$tool" --version 'argument with spaces')
  test "$actual" = $'--version\nargument with spaces'
done
# Isolate PATH but retain basename so the missing-runtime branch is exercised.
ln -s "$(command -v basename)" "$stage/host/basename"
rm "$stage/host/mpv"
set +e
PATH="$stage/host" "$stage/app with spaces/tools/mpv" 2>"$stage/error"
status=$?
set -e
test "$status" = 127
grep -q 'requires mpv' "$stage/error"
echo 'Linux host-tool wrappers: passed'
