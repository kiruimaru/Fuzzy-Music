# Favourites and local music library

## Goal

Implement the user-requested SoundCloud-like player controls, long timeline,
hover/focus vertical volume, persistent favourites, download quality and a
user-selected folder whose audio is indexed at startup.

## Scope and decisions

- Branch `feat/favourites-local-library` builds on unmerged #22 PR #24.
- This explicitly expands the earlier no-download MVP and overlaps #21; no PRs
  were merged or contributor-owned work overwritten. Coordinate before merge.
- One versioned SQLite database stores preferences, stable favourite metadata
  and the local index. No private filesystem paths or audio URLs are sent as
  Track metadata; no audio bytes are stored in the DB.
- A single cancellable worker downloads the best available source format by
  default. Bitrate presets prefer <=320/192/128/64 kbit/s with fallback to best.
  FLAC output uses ffmpeg and does not improve the quality of the source.
  Files are staged on disk, published without overwriting and never deleted on
  unlike/folder change. Failed downloads retain the like and show retry.
- `lofty` indexes supported local audio; startup and manual refresh scan with
  bounded traversal. Local files play using opaque library IDs through owned mpv.
- See decision 0010 and contracts for details and explicit limitations.

## Verification

- PNG mascot and SoundCloud source badge are included; VK and YouTube assets
  are reserved for future providers. The final branch passed 31 frontend
  tests, eight Chromium tests, 53 Rust tests (four ignored), and native debug
  build on Linux.
- Queue is a player popover; search navigation is no longer duplicated.
- Settings support dark/light themes, additional bitrate preferences and FLAC
  conversion via ffmpeg. Schema v1 migrates to v2, retaining existing settings.
- 53 Rust tests passed / 4 ignored, 30 frontend tests and eight browser tests
  passed; strict Clippy/rustfmt and native debug build were run on Linux.
- Live yt-dlp/ffmpeg conversion of one SoundCloud track produced FLAC; this was
  a command-line check, not a GUI download or proof of lossless source audio.
- Volume is sent during slider movement; pending intermediate values are
  coalesced and commands remain ordered. Native listening still needs confirmation.
- Rust locked workspace/all-target tests: 52 passed, 4 ignored.
- Strict Clippy/rustfmt passed; 28 frontend tests and TypeScript/Vite build passed.
- Eight Chromium fixture browser tests passed at desktop and 320px, including
  favourite/offline selection, folder setting and vertical volume popover.
- Native Tauri debug build passed and the Linux application window opened.
- Real yt-dlp proxied SoundCloud download succeeded into a temporary directory;
  ffprobe identified AAC ~160 kbit/s. This checks availability for one source
  track only, not a promise of lossless or universal download availability.
- Human GUI/listening, genuine like/restart/import, Windows build and clean
  environment remain outstanding until explicitly verified.

## Remaining

- Coordinate settings/favourites with #21 owner and review dependent PR strategy.
- Verify GUI/actual downloads and folder choice with the user; verify Windows.
- Existing source-playlist cache/worker/HTTP lifecycle findings remain separate.
