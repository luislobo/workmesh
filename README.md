# WorkMesh

WorkMesh is a MCP-first project and task system that keeps work in plain text while syncing with
Jira, Trello, and GitHub. It is designed for developers who want a clean DX: talk to the agent,
pull updates, resolve conflicts, and keep the full history of project management in the repo.

This repository is the Rust rewrite and evolution of xh-tasks.

## Features
- CLI for list/next/show/stats/export, plus task mutation (status, fields, labels, deps, notes).
- MCP server with parity tools and rootless resolution (infer backlog from CWD).
- Markdown task format with tolerant front-matter parsing.
- Backlog discovery supports `tasks/`, `backlog/tasks/`, or `project/tasks/`.
- Gantt output (PlantUML text/file/svg) with dependency links.
- Graph export command (property-graph JSON for nodes + edges).
- JSONL task index with rebuild/refresh/verify for fast queries.
- Sync engine scaffold with adapter interface + stub adapter.
- Docs-first project model under `docs/projects/<project-id>/`.
- Project scaffolding via `project-init` (CLI) / `project_init` (MCP).
- Validation for required fields, missing dependencies, and missing project docs.
- JSON output for CLI/MCP for easy automation.

## Graph export (JSON schema)
CLI: `workmesh --root <path> graph-export --pretty`

Output shape:
```json
{
  "nodes": [
    {
      "id": "task-012",
      "node_type": "task",
      "title": "Ready work query",
      "status": "To Do",
      "priority": "P2",
      "phase": "Phase3",
      "project": null,
      "initiative": null
    }
  ],
  "edges": [
    { "from": "task-012", "to": "task-011", "edge_type": "blocked_by" }
  ]
}
```

## Index (JSONL)
CLI:
```bash
workmesh --root <path> index-rebuild
workmesh --root <path> index-refresh
workmesh --root <path> index-verify
```

Index file: `backlog/.index/tasks.jsonl`

## Repo layout
- `docs/` - project documentation, PRDs, decisions, and updates.
- `backlog/tasks/` - Markdown tasks managed by the CLI/MCP tools.
- `crates/` - Rust crates (CLI, core, MCP server).

## Status
Phase 1 and 2 complete: behavior parity + docs-first project model.
See `docs/projects/workmesh/prds/phase-1-conversion.md` and `docs/projects/workmesh/prds/phase-2-docs-model.md`.
Phase 3 (sync engine + adapters) is planned.
