# AI Work Log

## 2026-11-19 | Agent: Codex | Task: pr23-manual-test-launch

### Goal

Open the reviewed PR #23 for the user's manual playback and playlist testing.

### Changed

- Built the original PR head in a separate snapshot and launched a standalone
  native window as transient user service `fuzzy-music-pr23.service`.
- Enabled service memory accounting and process-local rendering/proxy settings.
- Recorded launch evidence in `tasks/pr-review-2026-11-19.md`.

### Checks

- Native debug no-bundle build: passed.
- Direct SoundCloud HTTP check: timed out; existing local proxy: HTTP 200.
- Service running and native window mapped/visible: confirmed.

### Decisions

- Test the original reviewed implementation; no review fixes applied.

### Open questions

- User playback results and longer-session memory behaviour remain pending.

### Next action

Collect the user's observations and inspect service memory/processes after testing.

## 2026-11-19 | Agent: Codex | Task: pr-review-2026-11-19

### Goal

Update local main and review all open PRs requesting owner review, focusing on
the reported playlist memory problem.

### Changed

- Fast-forwarded main to `faa334a`; reviewed #23 `cfa6c69` and #20 `82bafc8`.
- Recorded findings and generated diagnostic patches in
  `tasks/pr-review-2026-11-19.md` and adjacent regression patches.
- Updated project context to reflect merged #17/#18 and remaining PR blockers.
- No implementation changes, commits, external messages, PR approvals or merges.

### Checks

- Original PR #23: 40 Rust tests passed / 4 ignored, 15 frontend tests,
  four Chromium fixture tests, strict Clippy, rustfmt and frontend build passed.
- Original PR #20: 27 Rust tests passed / 2 ignored, five frontend tests,
  two Chromium fixture tests, strict Clippy, rustfmt and frontend build passed.
- PR #23 real mpv with local WAV and null output: controls and EOF replay passed.
- Four added #23 regressions failed: retained audio bound, manual warm
  concurrency, HTTP cancellation latency and shared operation deadline.
- Added #20 UI regression and Linux release-discovery harness failed.
- Both generated patches pass apply checks on their original heads; repository
  context smoke, Bash syntax and local diff whitespace checks passed.
- Windows packaging/GUI, live SoundCloud and whole-system memory: not run.

### Decisions

- Both PRs need changes. Cache growth is bounded at 123 MiB of retained audio,
  but the stated four-track budget is unenforced and manual warm concurrency
  is unbounded. No unlimited retained-cache leak or system OOM was established.

### Open questions

- Windows clean-machine acceptance and actual HLS timing/memory remain unverified.

### Next action

Fix #23 resource/lifecycle defects and #20 Linux/runtime-guidance regressions;
use the saved patches to verify fixes before requesting approval.

## 2026-11-17 | Agent: OpenCode | Task: windows-network-integration

Implemented immutable native Windows manual proxy detection and shared explicit
child environment for yt-dlp/mpv over PR #17 in a separate local feature branch.
Decision 0004 documents env precedence, startup refresh, bypass and PAC limits.
23 Rust tests, strict Clippy/fmt, frontend five tests, two Chrome fixture tests,
frontend/native debug builds passed. Native EXE with proxy env stripped detected
current Windows manual proxy, returned ten tracks, freshly resolved and reached
Playing; owned mpv connected to the local proxy, Stop removed it, close exited.
No new listening claim; previous user sound confirmation is retained in context.
The user subsequently confirmed working search and authorized separate PR publication
under Maroi9p. No new-build listening confirmation is claimed. No global settings
changes. Published as https://github.com/kiruimaru/Fuzzy-Music/pull/18, stacked
on #17; review requested from kiruimaru. GitHub confirmed implementation author
and committer as Maroi9p. Publication details are tracked in `tasks/windows-network-integration.md`.

This is the concise handoff log for coding-agent sessions. It exists so a
different agent can reconstruct the current work quickly from the repository.
It is not a transcript.

## Rules

- Add one entry after each meaningful session or milestone.
- Use ISO dates and identify the agent/tool when known.
- Link to task files, decisions, and code rather than copying large output.
- Record commands and short results, especially failed checks.
- Never include secrets, tokens, cookies, private prompts, or personal machine
  paths.
- Keep entries factual. Put unresolved questions under `Open questions`.
- Use `./scripts/ai-context.sh` to prepare the shared context before starting
  a new CLI-agent session.

## Entry template

Copy this block for a new entry and place newest entries at the top.

```md
## YYYY-MM-DD | Agent: <name> | Task: <task-id>

### Goal

<one sentence>

### Changed

- `<path>`: <short description>

### Checks

- `<command>`: `<pass/fail/not run>` - <short result>

### Decisions

- <decision or `none`>

### Open questions

- <question or `none`>

### Next action

<the next concrete step>
```

