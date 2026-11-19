# AI Work Log

## 2026-11-19 | Agent: Codex | Task: favourites-local-library-review-handoff

### Goal

Prepare the local library and player changes for review above the #22 branch.

### Changed

Included the PNG mascot and SoundCloud search badge, with assets for future
YouTube and VK support. Aligned the library decision and task notes with the
FLAC and theme implementation.

### Checks

53 Rust tests passed (four ignored), 31 frontend tests, eight browser fixtures,
Clippy, rustfmt, frontend build and the agent-context smoke test passed.
The native debug build with the PNG assets opened on Linux.

### Next action

Review this branch above PR #24, coordinate with #21, and verify Windows and a
real download via the GUI before merging.

## 2026-11-19 | Agent: Codex | Task: player-queue-quality-themes

### Goal

Move queue into the player, simplify navigation and add themes and FLAC.

### Changed

Moved the session queue into a player popover; removed duplicate navigation.
Added dark/light persisted theme with schema v1-to-v2 migration, additional
bitrate preferences and ffmpeg-backed FLAC conversion. Existing downloads are
not rewritten; FLAC does not improve lossy source quality.

### Checks

53 Rust tests passed / 4 ignored, strict Clippy/rustfmt, 30 frontend and eight
browser tests passed. Native debug build run on Linux. A real yt-dlp command
produced FLAC verified by ffprobe; the GUI download and Windows remain untested.

### Next action

Restart the debug app and verify queue, theme persistence and FLAC via the UI.

## 2026-11-19 | Agent: Codex | Task: launch-readme

### Goal

Explain how to launch and restart the desktop app for manual testing.

### Changed

Added Russian README instructions for dependencies, Tauri development mode,
standalone debug builds, restarting after changes, WebKit and proxy notes.

### Checks

Commands were checked against package scripts and the existing shell guide;
the native debug build was verified in the preceding session.

### Next action

Continue manual testing with the rebuilt app.

## 2026-11-19 | Agent: Codex | Task: live-volume-adjustment

### Goal

Apply volume while the slider moves instead of waiting for release.

### Changed

Send volume changes immediately, serializing player commands and coalescing
intermediate pending values; keep settings writes ordered.

### Checks

29 frontend tests, TypeScript/Vite build and eight browser fixture tests passed.
Native audible adjustment remains for user verification.

### Next action

Restart the native application and confirm the slider changes sound continuously.

## 2026-11-19 | Agent: Codex | Task: favourites-local-library

### Goal

Implement the requested SoundCloud-style transport and persistent favourites,
download settings and an automatically indexed local music folder.

### Changed

- Rearranged playback controls left, expanded the timeline and added a vertical
  hover/focus volume control. Added favourites, library and settings views.
- Added one versioned SQLite database for favourites, audio index and settings;
  persisted volume replaces the temporary #22 memory port.
- Added one bounded/cancellable SoundCloud download worker, quality preferences,
  disk staging, no-overwrite publication, error/retry, and file-preserving unlike.
- Added startup/manual folder scanning, metadata parsing and local playback by
  opaque index ID. The user explicitly expanded the prior no-download scope.
- Decision 0010 and `tasks/favourites-local-library.md` describe limits and
  overlap with #21. Work is on `feat/favourites-local-library`, based on PR #24.

### Checks

- Locked workspace tests: 52 passed, 4 ignored; strict Clippy/rustfmt passed.
- Frontend: 28 tests and build passed; eight Chromium UI fixtures passed.
- Native Tauri debug build passed; application started and its window appeared
  on Linux. Manual playback and persistence acceptance are still outstanding.
- Real yt-dlp downloaded one public SoundCloud track through the configured
  proxy; ffprobe found AAC ~160 kbit/s. No universal quality promise is implied.
- Screenshots reviewed at 1000px/320px, including the compact transport.

### Decisions

- Best available source format is default; lower preferences fall back without
  transcoding. Unlike never removes the file. Local files are not exposed as paths.
