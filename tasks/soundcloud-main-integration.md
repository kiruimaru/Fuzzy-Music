# Task: Integrate the SoundCloud prototype with current main

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
5. Delivery: commit, push the new branch, and open a PR continuing #1;
   do not merge main or close the original PR.

## Verification

- `cargo test --workspace --locked`: pass, 8 passed, 1 ignored fixture.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: pass.
- `cargo fmt --all --check`: pass.
- Live SoundCloud search/resolve, yt-dlp download, Linux, playback, and UI:
  not run in this session. Automated tests do not require SoundCloud access.
- No Rust implementation defect requiring a change was found in this scope.

## Handoff

- Branch: `feat/soundcloud-main-integration`.
- Implementation and original docs/scripts are retained from PR #1.
- Updated README, project context, boundary status, and AI log.
- Risks: source availability can change; raw extractor errors are not UI-safe;
  prototype types differ from the planned DTOs. See the contracts document.
- Next action after PR creation: second-contributor review, then jointly agree
  DTO/error mapping before implementing the application shell. No merge is
  authorized by this task.