## 2026-11-17 | Agent: Codex | Task: tauri-shell-14

### Goal

Implement the minimal native search -> resolve -> mpv -> Stop slice in #14.

### Changed

- Added Tauri 2/React TypeScript shell, typed commands, blocking workers, separate
  generations/cancellation, fresh resolve, owned mpv groups and exit cleanup.
- Added process/race tests, frontend regressions, browser fixture smoke and
  an opt-in live playback test; source adapter code/API unchanged.
- Updated contracts, README/context, decision 0003, shell setup and task handoffs.

### Checks

- Locked workspace tests: pass, 20 passed, 2 ignored. Strict Clippy/rustfmt: pass.
- Frontend: 5 tests and 2 browser smoke tests pass; TypeScript/build pass.
- Full native Tauri debug build, context smoke and Bash syntax: pass.
- Direct live source: network timeout. Existing proxy: live search and two
  resolve/play/Stop cycles passed. Real window returned ten tracks, reached
  Playing with an active mpv audio-device stream; Stop removed its mpv child.
- Closing the real window during replay removed both app and mpv. Windows
  no-console flags are set on the command-group builder, which owns spawn flags.
- Native rendering needed the documented DMABUF workaround. Windows and human
  listening checks were unavailable; fixture tests are not live audio proof.

### Decisions

- Decision 0003 uses merged DTOs, shell-local request state and headless mpv;
  startup readiness is a fixed marker, with exit/deadline error handling.

### Open questions

- Joint human contract review, Windows validation and listening confirmation
  remain before claiming complete issue acceptance.

### Next action

Review `tasks/tauri-shell-14.md` and run target Windows/manual audio checks.
No issue closure or external messages performed.

## 2026-11-17 | Agent: Codex | Task: pr-integration-review

### Goal

Review branch changes, merge ready PRs, and assess repository structure.

### Changed

- Merged #12, retargeted and merged #15, then merged #13 after preserving
  both sets of journal entries in its only conflict.
- Reviewed all remote branch histories; old source branches need no extra merge.
- Updated README layout, project context, contract status, and task handoffs.
- Recorded review scope and remaining work in `tasks/pr-integration-review.md`.

### Checks

- Linux all-targets locked Rust tests: pass, 14 passed, one child fixture ignored.
- Strict offline Clippy, rustfmt, Bash syntax, and context smoke test: pass.
- Diff whitespace checks: pass.
- Live SoundCloud, Tauri, and audio: not run. No CI workflows exist.

### Decisions

- Keep existing module boundaries and preserve branch history.
- Use the merged application API for the next shell task; no structure rewrite.

### Open questions

- CI matrix, packaging, and playback integration remain to be implemented.

### Next action

Continue shell issue #14 using `tasks/application-api.md`; validate live source
and audio separately from offline tests.

## 2026-11-17 | Agent: OpenCode | Task: #11

### Goal

Verify the AI context shipped in main without publishing unrelated local work.

### Changed

- Added `scripts/test-ai-context.sh` with disposable offline fixtures.
- Recorded coverage and assignment blockers in `tasks/ai-context-11.md`.

### Checks

- `bash scripts/test-ai-context.sh`: pass on Windows Git Bash.
- `bash -n scripts/ai-context.sh scripts/test-ai-context.sh`: pass.
- Preserved application workspace: offline all-targets tests pass (14 passed,
  one ignored fixture); rustfmt and strict offline Clippy pass. These checks
  concern the separate local worktree, not the main-branch PR contents.
- No AI provider, live source, playback or Linux execution claimed.

### Decisions

- Use an isolated worktree from main; preserve uncommitted application API work.

### Open questions

- #9 needs a third closed PR and real meeting agreements; #7 needs scope criteria.

### Next action

Publish #11 verification for kiruimaru review without merging.

## 2026-11-17 | Agent: OpenCode | Task: application-api-publication

### Goal

Publish the application boundary and actionable handoff for issue #14.

### Changed

- Reviewed DTO serialization, duration extremes, safe errors, blocking usage,
  cancellation and source mapping; added fractional/negative/oversized input
  serialization checks and rounding edge assertions. No source API change.
- Updated task, contracts, decision and project context with shell owner steps.
- Selected stacked base `feat/soundcloud-main-integration`: PR #12 remains open;
  PR #13 is independent and its separate worktree is untouched.

### Checks

- `cargo fmt --all --check`: pass.
- `cargo test --workspace --all-targets --locked`: pass, 14 passed, 1 ignored.
- `cargo test --workspace --all-targets --locked --offline`: same result.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: pass.
- `cargo check -p fuzzy-application --example search --locked`: pass.
- `git diff --check`: pass; only local line-ending warnings.
- Live SoundCloud, Linux, Tauri and audio: not run.

