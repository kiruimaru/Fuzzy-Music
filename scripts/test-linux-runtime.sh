#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "$0")/.." && pwd)
stage=$(mktemp -d)
trap 'rm -rf -- "$stage"' EXIT
mkdir -p "$stage/app with spaces/tools" "$stage/mounted/usr/bin/tools" "$stage/host"
for tool in mpv yt-dlp; do
  install -m755 "$root/scripts/linux-runtime-tool.sh" "$stage/mounted/usr/bin/tools/$tool"
  printf '%s\n' '#!/bin/sh' 'test -z "${LD_LIBRARY_PATH:-}${PYTHONHOME:-}${PYTHONPATH:-}" || exit 99' 'printf "%s\n" "$@"' > "$stage/host/$tool"
  chmod +x "$stage/host/$tool"
  actual=$(PATH="$stage/host:$PATH" LD_LIBRARY_PATH=/appimage/lib PYTHONHOME=/appimage/python PYTHONPATH=/appimage/python \
    "$stage/mounted/usr/bin/tools/$tool" --version 'argument with spaces')
  test "$actual" = $'--version\nargument with spaces'
done

printf '%s\n' '#!/bin/sh' 'test -z "${LD_LIBRARY_PATH:-}${PYTHONHOME:-}${PYTHONPATH:-}" || exit 99' 'printf "adjacent:%s\n" "$@"' > "$stage/app with spaces/tools/yt-dlp"
chmod +x "$stage/app with spaces/tools/yt-dlp"
actual=$(APPIMAGE="$stage/app with spaces/Fuzzy Music.AppImage" PATH="$stage/host:$PATH" \
  LD_LIBRARY_PATH=/appimage/lib PYTHONHOME=/appimage/python PYTHONPATH=/appimage/python \
  "$stage/mounted/usr/bin/tools/yt-dlp" --version 'argument with spaces')
test "$actual" = $'adjacent:--version\nadjacent:argument with spaces'

managed="$stage/user data/io.github.kiruimaru.fuzzy-music/runtime/tools"
mkdir -p "$managed"
cp "$stage/app with spaces/tools/yt-dlp" "$managed/yt-dlp"
actual=$(XDG_DATA_HOME="$stage/user data" PATH="$stage/host:$PATH" \
  "$stage/mounted/usr/bin/tools/yt-dlp" 'managed argument')
test "$actual" = 'adjacent:managed argument'

# Isolate PATH but retain basename so the missing-runtime branch is exercised.
ln -s "$(command -v basename)" "$stage/host/basename"
rm "$stage/host/mpv"
set +e
PATH="$stage/host" "$stage/mounted/usr/bin/tools/mpv" 2>"$stage/error"
status=$?
set -e
test "$status" = 127
grep -q 'requires mpv' "$stage/error"
grep -q 'yt_dlp_sha256=' "$root/scripts/setup-linux-runtime.sh"
grep -q 'sha256sum --check --status' "$root/scripts/setup-linux-runtime.sh"
echo 'Linux host-tool wrappers: passed'