- #21 ownership overlaps; contributor review and coordination remain necessary.

### Open questions

- Human native listening, persistent favourite/download/folder restart scenario,
  Windows runtime/ffmpeg availability and contributor review remain.
- Existing source-playlist cache and HTTP lifecycle issues are not fixed here.

### Next action

Perform the user acceptance scenario and coordinate the dependent branch with
the #21 contributor.

## 2026-11-19 | Agent: Codex | Task: playback-queue-22

### Goal

Implement issue #22 on top of the reviewed playback/playlist integration.

### Changed

- Added an independent session queue with stable entry keys, duplicate support,
  selection/removal, Previous/Next, EOF advancement and no wraparound.
- Stop/search retain the queue; deleting current cancels/stops without choosing
  a successor. Errors wait for manual selection rather than infinite retries.
- Session tracks resolve fresh and drop the previous player before resolution.
  Existing public playlist playback keeps its prepared transition sequence.
- Added the typed #21 settings port and an explicitly temporary memory adapter.
  Preferred volume is passed at player startup, retained on Stop and saved
  through serialized port writes. No second persistence store was introduced.
- Updated contracts/context and decision 0009; task: `playback-queue-22.md`.
- Published implementation `8d91e75` in draft PR #24; Windows alpha CI started.
  Issue #22 remains open. Launched the new native window for user acceptance
  under transient service `fuzzy-music-issue22.service`; window is visible.

### Checks

- Locked workspace/all-target Rust tests: 47 passed, 4 ignored.
- Strict all-target Clippy, rustfmt, context smoke and Bash syntax passed.
- Frontend: 26 tests, TypeScript/Vite build and six Chromium UI tests passed.
  Queue screenshots checked at 1000px and 320px; player remains visible.
- Native Tauri debug no-bundle build passed.
- Real mpv/local WAV with null output passed initial volume, pause/seek/volume/
  resume and EOF/replay. It does not establish human audibility.
- The earlier combined-main Windows workflow 44389261965 completed successfully;
  this is base integration evidence, not Windows verification of #22.

### Decisions

- Current-entry removal stops without auto-selection; queue is session-only.
- Keep #21 storage ownership; proposed volume IPC names await contributor review.

### Open questions

- #21 persistent settings adapter, human listening, Windows and contributor review.
- Earlier source-playlist cache/HTTP lifecycle findings remain separate follow-ups.

### Next action

Check PR #24 Windows CI, perform human native queue acceptance, obtain contributor
review, and connect #21's storage API when available. Keep #22 open pending acceptance.

## 2026-11-19 | Agent: Codex | Task: pr20-pr23-integration

### Goal

Approve and integrate PRs #20/#23 as explicitly requested after manual testing.

### Changed

- Combined both contributor histories and preserved earlier review evidence.
- Resolved shell module/player conflicts, retaining mpv IPC and dependency errors.
- Updated current README, contracts and project context to match implementation.
- Published APPROVED reviews for both original heads; GitHub marks both PRs
  merged. Pushed combined history through `5e47cb8` to main and synced locally.
- Recorded accepted follow-ups in `tasks/pr20-pr23-integration.md`; no known
  cache, worker or HTTP lifecycle finding is represented as fixed.

### Checks

- Combined Linux Rust tests: 44 passed / 4 ignored; strict Clippy/rustfmt passed.
- Frontend: 15 tests, build and four Chromium fixture smoke tests passed.
- Combined native Tauri debug no-bundle build passed.
- Original native run: no observed crash/OOM/journal error; approximately
  537 MiB at 12.5 minutes, 677 MiB peak across the complete service.
- Combined Windows workflow 44389261965 is running against `5e47cb8`;
  clean-machine runtime verification remains pending.

### Decisions

- Owner accepts the reviewed limitations for integration and follow-up.
- Preserve contributor branches and keep MVP/platform issues open.

### Open questions

