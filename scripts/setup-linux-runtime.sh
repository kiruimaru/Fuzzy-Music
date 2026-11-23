#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "Usage: $0 /path/to/Fuzzy-Music.AppImage [--ffmpeg]" >&2
  exit 2
fi

appimage=$(realpath -- "${1:?AppImage path required}")
[[ -f "$appimage" ]] || { echo "AppImage not found: $appimage" >&2; exit 2; }
install_ffmpeg=false
if [[ $# -eq 2 ]]; then
  [[ $2 == --ffmpeg ]] || { echo "Unknown option: $2" >&2; exit 2; }
  install_ffmpeg=true
fi
[[ $(uname -m) == x86_64 ]] || {
  echo "This runtime setup supports Linux x86_64 only." >&2
  exit 1
}

tools_dir="$(dirname -- "$appimage")/tools"
mkdir -p "$tools_dir"
yt_dlp="$tools_dir/yt-dlp"
yt_dlp_sha256='767ba9dce7052aa196f8b12652f433d76de128ba168966fa84b6394d5e6c6795'
if [[ ! -f "$yt_dlp" ]] || ! printf '%s  %s\n' "$yt_dlp_sha256" "$yt_dlp" | sha256sum --check --status; then
  download=$(mktemp "$tools_dir/.yt-dlp.XXXXXX")
  trap 'rm -f -- "$download"' EXIT
  curl --fail --location --retry 3 --connect-timeout 15 --max-time 180 \
    'https://github.com/yt-dlp/yt-dlp/releases/download/2026.10.26/yt-dlp_linux' \
    --output "$download"
  printf '%s  %s\n' "$yt_dlp_sha256" "$download" | sha256sum --check --status ||
    { echo "Downloaded yt-dlp failed SHA256 verification." >&2; exit 1; }
  chmod 755 "$download"
  mv -f -- "$download" "$yt_dlp"
fi

for name in LICENSE THIRD_PARTY_LICENSES.txt; do
  case "$name" in
    LICENSE) expected='7e12e5df4bae12cb21581ba157ced20e1986a0508dd10d0e8a4ab9a4cf94e85c' ;;
    THIRD_PARTY_LICENSES.txt) expected='472aefe951c7db35e1657c1d13fd337140511ed6f2b329205105ad441c5a02b7' ;;
  esac
  notice="$tools_dir/yt-dlp-$name"
  if [[ ! -f "$notice" ]] || ! printf '%s  %s\n' "$expected" "$notice" | sha256sum --check --status; then
    curl --fail --location --retry 3 --connect-timeout 15 --max-time 60 \
      "https://raw.githubusercontent.com/yt-dlp/yt-dlp/2026.10.26/$name" \
      --output "$notice"
    printf '%s  %s\n' "$expected" "$notice" | sha256sum --check --status ||
      { echo "Downloaded yt-dlp $name failed SHA256 verification." >&2; exit 1; }
  fi
done

packages=(mpv)
if [[ $install_ffmpeg == true ]]; then
  packages+=(ffmpeg)
fi

install_packages() {
  local manager
  if command -v pacman >/dev/null 2>&1; then
    manager=pacman
  elif command -v apt-get >/dev/null 2>&1; then
    manager=apt
  elif command -v dnf >/dev/null 2>&1; then
    manager=dnf
  elif command -v zypper >/dev/null 2>&1; then
    manager=zypper
  else
    echo "Install mpv with your Linux distribution's package manager." >&2
    return 1
  fi

  local -a privilege=()
  if [[ $(id -u) -ne 0 ]]; then
    command -v sudo >/dev/null 2>&1 || {
      echo "Install mpv as root or install sudo, then rerun this script." >&2
      return 1
    }
    privilege=(sudo)
  fi

  case "$manager" in
    pacman) "${privilege[@]}" pacman -S --needed --noconfirm "${packages[@]}" ;;
    apt)
      "${privilege[@]}" apt-get update
      "${privilege[@]}" apt-get install -y --no-install-recommends "${packages[@]}"
      ;;
    dnf) "${privilege[@]}" dnf install -y "${packages[@]}" ;;
    zypper) "${privilege[@]}" zypper --non-interactive install "${packages[@]}" ;;
  esac
}

if ! command -v mpv >/dev/null 2>&1 || { [[ $install_ffmpeg == true ]] && ! command -v ffmpeg >/dev/null 2>&1; }; then
  install_packages
fi

echo "yt-dlp installed at: $yt_dlp"
mpv --version | head -n 1
if [[ $install_ffmpeg == true ]]; then
  ffmpeg -version | head -n 1
fi
