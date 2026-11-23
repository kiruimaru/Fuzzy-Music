# Task: Integrate PRs and publish 0.2.1 with Linux setup

## Goal

Merge every open PR and publish an easy Linux download with explicit in-app
runtime setup, without changing previously published 0.1.0 assets.

## Progress

- Merged #32, #33, #34 and #35 with individual merge commits into main.
- Fast-forwarded their stacked base branches so GitHub marks all four merged.
- GitHub open-PR query returned an empty list after integration.
- Added typed runtime status/install commands and first-run/settings controls.
- Downloads use fixed upstream URLs, pinned hashes, limits and atomic writes.
- Host package installation uses fixed arguments via pkexec with explicit consent.
- Added new-tag AppImage build/publication workflow and 0.2.0 release notes.
- Publication and independent download verification are pending.
- Immutable v0.2.0 triggered Linux run 45193904521 (AppImage build/startup
  passed) and Windows run 45193904646 (strict Clippy failed before publication).
- Reproduced and fixed Windows-only needless_bool lint for new v0.2.1.
- Added a root-owned package timeout and nonblocking elevated-child reaping;
  cancellation may leave an authorized transaction completing in the background.

## Verification

- Frontend: 45 tests passed; TypeScript/Vite build passed.
- Browser fixtures: ten tests passed at 320px/1000px; setup screenshots inspected.
- Rust: 68 tests passed before the added opt-in live test; normal ignored count
  is now five. Live pinned download/notices/version test passed separately.
- Linux wrapper tests, context smoke and eight release Node tests passed.
- Strict Clippy, rustfmt, native debug build and actionlint passed.
- Corrective 0.2.1: 68 Rust tests passed, five ignored; strict Clippy passed.
- GitHub workflows/publication remain to be run from the new immutable tag.

## Handoff

Remaining: finalize checks, commit corrective work, integrate it into main,
push immutable v0.2.1 tag, monitor both platform workflows, download and verify
published assets. Human listening and privileged clean-machine installation
are not established by fixtures or the download test.
