# PRD: Phase 1 - Rust conversion (behavior parity)

Date: 2026-02-01
Owner: Luis Lobo
Status: Draft

## Problem
xh-tasks is a Python CLI + MCP server. We need a Rust implementation that preserves existing
behavior while preparing for future sync and conflict-resolution features.

## Goals
- Rust CLI and MCP server that match the current xh-tasks behavior.
- Rootless MCP: if started inside a repo, infer the backlog root from CWD.
- Maintain Markdown task format and tolerant front-matter parsing.
- Keep the codebase ready for sync and conflict modules in later phases.

## Non-goals
- Jira/Trello/GitHub sync in Phase 1.
- New UI or visual planning views.
- Migration of existing user data (handled later).

## Requirements
- CLI commands: list, next, show, stats, set-status, set-field, label add/remove,
  dep add/remove, note, set-body, set-section, add, validate, export, gantt (text/file/svg).
- MCP tools: same surface as xh-tasks with optional `root`.
- File layout compatibility: `tasks/`, `backlog/tasks`, or `project/tasks`.
- Tests covering task parsing, root resolution, and core operations.

## Acceptance criteria
- A Rust binary can run `list` and `show` with output parity for a sample backlog.
- MCP server can run without `root` and list tasks when started in a repo.
- Tests pass for the core task parsing and list/next logic.
