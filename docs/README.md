# WorkMesh Docs

Canonical workflow entrypoint:
- [`docs/getting-started.md`](getting-started.md)

Command catalog:
- [`docs/reference/commands.md`](reference/commands.md)

## Documentation structure
- [`docs/getting-started.md`](getting-started.md): progressive DX runbook (start -> parallelize -> recover -> consolidate clones).
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
- Primary workflow guidance lives in one place: [`docs/getting-started.md`](getting-started.md).
- [`README.md`](../README.md) and [`README.json`](../README.json) stay synchronized for humans and agents.
- Legacy migration guidance remains intentionally minimal and out of the main DX flow.
