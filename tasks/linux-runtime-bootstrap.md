# Task: Linux runtime bootstrap

## Goal

Make the Linux AppImage setup acquire its runtime tools through a verified
script distributed beside the application, without redistributing media-tool
binaries from this project.

## Scope

### In scope

- Download pinned yt-dlp for Linux x86_64 with SHA256 verification.
- Keep upstream yt-dlp license notices beside the downloaded executable.
- Install mpv through pacman, apt, dnf or zypper when it is missing.
- Optionally install ffmpeg for FLAC conversion.
- Let AppImage wrappers prefer the adjacent yt-dlp and preserve host fallback.
- Show platform-correct dependency repair guidance.

### Out of scope

- Silent startup downloads or privileged installation.
- Bundling mpv/ffmpeg or changing their upstream source obligations.
- Replacing or rebuilding assets under the published v0.1.0 tag.

## Context

- Relevant files: `scripts/setup-linux-runtime.sh`, `scripts/linux-runtime-tool.sh`,
  `scripts/package-appimage.sh`, `.github/workflows/platform-assets.yml`.
- Decision: `docs/decisions/0019-linux-runtime-setup.md`.
- The repo is currently on `feat/linux-runtime-bootstrap`, based on the open
  PR stack through #35.

## Progress

### Completed

- Added the verified setup script, AppImage sibling-tool dispatch, workflow
  packaging/checksum inclusion, and platform-specific dependency errors.
- Launched the published v0.1.0 AppImage for manual UI testing. It opens, but
  that immutable binary does not contain the new wrapper or setup behavior.
- After the owner installed system prerequisites, built the current working
  tree as a standalone debug binary and confirmed its visible native window.
  The complete Rust workspace test suite now passes.
- The owner then requested full integration and in-app Linux setup. PRs
  #32-#35 are merged; decision 0020 extends the sidecar with explicit in-app
  setup. See `release-0.2.0.md` for publication progress.

### Remaining

- Build an AppImage on the now-prepared Linux host.
- Build and manually test the next-version AppImage with mpv installed.
- Publish only under a new version; do not replace v0.1.0 assets.

## Verification

Commands and results:

```text
npm test
42 tests passed on PR stack through #35.
npm run build
Passed on PR stack through #35.
cargo test -p fuzzy-application -p fuzzy-soundcloud --all-targets --locked
28 passed, 1 ignored.
cargo clippy -p fuzzy-application -p fuzzy-soundcloud --all-targets --locked -- -D warnings
Passed.
cargo fmt --all --check
Passed.
bash scripts/test-linux-runtime.sh
Passed.
bash scripts/test-linux-apt.sh
Passed.
node --test scripts/release-assets.test.mjs scripts/check-release.test.mjs
8 passed.
Full cargo test --workspace --all-targets --locked
Blocked: missing javascriptcoregtk-4.1 development package; sudo installation is unavailable.
Local build/launch retry on 2026-11-23
npm test: 42 passed. npm run tauri -- build --debug --no-bundle:
frontend passed; native build failed because javascriptcoregtk-4.1.pc and
webkit2gtk-4.1.pc are missing. No new binary was launched.
sudo -n true: a password is required. mpv and yt-dlp are absent from PATH.
Host prerequisite command: sudo pacman -S --needed webkit2gtk-4.1 mpv yt-dlp
After owner installed prerequisites on 2026-11-23
npm run tauri -- build --debug --no-bundle: passed.
cargo test --workspace --all-targets --locked: 66 passed, 4 ignored.
Native debug binary launched via fuzzy-music-local.service with
WEBKIT_DISABLE_DMABUF_RENDERER=1; Hyprland confirms a mapped, non-hidden window.
Real sidecar setup
Downloaded yt-dlp and verified its executable and both license SHA256 hashes; mpv was mocked.
```

## Handoff

- Current branch: `feat/linux-runtime-bootstrap`.
- Files changed: Linux runtime scripts/workflow, dependency diagnostics,
  release documentation, decision and task context.
- Known risks: native debug build and startup are verified, but live search,
  audible playback and next-version AppImage acceptance remain unverified.
- Next action: build and test the AppImage on the prepared Linux host,
  after stacked PR review/CI.
