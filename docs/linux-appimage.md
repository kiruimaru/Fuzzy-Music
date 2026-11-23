# Linux AppImage

The Linux release workflow builds x86_64 AppImages on Ubuntu 22.04 from each
new release tag. The additional-assets workflow remains for older releases.
The AppImage is unsigned; compatibility on every Linux distribution is not
established by a hosted build or an Xvfb startup check.

## Running

For 0.2.1, download `Fuzzy-Music-0.2.1-linux-x86_64.AppImage`, make it
executable, and open it. Missing playback tools are shown inside the app.
Press Install dependencies to download pinned, SHA256-verified yt-dlp and its
notices into `$XDG_DATA_HOME/io.github.kiruimaru.fuzzy-music/runtime/tools`
(default `~/.local/share/...`). The AppImage need not be writable.

To install mpv, check Install system packages (administrator approval) and
approve the desktop authentication prompt. Include ffmpeg is optional for
FLAC. This uses pkexec with pacman, apt-get, dnf or zypper; a polkit
authentication agent must be running. Nothing downloads or elevates silently.
Settings retains the dependency status and setup controls. Failed downloads
can be retried; package failures may need manual package-manager repair.
An explicit FUZZY_YTDLP override must be repaired manually.

```bash
chmod +x Fuzzy-Music-0.2.1-linux-x86_64.AppImage
./Fuzzy-Music-0.2.1-linux-x86_64.AppImage
```

Without FUSE use `APPIMAGE_EXTRACT_AND_RUN=1` before that command; for the
observed WebKit/Hyprland issue use `WEBKIT_DISABLE_DMABUF_RENDERER=1`.

### Older Release And Optional Sidecar Setup

The published 0.1.0 AppImage requires host-installed `mpv` and `yt-dlp` as
described in its release notes. The setup script below is included with
subsequent AppImage builds; it cannot change the already-published 0.1.0 image.

Run the setup script once to download a pinned, SHA256-verified `yt-dlp` beside
the AppImage and install `mpv` through the host distribution's package manager.
Use `--ffmpeg` to install the optional FLAC converter as well:

```bash
chmod +x Fuzzy-Music-0.1.0-linux-x86_64.AppImage
chmod +x setup-linux-runtime.sh
./setup-linux-runtime.sh ./Fuzzy-Music-0.1.0-linux-x86_64.AppImage --ffmpeg
./Fuzzy-Music-0.1.0-linux-x86_64.AppImage
```

The setup script supports pacman, apt, dnf and zypper. It requests elevated
permissions only when the host package manager needs them. Keep the `tools`
directory beside the AppImage; the wrapper prefers its local `yt-dlp` and
continues to use host `mpv`. `FUZZY_MPV` and `FUZZY_YTDLP` overrides remain
available.

FUSE 2 may be required by the AppImage runtime. Without FUSE, try
`APPIMAGE_EXTRACT_AND_RUN=1 ./Fuzzy-Music-0.1.0-linux-x86_64.AppImage`.
For the observed WebKit/Hyprland rendering problem, add the process-local
`WEBKIT_DISABLE_DMABUF_RENDERER=1` environment setting.

Executable-relative wrappers preserve the v0.1.0 application's tool-discovery
policy. They prefer tools installed beside the AppImage, then fall back to the
host PATH, clearing AppImage library and Python paths first. SoundCloud access,
listening and long-session behavior need manual testing.

The published 0.1.0 AppImage passed the Ubuntu 22.04 Xvfb startup check in
workflow `44716472401`. Its public download and all Linux asset checksums were
independently verified; its window also opened on an Arch desktop using the
process-local extract-and-run and WebKit settings above. These checks establish
startup, not working SoundCloud search, human listening or universal compatibility.

## Source and verification

Every asset has a SHA256 sidecar. The Linux provenance JSON records the exact
application commit/tree, packaging commit and original/packaged ELF and AppImage
digests. The bundler patches library lookup paths and strips ELF metadata; the
packaging check compares the nonempty code and read-only data sections, allowing
only Tauri's known UNK-to-APP bundle-type marker in the latter. The
existing `Fuzzy-Music-0.1.0-corresponding-source.zip` supplies the tagged
application, locked Rust dependency sources and frontend runtime sources. New
releases publish a matching versioned corresponding-source ZIP. The
additional `linux-packaging-source.tar.gz` supplies packaging scripts, workflow,
system copyright notices and exact Ubuntu source-package coordinates. Ubuntu
source packages include their upstream sources, downstream patches and build
recipes; enable the matching `deb-src` repositories and run the recorded
`apt-get source SOURCE=SOURCE_VERSION` commands. Preserve all source and notice
access when redistributing. The setup script downloads yt-dlp directly from its
upstream release and keeps its two license files beside the executable.
Bundled npm notices and the application license
are also inside `usr/share/doc/fuzzy-music` in the extracted AppImage.

No published tag or pre-existing Windows asset is overwritten.
