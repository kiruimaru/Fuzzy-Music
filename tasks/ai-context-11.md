# Task: Verify AI context (#11)

## Integration review update (2026-11-17)

PR #13 is merged into main after Linux execution of the smoke test and Bash
syntax checks. Main was merged into this branch without rewriting history;
the AI log conflict was resolved by retaining both branches' entries.
The publication-only instructions below describe the earlier task.
See `tasks/pr-integration-review.md` for the combined review.

## Scope and progress

- Read AGENTS, README, project context, contracts, decision 0001 and AI log.
- Test the actual main-branch context independently of uncommitted application API work.
- Add offline regression coverage in `scripts/test-ai-context.sh`.
- Verify default bundle, task paths, working-directory independence, spaces,
  fail-fast missing inputs, and optional AI log.
- Publish a focused PR against main and request kiruimaru review; do not merge.

## Verification

Run `bash scripts/test-ai-context.sh` and `bash -n scripts/ai-context.sh scripts/test-ai-context.sh`.
The test uses disposable local fixtures and does not call an AI provider.
This verifies context assembly, not model comprehension or Linux execution.

## Other assignments

- #9: team/README material is pending in PR #12; only two closed PRs exist
  (#1 and #10), not the required three. Meeting agreements are not documented.
- #7: no acceptance criteria; visual design overlaps the shell owner's scope.
- Local `feat/application-api` work is preserved and excluded from this PR.
