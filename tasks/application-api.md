# Task: Typed application API

## Goal and scope

Prepare a minimal runtime-independent search/resolve boundary for later Tauri
integration and publish the API with a handoff for issue #14. No UI, mpv or
runtime dependency; no merging existing PRs or closing issues.

## Plan and progress

1. Complete: inspect AGENTS, README, project context, contracts, prior task/log,
   source implementation; confirm clean branch at 12c21c1 and PR #12 still open.
2. Complete: create local `feat/application-api` from integration branch.
3. Complete: implement DTOs, mapping, safe errors and explicit request options.
4. Complete: add offline mapping/serialization/error/control tests and a
   compilable worker example; document new decisions and shell responsibilities.
5. Complete: formatting, locked all-targets workspace tests (14 passed, one
   ignored child fixture), strict all-targets Clippy, example check and diff
   whitespace check passed. See AI log for exact commands.

## Handoff

Implementation: `crates/application`; specification: contracts and decision 0002.
Internal SoundCloud API unchanged. sourceUrl is a deliberate contract extension.
No ready Tauri bridge: actual IPC cancellation, worker dispatch and generation
guards belong to the shell. Review with second contributor before integration.
Offline tests cover application mapping/errors/options and existing adapter
process timeout/cancellation. Live source requests, Linux and UI are not checked.
The offline end-to-end test compiles a tiny extractor with rustc, then exercises
search, resolve, failure, timeout and cancellation through the real adapter.
No todowrite tool was available; this task file tracks the plan instead.

## Publication and shell owner handoff

PR #12 is OPEN at publication: this branch is stacked on
`feat/soundcloud-main-integration` (12c21c1). Review and merge #12 first, then
retarget the API PR to `main` and recheck its diff and checks. Do not merge the
API into the temporary source branch. PR #13 is independent, not a prerequisite.
The published API PR is linked from [issue #14](https://github.com/kiruimaru/Fuzzy-Music/issues/14).

For the shell owner:

1. Review `docs/architecture/contracts.md` and decision 0002 with source owner.
2. Use `fuzzy-application` and `Application::new(SoundCloudClient::new(path))`;
   add path dependencies for application and soundcloud in the future shell.
   Tool discovery/distribution is still shell setup work, not done by this API.
3. Run search/resolve in Tauri's blocking worker pool, await the result, and map
   worker panics to a fixed safe internal error. Never join on the UI thread.
4. Retain a fresh cancellation token per operation and expose cancellation IPC.
   Keep separate search and playback generations; invalidate playback on Stop,
   cancel pending resolve, and ignore its late result before starting mpv.
   Closing the window cancels requests and stops owned playback/processes.
5. Pass returned `sourceUrl` unchanged to fresh resolve on every Play; hand only
   the temporary response `streamUrl` to Rust-owned mpv. Do not persist it.
6. Add responsive loading/empty/error/cancel states and verify the real
   SoundCloud-to-audio scenario separately. Offline fixtures do not prove sound.

Compilable worker example (no Tauri bridge):
`cargo check -p fuzzy-application --example search --locked`.
Optional live CLI invocation:
`cargo run -p fuzzy-application --example search -- <yt-dlp-executable>`.
It only prints the result count, does not play audio, and requires a working
extractor/network. See `docs/soundcloud-development.md` for extractor setup.
