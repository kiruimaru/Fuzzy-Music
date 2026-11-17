# Task: Integrate the SoundCloud prototype with current main

## Integration review update (2026-11-17)

PR #12 is merged into main after code review and Linux verification, followed
by API PR #15 and context-test PR #13. PR #1 was already closed when reviewed.
The original source branch remains unchanged. Earlier publication instructions
below are historical. See `tasks/pr-integration-review.md` for current status;
the next implementation task is shell issue #14.

## Goal

Continue PR #1 in a separate reviewable branch while preserving the current
repository foundation and the source author's implementation.

## Scope

- Integrate `origin/feat/soundcloud-request-control` into current `main`.
- Preserve source history, setup scripts, documentation, and request control.
- Exclude Tauri, React, mpv, product expansion, and public API redesign.

## Context

- Rules: `AGENTS.md`; boundary: `docs/architecture/contracts.md`.
- Source: PR #1; base at integration: `2d5ec5a`.
- Original feature branch and PR must remain unchanged and open.

## Plan and Progress

1. Completed: read shared context and inspect source history and PR #1.
2. Completed: create `feat/soundcloud-main-integration` from current main and
   merge the feature without rewriting history.
3. Completed: resolve the README conflict, retaining main's scope/workflow
   and source usage, stack, and team links. Other source files merge unchanged.
4. Completed: inspect implementation and run all required Rust checks.
5. Completed: committed and pushed the new branch; opened
   [PR #12](https://github.com/kiruimaru/Fuzzy-Music/pull/12) continuing #1
   and requested review from `kiruimaru`. Neither PR was merged or closed.

## Verification

- `cargo test --workspace --locked`: pass, 8 passed, 1 ignored fixture.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: pass.
- `cargo fmt --all --check`: pass.
- Live SoundCloud search/resolve, yt-dlp download, Linux, playback, and UI:
  not run in this session. Automated tests do not require SoundCloud access.
- No Rust implementation defect requiring a change was found in this scope.

## Handoff

- Branch: `feat/soundcloud-main-integration`.
- Integration commit: `02628f4`.
- Implementation and original docs/scripts are retained from PR #1.
- Updated README, project context, boundary status, and AI log.
- Risks: source availability can change; raw extractor errors are not UI-safe;
  prototype types differ from the planned DTOs. See the contracts document.
- Next action: second-contributor review of PR #12, then jointly agree
  DTO/error mapping before implementing the application shell. No merge is
  authorized by this task.