### Decisions

- Publish with contributor review, no merging or history rewriting. Merge #12
  first, retarget API to main and recheck; shell implementation follows separately.

### Open questions

- Both owners must review contract policies; shell IPC, mpv and packaging remain.

### Next action

Published [API PR #15](https://github.com/kiruimaru/Fuzzy-Music/pull/15) with
review requested from `kiruimaru`; updated only the API dependency paragraph
in #14, preserving shell requirements. API commit: `0ff65fe`. Await #12 review
and merge, then retarget #15 to main and recheck before API merge. Nothing was
merged or closed; no API approval or live playback is claimed.

## 2026-11-17 | Agent: OpenCode | Task: application-api

### Goal

Prepare a typed, runtime-independent boundary for future Tauri integration.

### Changed

- `crates/application`: search/resolve DTOs, metadata mapping, safe AppError,
  explicit request options, offline unit and extractor-to-application tests.
- Added a compilable worker example without pretending to supply a Tauri bridge.
- Updated README, contracts, project context and `tasks/application-api.md`.
- Decision 0002 records new policies, including sourceUrl and separate temporary
  stream resolution; these require contributor review, not prior agreement.
- Created local `feat/application-api` from clean integration HEAD 12c21c1.
  GitHub PR #12 remains open. No commit, push, PR or merge performed.

### Checks

- `cargo test --workspace --all-targets --locked`: pass, 14 passed, 1 ignored.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: pass.
- `cargo fmt --all --check`: pass.
- `cargo check -p fuzzy-application --example search --locked`: pass.
- `git diff --check`: pass (Git emits local line-ending conversion warnings).
- Live source, Linux, Tauri and playback: not run.

### Decisions

- Preserve internal SoundCloud API and control behavior; no runtime dependencies.
- Keep stale-response generations local to shell owners; document blocking-worker
  dispatch and per-operation cancellation instead of adding global state.

### Open questions

- Contributor review of boundary policies and future shell IPC/packaging details.

### Next action

Review local changes; connect actual shell worker dispatch, cancellation IPC and
generation guards separately. Publishing requires new explicit permission.

## 2026-11-17 | Agent: OpenCode | Task: soundcloud-main-integration

### Goal

Integrate the SoundCloud prototype with current main and deliver a separate PR
continuing #1 without modifying the original branch.

### Changed

- Merged the feature history into `feat/soundcloud-main-integration` from main.
- Resolved the README conflict, preserving foundation guidance and probe usage.
- Preserved Rust implementation, setup scripts, SoundCloud guide, and team docs.
- Updated project context and documented prototype/DTO gaps in the contracts.
- Added `tasks/soundcloud-main-integration.md` with verification and handoff.

### Checks

- `cargo test --workspace --locked`: pass - 8 passed, 1 ignored child fixture.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: pass.
- `cargo fmt --all --check`: pass.
- Live SoundCloud requests, tool download, Linux, playback, UI: not run.

### Decisions

- Preserve source history and public API; no Rust fixes were needed for this
  integration. Use a new PR continuing #1; leave #1 open and do not merge main.
- Defer DTO mapping and safe error translation to agreement between owners.

### Open questions

- Missing author, duration conversion, pagination, stream mapping, and safe
  errors must be agreed before connecting the adapter to Tauri.

### Next action

Branch pushed; [PR #12](https://github.com/kiruimaru/Fuzzy-Music/pull/12)
created with review requested from `kiruimaru`. Integration commit: `02628f4`.
Await review, then agree boundary mapping as the next implementation stage.
PR #1 remains open; main was not changed.

## 2026-11-17 | Agent: Codex | Task: agent-context-and-ai-log

### Goal

Create a shared repository context that allows multiple coding agents to
continue work without relying on one chat session.

### Changed

- Added `AGENTS.md` as the entry point for agent sessions.
- Added `docs/project-context.md` with the product scope, architecture, status,
  risks, and delivery order.
- Added `docs/architecture/contracts.md` for the frontend/Rust boundary.
- Added `tasks/_template.md` for task handoffs.
- Added this log and updated `README.md`.

### Checks

- Repository inspection: pass - repository contains only the initial README,
  license, and git metadata before this change.
- `gh auth status`: not ready - the configured GitHub token is invalid.
- Branch creation: blocked by local git ref permissions; no branch was created.

### Decisions

- Store durable context in versioned Markdown files.
- Keep AI logs concise and exclude raw transcripts and secrets.
- Treat the code and tests as the source of truth when notes become stale.

### Open questions

- Which AI CLI tools should be supported by project-specific launch scripts?
- How will yt-dlp and mpv be packaged for Windows and Linux?
- What is the final Rust-to-mpv integration API?

### Next action

Create a task branch, commit this foundation, open a PR, and then implement
the minimal Tauri shell.
