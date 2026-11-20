# Windows distribution and provenance

The portable ZIP contains Fuzzy Music, its GPL-3.0 license and npm package
notices in `licenses/npm/`. The separate **corresponding-source** download on the
same release or Actions run contains exact application sources/build scripts/lockfiles,
locked Rust dependency sources/licenses in `vendor/`, and frontend runtime sources.
Both archives carry matching SOURCE-PROVENANCE.json (Git commit/tree, dirty flag,
executable SHA256); each has its own checksum. Distributors must keep the matching
source download available alongside the executable, not just link to a moving branch.
Build from the extracted source using the Windows prerequisites in README: `npm ci`, then
`npm run tauri -- build --no-bundle`. The vendored Cargo configuration supports
offline Rust dependency resolution. Node build tools are acquired using the lockfile.
WebView2 is a separately installed Microsoft runtime, not included in this ZIP.

## External runtime acquisition (not redistributed)

Run the included `setup-windows-runtime.ps1` once. It downloads directly from
upstream over HTTPS and checks the following SHA256 values before extraction/use.
The generated `tools/runtime-provenance.json` records versions, URLs and digests.
The script preserves all mpv archive files, including upstream documentation.
It also acquires yt-dlp LICENSE and THIRD_PARTY_LICENSES.txt from the release tag.

| Component | Exact upstream asset | SHA256 |
|---|---|---|
| mpv | https://github.com/shinchiro/mpv-winbuild-cmake/releases/download/20261111/mpv-x86_64-20261111-git-bcde6b93cc.7z | f259b5fd4b5c395617c948a4fadcdf55bba1c5282fb7c13e88626b3f71072baf |
| yt-dlp | https://github.com/yt-dlp/yt-dlp/releases/download/2026.10.26/yt-dlp.exe | f5389d3698ecde4ef06f4e6b1ce0294fe9ac1396228c8990b8d3416d6d177a7d |

mpv defaults to GPL-2.0-or-later. Its binary statically incorporates FFmpeg and
many other libraries with distinct licenses; the companion FFmpeg revision is
cd4e96eb3. Build recipes: https://github.com/shinchiro/mpv-winbuild-cmake/tree/20261111 ;
build run: https://github.com/shinchiro/mpv-winbuild-cmake/actions/runs/42927840447 .
That run exposes binaries/debug artifacts, not a complete corresponding-source
archive. Recipes include moving Git dependencies; downloading current HEADs is
not an exact source reconstruction. Consequently this distribution does not redistribute
mpv, FFmpeg, d3dcompiler_43.dll or the upstream archive. A user-installed `tools/`
directory must not be repackaged as this project's distribution without supplying
the complete matching sources, build scripts and required notices.

yt-dlp's own source is Unlicense, but its PyInstaller executable has bundled
components with other licenses. The upstream 2026.10.26 THIRD_PARTY_LICENSES.txt
includes their full texts and a source fallback contact, maintainers@yt-dlp.org:
https://github.com/yt-dlp/yt-dlp/blob/2026.10.26/THIRD_PARTY_LICENSES.txt .
That upstream offer is not represented as a source offer by Fuzzy Music.
The executable is acquired by the user directly, not shipped by this project.

The previous developer-host all-in-one ZIP is a local test candidate only. The
GitHub Actions artifact is the bootstrap distribution described above, without
external runtime binaries. FLAC conversion requires a separately installed
`ffmpeg.exe` on PATH; it is not included or acquired by the runtime setup script.
