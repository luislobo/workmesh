# WorkMesh Docs

Canonical workflow entrypoint:
- [`docs/getting-started.md`](getting-started.md)

Command catalog:
- [`docs/reference/commands.md`](reference/commands.md)

## Documentation structure
- [`docs/getting-started.md`](getting-started.md): Codex-first onboarding and daily workflow.
- [`docs/setup/run-modes-and-agent-mcp.md`](setup/run-modes-and-agent-mcp.md): run/install/configure for CLI, MCP stdio, MCP HTTP, and agent wiring (CLI/GUI).
- [`docs/reference/commands.md`](reference/commands.md): authoritative CLI/MCP command surface.
- `docs/projects/<project-id>/`: project-level PRDs, decisions, and updates.
- [`docs/diagrams/`](diagrams/): architecture and workflow diagrams.
- [`docker/workmesh-service/`](../docker/workmesh-service/): sample container deployment for `workmesh-service`.

## Core concepts
- Tasks: `workmesh/tasks/` (or `.workmesh/tasks/`) markdown task files.
- Context: `workmesh/context.json` repo-local intent/scope pointer.
- Truth: `workmesh/truth/` durable decision records.
- Sessions: global continuity records under `WORKMESH_HOME`.
- Worktrees: runtime stream isolation for parallel work.
- Service mode: `workmesh-service` HTTP runtime for local/LAN operations.

## Policy
- Primary guidance is prompt-driven and Codex-first.
- [`README.md`](../README.md) and [`README.json`](../README.json) stay synchronized for humans and agents.
- Legacy migration guidance remains minimal and out of the main flow.
- Archive defaults are safety-first: only terminal statuses are archived unless an explicit status override is provided.
- Task lifecycle quality is enforced: `Done` transitions require complete task sections and outcome-based Definition of Done criteria.

## Storage Integrity Policy
- Tracking-file writes must use WorkMesh storage primitives (no ad-hoc direct writes for critical state).
- Critical mutable snapshots are versioned and CAS-protected.
- JSONL event streams must be append-safe and recoverable via doctor fix-storage pathways.
- Doctor diagnostics are the canonical integrity signal for locks, JSONL health, snapshot versioning, and truth projection consistency.
