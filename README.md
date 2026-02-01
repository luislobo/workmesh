# WorkMesh

WorkMesh is a MCP-first project and task system that keeps work in plain text while syncing with
Jira, Trello, and GitHub. It is designed for developers who want a clean DX: talk to the agent,
pull updates, resolve conflicts, and keep the full history of project management in the repo.

This repository is the Rust rewrite and evolution of xh-tasks.

## Repo layout
- `docs/` - project documentation, PRDs, decisions, and updates.
- `backlog/tasks/` - Markdown tasks managed by the CLI/MCP tools.
- `crates/` - Rust crates (CLI, core, MCP server).

## Status
Phase 1: Convert existing xh-tasks behavior to Rust while preserving behavior and rootless MCP.
See `docs/projects/workmesh/prds/phase-1-conversion.md`.
