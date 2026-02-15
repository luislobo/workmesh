# WorkMesh Docs

Canonical workflow entrypoint:
- [`docs/getting-started.md`](getting-started.md)

Command catalog:
- [`docs/reference/commands.md`](reference/commands.md)

## Documentation structure
- [`docs/getting-started.md`](getting-started.md): Codex-first onboarding and daily workflow.
- [`docs/reference/commands.md`](reference/commands.md): authoritative CLI/MCP command surface.
- `docs/projects/<project-id>/`: project-level PRDs, decisions, and updates.
- [`docs/diagrams/`](diagrams/): architecture and workflow diagrams.

## Core concepts
- Tasks: `workmesh/tasks/` (or `.workmesh/tasks/`) markdown task files.
- Context: `workmesh/context.json` repo-local intent/scope pointer.
- Truth: `workmesh/truth/` durable decision records.
- Sessions: global continuity records under `WORKMESH_HOME`.
- Worktrees: runtime stream isolation for parallel work.

## Policy
- Primary guidance is prompt-driven and Codex-first.
- [`README.md`](../README.md) and [`README.json`](../README.json) stay synchronized for humans and agents.
- Legacy migration guidance remains minimal and out of the main flow.
- Archive defaults are safety-first: only terminal statuses are archived unless an explicit status override is provided.
