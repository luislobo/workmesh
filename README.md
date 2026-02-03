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
- Docs-first project model under `docs/projects/<project-id>/`.
- Project scaffolding via `project-init` (CLI) / `project_init` (MCP).
- Validation for required fields, missing dependencies, and missing project docs.
- JSON output for CLI/MCP for easy automation.

## Repo layout
- `docs/` - project documentation, PRDs, decisions, and updates.
- `backlog/tasks/` - Markdown tasks managed by the CLI/MCP tools.
- `crates/` - Rust crates (CLI, core, MCP server).

## Status
Phase 1 and 2 complete: behavior parity + docs-first project model.
See `docs/projects/workmesh/prds/phase-1-conversion.md` and `docs/projects/workmesh/prds/phase-2-docs-model.md`.
Phase 3 (sync engine + adapters) is planned.
