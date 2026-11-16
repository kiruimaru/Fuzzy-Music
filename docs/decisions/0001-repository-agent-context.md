# Decision: Versioned repository context for coding agents

## Status

Accepted

## Context

Multiple people and coding agents need to continue the same task without
depending on one chat session or a provider-specific memory feature.

## Decision

Keep durable agent context in versioned Markdown files:

- `AGENTS.md` is the entry point and working contract.
- `docs/project-context.md` is the current project snapshot.
- `docs/architecture/contracts.md` contains cross-module interfaces.
- `tasks/` contains task-specific handoffs.
- `docs/ai-log.md` contains concise session results.
- `scripts/ai-context.sh` prints a compact context bundle for a CLI agent.

Agents may use the bundle as input, but code, tests, and explicit architecture
decisions remain authoritative.

## Consequences

Any supported CLI agent can start from the same repository state. The team
must update the context when decisions or task status change, and stale notes
must be corrected rather than treated as hidden memory.
