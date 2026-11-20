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
- Pending merge, tag push and the tagged Windows workflow result.
  Do not treat creating the tag or starting CI as a published release.

## Handoff

- Working branch: `release/0.1.0`.
- Manual clean-Windows, listening, real downloads and long-session acceptance
  remain separate from hosted build checks.
- Prior cache/worker/HTTP lifecycle issues are disclosed, not fixed here.
