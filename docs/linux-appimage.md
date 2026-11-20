# Linux AppImage

The additional-assets workflow builds x86_64 AppImages on Ubuntu 22.04 from the
unchanged release tag. It does not change the application/frontend contracts.
The AppImage is unsigned; compatibility on every Linux distribution is not
established by a hosted build or an Xvfb startup check.

## Running

Install `mpv` and a current `yt-dlp` through your distribution or their upstream
instructions. Optional `ffmpeg` is required for FLAC conversion. These host
programs are not redistributed in the AppImage. For example, on Arch Linux:

```bash
sudo pacman -S mpv yt-dlp ffmpeg
chmod +x Fuzzy-Music-0.1.0-linux-x86_64.AppImage
./Fuzzy-Music-0.1.0-linux-x86_64.AppImage
```

FUSE 2 may be required by the AppImage runtime. Without FUSE, try
`APPIMAGE_EXTRACT_AND_RUN=1 ./Fuzzy-Music-0.1.0-linux-x86_64.AppImage`.
For the observed WebKit/Hyprland rendering problem, add the process-local
`WEBKIT_DISABLE_DMABUF_RENDERER=1` environment setting.

Executable-relative wrappers preserve the v0.1.0 application's tool-discovery
policy and dispatch to host `mpv`/`yt-dlp`, clearing AppImage library and Python
paths first. `FUZZY_MPV` and `FUZZY_YTDLP` still override these wrappers.
SoundCloud access, listening and long-session behavior need manual testing.

## Source and verification

Every asset has a SHA256 sidecar. The Linux provenance JSON records the exact
application commit/tree, packaging commit and original/packaged ELF and AppImage
digests. The bundler patches library lookup paths and strips ELF metadata; the
packaging check compares the nonempty code and read-only data sections, allowing
only Tauri's known UNK-to-APP bundle-type marker in the latter. The
existing `Fuzzy-Music-0.1.0-corresponding-source.zip` supplies the tagged
application, locked Rust dependency sources and frontend runtime sources. The
additional `linux-packaging-source.tar.gz` supplies packaging scripts, workflow,
system copyright notices and exact Ubuntu source-package coordinates. Ubuntu
source packages include their upstream sources, downstream patches and build
recipes; enable the matching `deb-src` repositories and run the recorded
`apt-get source SOURCE=SOURCE_VERSION` commands. Preserve all source and notice
access when redistributing. Bundled npm notices and the application license
are also inside `usr/share/doc/fuzzy-music` in the extracted AppImage.

No published tag or pre-existing Windows asset is overwritten.
