# PRD: Phase 1 - Rust conversion (behavior parity)

Date: 2026-02-01
Owner: Luis Lobo
Status: Draft

## Problem
WorkMesh needs a Rust implementation that preserves the existing behavior.

## Goals
- Rust CLI and MCP server that match the current behavior.
- Rootless MCP: if started inside a repo, infer the workmesh root from CWD.
- Maintain Markdown task format and tolerant front-matter parsing.

## Non-goals
- New UI or visual planning views.
- Migration of existing user data (handled later).

## Requirements
- CLI commands: list, next, show, stats, set-status, set-field, label add/remove,
  dep add/remove, note, set-body, set-section, add, validate, export, gantt (text/file/svg).
- MCP tools: same surface with optional `root`.
- File layout compatibility: `workmesh/tasks`, `.workmesh/tasks`, `tasks/`, and legacy `backlog/tasks` or `project/tasks`.
- Tests covering task parsing, root resolution, and core operations.

## Acceptance criteria
- A Rust binary can run `list` and `show` with output parity for a sample workmesh.
- MCP server can run without `root` and list tasks when started in a repo.
- Tests pass for the core task parsing and list/next logic.
