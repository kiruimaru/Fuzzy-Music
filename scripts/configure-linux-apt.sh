#!/usr/bin/env bash
set -euo pipefail
root=${1:-/etc/apt}
# GitHub runners use a mirror+file indirection, not just direct mirror URLs.
find "$root" -maxdepth 2 -type f \( -name '*.list' -o -name '*.sources' \) -exec sed -i \
  -e 's|mirror+file:/etc/apt/apt-mirrors.txt|https://archive.ubuntu.com/ubuntu|g' \
  -e 's|http://azure.archive.ubuntu.com/ubuntu|https://archive.ubuntu.com/ubuntu|g' {} +
