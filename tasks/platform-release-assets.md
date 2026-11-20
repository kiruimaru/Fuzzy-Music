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

PR #30 merged as `716b923`. Workflow `44710709445` published the Windows
EXE (15,822,336 bytes), runtime setup script and provenance with SHA256 sidecars.
All six assets were downloaded again and verified; the public EXE is identical
to the original portable executable. The AppImage built (80.50 MiB), but the
first attempt correctly stopped before publication because the validation used
whole-file identity despite linuxdeploy's ELF processing. The follow-up compares
nonempty .text/.rodata and records both original and packaged ELF hashes.
The second attempt also stopped before publication on Tauri's documented
UNK-to-APP marker inside .rodata. The checker now normalizes only that exact
marker and rejects unrelated changes; code remains byte-identical. A third
workflow, `44713298480`, was cancelled after its Azure Ubuntu mirror throttled
package downloads for over six minutes. The workflow now selects the official
HTTPS archive, avoids recommends, and bounds network retries and install time.
Failed candidates are retained only
as explicitly non-release diagnostic Actions artifacts.

A follow-up validates every local checksum before the first upload, with a
regression covering an empty payload, missing checksum and a corrupt late file.
Eight Node tests now pass. Frontend 37 tests/build and context smoke also passed.
Local workspace Rust tests (60 passed, four ignored), strict Clippy and rustfmt
passed again. Stripping the real local ELF preserved the checked sections.

## Handoff

Do not describe the standalone EXE as an installer or dependency-free binary.
Do not move v0.1.0 or replace existing published assets.
