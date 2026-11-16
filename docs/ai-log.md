# AI Work Log

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
