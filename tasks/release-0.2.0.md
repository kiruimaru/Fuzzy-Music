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
- Published corrective 0.2.1 after the unpublished 0.2.0 attempt.
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
- Immutable v0.2.1 points to `5326c2531dc616bf3962ada670d4790258b616e0`.
- Windows run `45197019327` and Linux run `45197019220` passed all jobs.
- All twelve public assets downloaded; six SHA256 sidecars passed.
- Corresponding-source and Windows/Linux provenance match the tag commit/tree;
  both packaged executable digests were independently verified.
- Downloaded AppImage opened a mapped, non-hidden native window on Arch with
  extract-and-run and the process-local WebKit workaround. The isolated smoke
  service was stopped without touching the user's library.
- Final 0.2.1 local checks: native debug build, 45 frontend tests, ten browser
  fixtures, wrapper checks and eight release Node tests passed.

## Handoff

Requested integration and publication are complete. No open PRs remain.
Human listening, broader distribution acceptance and privileged clean-machine
installation are not established by fixtures, native startup or the download
test. Keep v0.1.0 and v0.2.0 immutable.
