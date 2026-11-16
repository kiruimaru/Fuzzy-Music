# Fuzzy Music Agent Guide

This file is the entry point for every coding agent working in this
repository. Read it before inspecting or changing implementation code.

## Start here

Read these files in order:

1. `README.md` for the product boundary and development workflow.
2. `docs/project-context.md` for the current state, decisions, risks, and
   next actions.
3. `docs/architecture/contracts.md` before changing the Tauri/Rust/frontend
   boundary.
4. The active task context under `tasks/`, if one exists.
5. The latest relevant entries in `docs/ai-log.md`.

To give a CLI agent the same compact starting context, run:

```bash
./scripts/ai-context.sh
./scripts/ai-context.sh tasks/<short-task-name>.md
```

The output can be supplied to any CLI agent that accepts stdin or an initial
prompt. The command does not call a provider or transmit repository data.

If these documents disagree with the code or tests, trust executable code and
tests first, then update the stale documentation in the same change.

## Working rules

- Keep the first release limited to search, metadata, streaming playback,
  queue, favourites, and settings.
- Keep SoundCloud-specific code behind a Rust module boundary. The UI must not
  depend on SoundCloud response shapes.
- Use explicit typed contracts at the Tauri command boundary.
- Preserve request timeout and cancellation behavior.
- Do not commit API keys, cookies, access tokens, personal paths, or raw
  private chat transcripts.
- Do not paste raw CLI output into `docs/ai-log.md` when a short summary and
  command result are sufficient.
- Record architectural choices in `docs/decisions/` when they affect another
  contributor or agent.
- Prefer small, reviewable Pull Requests. Avoid unrelated formatting or
  generated-file churn.

## Session handoff

Before ending a non-trivial session:

1. Update the relevant task context with what changed and what remains.
2. Run the checks that are available for the changed area.
3. Add one entry to `docs/ai-log.md` using the template in that file.
4. State blockers as concrete commands, errors, or missing decisions.

The next agent should be able to continue from the repository without needing
the previous conversation.

## Suggested task context

Create `tasks/<short-task-name>.md` from
[`tasks/_template.md`](tasks/_template.md). A task context is temporary
working memory; source code, tests, and durable architecture decisions remain
the source of truth.
