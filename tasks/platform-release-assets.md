# Task: Additional 0.1.0 platform assets

## Goal

Owner requested a Linux AppImage and a separately downloadable Windows EXE
after publication of 0.1.0. Preserve the existing tag and Windows artifacts.

## Implementation

- Dispatch-only publication workflow builds the exact release tag separately
  from the packaging checkout. PR events run packaging regression checks only.
- Windows EXE and runtime bootstrap are extracted from the verified existing
  portable ZIP; the EXE must match clean tagged-commit provenance.
- AppImage overlay supplies executable-relative host-tool wrappers without
  modifying tagged application code or redistributing mpv/yt-dlp.
- Additional source/provenance/checksum assets accompany Linux packaging.
- Uploads never clobber; an existing different asset causes failure.

## Verification

Local: six Node tests, Linux wrapper tests (arguments/spaces, environment cleanup,
missing runtime), Bash syntax, actionlint and whitespace checks passed. The
standalone Windows EXE extracted from the published ZIP matches the original
tagged commit and executable SHA256.

Pending PR merge, hosted packaging and public downloads.

## Handoff

Do not describe the standalone EXE as an installer or dependency-free binary.
Do not move v0.1.0 or replace existing published assets.
