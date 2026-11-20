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

Local: eight Node tests across both release suites, Linux wrapper tests (arguments/spaces, environment cleanup,
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
Run `44714676785` hit the eight-minute APT limit: the official runner source
uses mirror+file indirection, so replacing only direct URLs was insufficient.
Commit `a7b6044` adds an indirect/direct APT rewrite with regression fixtures.
Workflow `44716472401` passed all jobs and published the Linux AppImage
(84,412,920 bytes), packaging source, provenance and SHA256 sidecars. All Linux
public checksums passed; downloaded packaging scripts match the repository.
The AppImage opened as a mapped window on the Arch desktop and remains running
in `fuzzy-music-appimage-test.service`. This verifies startup only, not search,
listening or long-session stability. Windows assets remain verified and unchanged.
Failed candidates are retained only
as explicitly non-release diagnostic Actions artifacts.

A follow-up validates every local checksum before the first upload, with a
regression covering an empty payload, missing checksum and a corrupt late file.
Eight Node tests across both release suites pass. Frontend 37 tests/build and context smoke also passed.
Local workspace Rust tests (60 passed, four ignored), strict Clippy and rustfmt
passed again. Stripping the real local ELF preserved the checked sections.

## Handoff

Do not describe the standalone EXE as an installer or dependency-free binary.
Do not move v0.1.0 or replace existing published assets.
PR #31 merged into main at `8d05a69`; no open PRs remain. Merged local and remote
topic branches were removed (14 remote, 10 local), preserving main and release
tags. Windows CI run `44716481312` passed. Final packaging regressions, wrapper
and APT fixtures, context smoke and whitespace checks passed again. Clean Windows
acceptance, human listening and broader Linux compatibility remain open.