- Resource limits, HTTP lifecycle, Linux release discovery and runtime guidance.

### Next action

Check the combined Windows workflow result and continue accepted resource/lifecycle
follow-ups before release. Publication details are in the integration task.

## 2026-11-17 | Agent: OpenCode | Task: windows-alpha-download-size

Measured old ZIP entries: 81.46 MB compressed, 556.97 MB unpacked, dominated by
547.40 MB source/vendor. Split portable and matching corresponding-source ZIPs,
separate Actions uploads, per-archive checksums and Git/executable provenance.
Source copy uses tracked inputs, excluding build/runtime output. Added meaningful
package entry/checksum assertions; both passed. Local portable 2.89 MB vs 81.46 MB
(96.5% reduction), same optimized EXE. Fresh runtime setup and live GUI smoke
passed search/Playing/system-proxy/Stop/exit without developer PATH or overrides;
zero new consoles. Runtime downloads and installed bytes documented separately.
No clean-machine or human listening claim; existing WebView2 exit warning remains.

## 2026-11-17 | Agent: OpenCode | Task: windows-alpha-distribution

Preserved alpha/discovery and hidden-console changes. Inspected exact upstream
mpv release/run: binary/debug artifacts but no complete matching transitive source
bundle. Implemented documented digest-verified direct-upstream runtime bootstrap;
distributed ZIP excludes those executables and includes application/locked Rust
sources, frontend runtime sources and npm license notices. Added safe actionable
missing/corrupt tool errors/regressions and PR/manual Windows artifact workflow.
Native release and frontend tests/build passed; corrected Clippy test ordering.
Packaging timed out during initial vendoring, then completed with longer timeout.
Fresh runtime acquisition and live GUI search/Playing/Stop/exit passed, 626 console
samples/337 active extractor samples/zero new consoles. Clean Windows and listening
not claimed. Task file records artifact/provenance and external smoke requirement.
User authorized branch/PR publication under Maroi9p; no release/tag authorized.
Published implementation `57efb24` as PR #20, review requested from kiruimaru;
local author/committer and GitHub API identities both verified Maroi9p. Windows
Actions run 44081260899 passed all checks/build/packaging and uploaded artifact
14327467326 (ZIP+SHA, 14 days). Publication links/size are in the task context.
No release/tag/merge performed.

## 2026-11-17 | Agent: OpenCode | Task: windows-alpha-yt-dlp-console

Applied Windows-only CREATE_NO_WINDOW via the command-group builder in
`crates/soundcloud/src/process.rs`, preserving process-job cleanup and request
control. Extended portable UI smoke with EnumWindows observation. Locked all-target
workspace tests: 24 passed, 2 ignored; fmt and strict Clippy passed. Native release
and existing portable packaging passed (download retried after timeout). Real GUI
returned ten tracks and Playing through native system proxy with packaged mpv,
no dependency/proxy env overrides: 641 window samples, 204 active extractor samples,
zero new visible consoles. Stop/own app cleanup passed; no listening claim and
sampling cannot exclude between-sample flashes. ZIP/hash and remaining existing
alpha limitations recorded in `tasks/windows-alpha.md`. Existing uncommitted alpha
work retained; no commit, push or release.

## 2026-11-17 | Agent: OpenCode | Task: windows-alpha

Created separate feature worktree from merged main faa334a. Added portable
executable-relative dependencies, digest-pinned packaging, bilingual guide and
decision 0005. Rust 24 tests/2 ignored, frontend five tests, frontend/native
release builds and corrected strict Clippy passed. Actual portable UI with
different cwd and Windows-only PATH returned ten results, reached Playing via
system proxy, Stop cleaned owned mpv and own app exited. No listening claim.
Created Linux issue #19 assigned kiruimaru under verified Maroi9p login.
Public readiness remains incomplete: full third-party license/source inventory,
clean Windows/runtime verification and missing-dependency acceptance. See
tasks/windows-alpha.md. No commit/push/PR/release/tag performed.
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

