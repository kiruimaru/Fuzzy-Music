#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "$0")/.." && pwd)
stage=$(mktemp -d)
trap 'rm -rf -- "$stage"' EXIT
mkdir -p "$stage/sources.list.d"
printf '%s\n' 'deb mirror+file:/etc/apt/apt-mirrors.txt jammy main universe' > "$stage/sources.list"
printf '%s\n' 'URIs: http://azure.archive.ubuntu.com/ubuntu/' > "$stage/sources.list.d/ubuntu.sources"
printf '%s\n' 'deb https://packages.microsoft.com/ubuntu/22.04/prod jammy main' > "$stage/sources.list.d/microsoft.list"
bash "$root/scripts/configure-linux-apt.sh" "$stage"
grep -qx 'deb https://archive.ubuntu.com/ubuntu jammy main universe' "$stage/sources.list"
grep -qx 'URIs: https://archive.ubuntu.com/ubuntu/' "$stage/sources.list.d/ubuntu.sources"
grep -qx 'deb https://packages.microsoft.com/ubuntu/22.04/prod jammy main' "$stage/sources.list.d/microsoft.list"
echo 'Linux APT direct/mirror-list sources: passed'
