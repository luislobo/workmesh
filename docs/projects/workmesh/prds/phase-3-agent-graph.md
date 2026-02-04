# PRD: Phase 3 - Agent-ready graph + coordination

Date: 2026-02-03
Owner: Luis Lobo
Status: Draft

## Problem
WorkMesh needs to keep task history in-repo while enabling multi-agent work and rich dependency
relationships. Markdown tasks alone are not enough to support ready-work queries and durable
audit trails.

## Goals
- Treat dependencies and relationships as first-class (parent/child, blocked_by, discovered_from).
- Provide a "ready work" query that is deterministic and machine friendly.
- Support multi-agent coordination (assignee/lease/in-progress).
- Preserve an append-only audit trail for semantic task changes.
- Avoid ID collisions for concurrent task creation.
- Keep Markdown tasks as the source of truth with an optional structured index for speed.

## Non-goals
- Full UI/visual planner.
- Replacing Markdown tasks as the canonical source of truth.

## Requirements
- Task model adds `project`, `initiative`, `relationships`, `assignee`, `state` fields as needed.
- CLI/MCP: `ready` query, `claim`/`release` (lease) operations, and relationship management.
- Structured index (JSONL or sqlite) derived from Markdown tasks; rebuildable from source.
- Audit log for semantic updates (status changes, dependency edits, claims).
- ID strategy (ULID or namespaced IDs) to avoid collisions.

## Index design (proposed)
- Source of truth: Markdown tasks on disk.
- Index file: `backlog/.index/tasks.jsonl` (newline-delimited JSON per task).
- Entry fields: `id`, `path`, `status`, `priority`, `phase`, `dependencies`, `labels`,
  `assignee`, `project`, `initiative`, `updated_date`, `mtime`, `hash`.
- Operations:
  - `index rebuild` scans all tasks and rewrites the index.
  - `index refresh` updates only changed files using `mtime` + `hash`.
  - `index verify` compares index vs Markdown for drift.
- Usage:
  - `ready` queries can use the index for speed but must fall back to Markdown
    if index is missing or stale.

## Acceptance criteria
- `ready` command returns deterministic work items based on deps + status + lease state.
- Agents can safely claim work without stomping each other.
- A missing project/initiative is still validated against docs.
- Structured index rebuild matches Markdown tasks 1:1.
