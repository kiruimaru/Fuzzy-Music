# Task: First public release 0.1.0

## Goal and authorization

The owner requested merging into main and publishing the first 0.1 release
through GitHub Actions. All feature PRs were already integrated. Use SemVer
`0.1.0` and tag `v0.1.0`; do not claim unverified platform acceptance.

## Scope

- Align npm/Tauri/Rust versions and derive archive names from the config.
- Extend the existing Windows workflow to publish on version tags after checks.
- Publish the Windows x64 portable ZIP and corresponding source, each with SHA256.
- Preserve direct upstream runtime acquisition and add the previously omitted
  lucide runtime package to corresponding source.
- Document setup, optional ffmpeg, unsigned/no-installer status and known limits.
- No Linux binary, updater, signing, installer or application redesign.

## Implementation

- `windows-alpha.yml` keeps its existing filename and PR/manual triggers,
  adds tag builds and a narrowly permissioned publication job.
- Tag/config mismatch, missing release notes, test/build errors, package assertions
  or failed SHA256 validation prevent publication. Assets upload through the
  GitHub CLI's draft-before-publish creation path.
- `scripts/check-release.mjs` checks npm/Tauri/Cargo version alignment.
- Archives use URL-safe versioned names; both retain matching Git/executable
  provenance. The source includes all direct frontend runtime packages.

## Verification and publication

- Local: version guard and its four regression tests passed; 60 Rust tests
  passed (four ignored), 37 frontend tests and frontend production build passed.
- Strict Clippy, rustfmt, actionlint 1.7.7, context smoke and whitespace checks passed.
- Package assertions now verify URL-safe archive versions, matching clean Git
  provenance across both ZIPs, and the actual executable's SHA256.
- PR #29 merged as `5baa7c28842188cfa68563d35d9e3702a612b19d`.
  Annotated tag `v0.1.0` points to that commit; keep the published tag immutable.
- Tagged workflow [44706880926](https://github.com/kiruimaru/Fuzzy-Music/actions/runs/44706880926)
  passed both jobs: Windows tests/build/package in 9m25s and publication in 16s.
- [Fuzzy Music 0.1.0](https://github.com/kiruimaru/Fuzzy-Music/releases/tag/v0.1.0)
  published on 2026-11-20 at 09:08:56 UTC; not a draft/prerelease, and Latest.
- Verified all four public assets by downloading the release: portable ZIP
  5,664,478 bytes; corresponding source ZIP 108,387,383 bytes; two SHA256 sidecars.
  Both archive checksums passed. Matching provenance reports version 0.1.0,
  the tagged commit and `dirty=false`; the extracted EXE's digest matches it.
  Tauri/npm versions and included lucide runtime sources were checked in the ZIP.

## Handoff

- Release branch `release/0.1.0` merged; checkout returned to `main`.
- Source/executable artifacts stay pinned to the release commit; the later
  documentation handoff must not move the tag or replace published files.
- Manual clean-Windows, listening, real downloads and long-session acceptance
  remain separate from hosted build checks.
- Prior cache/worker/HTTP lifecycle issues are disclosed, not fixed here.
- The successful jobs warned that pinned actions target deprecated Node 20
  and ran on forced Node 24. Refreshing action SHAs is future maintenance,
  not a release failure.