## 2026-11-18 — Playlist preparation retry

- Added guarded, delayed retry when preparing the next playlist stream fails.
- Checks: npm test (10 passed); npm run build passed.
- Gapless transitions remain unimplemented; see tasks/playlist-transitions.md for the next steps and limitations.

## 2026-11-18 — mpv playlist transitions

- Append the resolved next stream to the owned mpv process; enable gapless audio and playlist prefetch and report completed transitions to the UI.
- Fixture and frontend tests cover automatic advancement; real mpv/SoundCloud listening and transition timing remain outstanding. See tasks/playlist-transitions.md.

## 2026-11-18 — Playlist lookahead

- Resolve up to two following playlist tracks in the background, cache temporary links with bounded lifetime, reuse them for manual and automatic selection, and cancel work on replacement/stop.
- Checks: 12 Rust shell tests and 12 frontend tests pass; shell Clippy passes. Live speed and audio buffering remain to be measured.

## 2026-11-18 — Bidirectional playlist lookahead

- Expand temporary stream-link caching to three neighbours in each direction, prioritized by distance, and retain still-valid links for manual navigation.
- Checks: 12 shell tests, 13 frontend tests, and shell Clippy pass; real timing remains to be checked.

## 2026-11-18 — Twenty-track lookahead

- Expanded bounded link lookahead to twenty nearest neighbours; tests cover a large playlist and both directions.
- Tests: 12 shell and 14 frontend passed; shell Clippy passed. A live probe returned HLS; audio-byte caching in RAM remains unimplemented and unverified.

## 2026-11-18 — Parallel lookahead preparation

- Resolve up to four neighbouring stream links concurrently, retaining completed results while cancelling obsolete work.
- Shell tests (13 passed) include concurrent fixture timing; Clippy passes. Audio prebuffering and twenty-track rapid-switch acceptance remain outstanding.

## 2026-11-18 — RAM audio startup cache

- Added bounded in-memory HLS startup segments, loopback delivery to mpv, proxy bypass for loopback, and retained link reuse for repeated selection. Unsupported HLS forms fall back to normal streaming.
- Shell tests include manifest rewriting, cached segment delivery and clearing; real SoundCloud listening and latency still need verification.
- A live SoundCloud HLS stream with an initialization segment was cached and successfully played through mpv with null audio output. Audible performance and rapid switching across a playlist still require user testing.

## 2026-11-18 — Expanded RAM lookahead

- Expanded lookahead to forty neighbours and bounded audio cache to 6 MiB per track (41 tracks maximum); publish the first three segments while later segments are fetched.
- Check real-world memory use and switching after prefetch; allocation depends on playlist and stream size rather than being padded to a target.

## 2026-11-18 — Faster lookahead scheduling

- Use eight continuously scheduled workers and reuse cached links when audio preloading needs another attempt. Check cancellation between segments and retain at most 3 MiB per track.
- Offline coverage includes a slow-track scheduling fixture; live SoundCloud/mpv and user-measured switching remain separate checks.

## 2026-11-18 — Preserve playlist preparation across manual skips

- Keep an active playlist lookahead running across manual track changes; reuse links and audio until the signed expiry safety margin or 15-minute cap. Unknown expiry retains short fallback lifetimes.
- Fixture checks cover in-flight prefetch across a skip, repeated back-and-forth link reuse, and signed expiration; listening and latency on the user's playlist remain to be checked.

## 2026-11-18 — Thirty-minute cache lifetime

- Increased the upper bound for signed links and HLS audio from 15 to 30 minutes, preserving expiry checks and the safety margin.
- Added boundary coverage for both the old and new limits; user confirmed the preceding playback behavior.

## 2026-11-19 — Lookahead resource limits

