# WorkMesh Docs

Read in this order:
1. [`README.md`](../README.md): high-level product and developer entrypoint
2. [`docs/architecture.md`](architecture.md): crate boundaries, runtime flow, and state topology
3. [`docs/getting-started.md`](getting-started.md): day-to-day workflow
4. [`docs/reference/commands.md`](reference/commands.md): exact command surface

## Documentation structure
- [`docs/architecture.md`](architecture.md): application architecture diagrams and contributor routing guidance.
- [`docs/getting-started.md`](getting-started.md): Codex-first onboarding and daily workflow.
- [`docs/setup/run-modes-and-agent-mcp.md`](setup/run-modes-and-agent-mcp.md): install and agent wiring for CLI and MCP stdio.
- [`docs/reference/commands.md`](reference/commands.md): authoritative CLI/MCP command surface.
- `docs/projects/<project-id>/`: project-level PRDs, decisions, and updates.
- [`docs/diagrams/`](diagrams/): architecture and workflow diagrams.
- [`docs/samples/workmesh-demo/README.md`](samples/workmesh-demo/README.md): sample project demonstrating WorkMesh capabilities.

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
- Task lifecycle quality is enforced: `Done` transitions require complete task sections and outcome-based Definition of Done criteria.

## Storage Integrity Policy
- Tracking-file writes must use WorkMesh storage primitives (no ad-hoc direct writes for critical state).
- Critical mutable snapshots are versioned and CAS-protected.
- JSONL event streams must be append-safe and recoverable via doctor fix-storage pathways.
- Doctor diagnostics are the canonical integrity signal for locks, JSONL health, snapshot versioning, and truth projection consistency.
