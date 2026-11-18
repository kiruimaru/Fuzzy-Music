# Task: Integrate reviewed PRs #20 and #23

## Goal and authorization

The repository owner authorized approving both reviewed PRs and integrating
their histories into main after trying the PR #23 native application.
Known review findings are accepted for follow-up rather than reported as fixed.
This supersedes the earlier recommendation to defer integration; it does not
establish release readiness or close MVP issues.

## Implementation

- Preserve PR #20 head `82bafc8` and PR #23 head `cfa6c69` in merge history.
- Resolve shell module conflict by keeping both audio cache and dependencies.
- Preserve the PR #23 IPC player and the PR #20 safe dependency spawn error.
- Preserve both contributors' journal histories and the local review evidence.
- Update current product/context documentation to match the integrated code.
- Publish approvals against the reviewed commit IDs, then push the combined
  main history without rewriting or deleting contributor branches.

## Publication outcome

- Both PRs received APPROVED reviews against their original reviewed heads.
- PR #20 merged as `1d9cd8e`; PR #23 merged as `ea2f2d5`.
- Combined implementation and review handoff pushed to main as `5e47cb8`;
  the primary checkout was fast-forwarded to the same commit.
- Contributor branches were preserved; no open PRs remained after integration.
- Combined Windows workflow was dispatched for `5e47cb8`:
  https://github.com/kiruimaru/Fuzzy-Music/actions/runs/44389261965
  It was running Rust tests at the final status check; no Windows success
  result or clean-machine runtime acceptance is claimed.

## Verification

- Combined locked workspace/all-target Rust tests: 44 passed, 4 ignored.
- Strict all-target Clippy and rustfmt: passed.
- Frontend: 15 tests, TypeScript/Vite build, four Chromium fixture tests passed.
- Combined native Tauri debug no-bundle build: passed.
- Original PR #23: real local mpv controls/replay passed; manual native test
  had no observed crash/OOM/journal error. Service memory at 12.5 minutes was
  approximately 537 MiB, with a 677 MiB peak and 269 MiB startup baseline.
- Memory measurements cover the app, WebKit, mpv and other owned children,
  rather than only audio-cache payload. No continuous time series was captured.
- Windows CI for the original #20 head passed; the combined Windows workflow
  and clean-machine GUI/runtime verification remain separate acceptance work.

## Accepted follow-ups

1. Enforce a global current-plus-three audio budget; old neighbourhoods currently
   accumulate up to 41 tracks / 123 MiB of retained segment payload.
2. Bound, deduplicate and cancel manual audio warming; rapid Play bypasses the
   eight-worker lookahead pool and shutdown does not own all warm workers.
3. Make in-flight HTTP search cancellation responsive and test production search.
4. Apply one deadline across discovery, retries and playlist metadata requests.
5. Restore Linux release PATH discovery or provide the portable runtime migration.
6. Expose a safe actionable missing-extractor repair instruction in the UI.

Details and diagnostic patches: `pr-review-2026-11-19.md`. They remain failing
counterexamples on the original heads and are not enabled as passing tests.

## Next work

Prioritize resource limits/lifecycle and Linux runtime discovery (issue #19).
Continue remaining queue work under #22; coordinate favourites/settings with
the #21 owner. Do not close #19/#21/#22 or publish a release from this integration.