- Investigated a user report of system-wide lag and FPS drops during long playlist sessions; live probes showed signed links stay valid for about two hours, so the 30-minute cap caused periodic storms of up to 40 extractor launches plus hundreds of segment downloads.
- Raised the cache cap to 55 minutes, dropped expired entries on lookup, and bounded extractor launches to 16 per cycle with leftovers deferred to the next 45-second cycle.
- Limited RAM audio warming to the 3 nearest neighbours; the rest of the 40-track lookahead stores links only, and mpv demuxer caches now have explicit byte caps.
- Updated the freshness boundary test to straddle the new 3300-second cap; decision recorded in `docs/decisions/0008-lookahead-resource-limits.md`.
- Verification: cargo fmt clean; `cargo test --workspace --all-targets --locked` all passed; frontend untouched.

## 2026-11-19 — Playlist bookmarks

- Added bookmarked SoundCloud playlists in SQLite schema v3 (`playlists` and `playlist_tracks`), independent of track favourites and storing no signed URLs or audio.
- Liking a playlist starts no downloads; an explicit Download action resolves the track list through the application boundary and queues tracks through the existing single download worker (queue capacity 1024, 1000-track request cap, duplicates collapsed by source id).
- Playlist rows survive unliking so downloads keep their association; downloaded playlist tracks appear in the music library, and Favourites shows per-playlist progress.
- UI adds hearts on playlist cards and the playlist header, a Favourite playlists section and open/download actions; work is stacked on PR #25 and recorded in `docs/decisions/0011-playlist-favourites.md`.
- Verification: cargo fmt clean; workspace tests 54 passed and 4 ignored; frontend 32 tests; TypeScript/Vite build passed.

## 2026-11-19 — Playlist download cancellation

- A running playlist download can now be cancelled from the Favourite playlists list and the playlist header; pending and in-flight media return to idle while the bookmark, track list and downloaded files stay untouched, so Download can resume later.
- The library service tracks explicitly cancelled media ids so its requeue safety net cannot restart them; the same cancellation token still interrupts the running yt-dlp download.
- Playlist downloads continue to never mark tracks as favourites, and a test asserts cancelled playlist tracks stay out of Favourites.
- Verification: added a fixture-backed cancellation test (in-flight abort, queued ids skipped, no requeue, bookmark kept, no favourites).

## 2026-11-19 — Artist pages and list playback

- Added artist pages: a SoundCloud artist URL resolves to a user id and loads the first 50 public tracks through the web API; non-artist URLs are rejected before network access.
- Artist names in track rows are links; the artist view reuses playlist header styling, keeps search results for Back to search and offers Play artist.
- Playing any row now starts the visible list as the queue, so search results, artist pages, playlists, Favourites and the music library auto-advance through the existing transition machinery; the session queue keeps its semantics.
- New commands `artist_tracks` and `cancel_artists`; new `Artist` DTO; response parsing accepts both bare-array and collection track shapes.
- Verification: workspace tests 58 passed and 4 ignored; frontend 34 tests; clippy clean; Tauri debug build passed.

## 2026-11-19 — Unliking, artist presentation and universal artist links

- Favourite rows now disappear the moment the heart is unclicked: the list updates optimistically and the background refresh reconciles with SQLite, restoring the entry if the write fails. The same applies to bookmarked playlists in Favourites.
- Heart buttons on track rows now announce Remove from favourites when the track is liked instead of a stale Favourite label, and library rows gained the grid they need when the Retry action makes five buttons.
- The artist page no longer borrows playlist chrome: it has its own banner with a blurred avatar backdrop, a circular portrait, name, track counts and Play artist, followed by a titled track list.
- Artist names link to the artist page in all remaining surfaces: Favourites and music-library rows, favourite playlist rows, playlist cards, the playlist header, the session queue and the now-playing panel. Playlist authors are derived from the playlist URL's first path segment; names without a SoundCloud page stay plain text.
- Playlist cards were restructured so the author link is a real button rather than a nested one inside the open-card button.
- Verification: frontend 35 tests including immediate-unlike and playlist-author-navigation cases; TypeScript/Vite build passed; Tauri debug build passed.


