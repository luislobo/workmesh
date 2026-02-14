# WorkMesh Docs

These docs explain how to use WorkMesh, both as a human tool and as an agent-facing system.

Start here: `docs/getting-started.md`

## Structure
- `docs/getting-started.md` - the guided path to install + quickstart + daily loop.
- `docs/reference/commands.md` - CLI + MCP command reference (names, intent, examples).
- `docs/projects/<project-id>/` - project-level docs.
  - `prds/` - product requirement documents.
  - `decisions/` - ADRs and decision logs.
  - `updates/` - status updates (date-stamped).
  - `comments/` - comment history (append-only).
  - `events/` - normalized change events (append-only).
- `docs/test-coverage.md` - how we measure and enforce test coverage.

## Concepts
- Tasks: `workmesh/tasks/` (or `.workmesh/tasks/`) Markdown files with front matter.
- Context: `workmesh/context.json` (repo-local scope pointer for humans + agents).
- Truth: `workmesh/truth/` (append-only events + current projection for durable decisions).
- Sessions: cross-repo continuity and resume scripts.
- Index: JSONL index under `workmesh/.index/` (derived, rebuildable).
- Graph: relationships + dependencies export for analysis.

## Reference
- CLI: see `README.md` for the canonical command list and examples.
- Agent docs: `README.json` (kept in sync with `README.md`) and MCP tool `readme`.
