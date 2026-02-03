# PRD: Phase 3 - Sync engine + agent-ready graph

Date: 2026-02-03
Owner: Luis Lobo
Status: Draft

## Problem
WorkMesh needs to keep task history in-repo while enabling multi-agent work, rich dependency
relationships, and synchronization with external systems (Jira/Trello/GitHub). Markdown tasks alone
are not enough to support ready-work queries, conflict resolution, and durable audit trails.

## Goals
- Treat dependencies and relationships as first-class (parent/child, blocked_by, discovered_from).
- Provide a "ready work" query that is deterministic and machine friendly.
- Support multi-agent coordination (assignee/lease/in-progress).
- Preserve an append-only audit trail for semantic task changes.
- Avoid ID collisions for concurrent task creation.
- Keep Markdown tasks as the source of truth with an optional structured index for speed.
- Establish a sync engine interface for Jira/Trello/GitHub adapters.

## Non-goals
- Full UI/visual planner.
- Replacing Markdown tasks as the canonical source of truth.
- Automatic two-way sync for all providers in the first cut.

## Requirements
- Task model adds `project`, `initiative`, `relationships`, `assignee`, `state` fields as needed.
- CLI/MCP: `ready` query, `claim`/`release` (lease) operations, and relationship management.
- Structured index (JSONL or sqlite) derived from Markdown tasks; rebuildable from source.
- Audit log for semantic updates (status changes, dependency edits, claims).
- ID strategy (ULID or namespaced IDs) to avoid collisions.
- Sync engine abstraction with at least one adapter stub.

## Acceptance criteria
- `ready` command returns deterministic work items based on deps + status + lease state.
- Agents can safely claim work without stomping each other.
- A missing project/initiative is still validated against docs.
- Structured index rebuild matches Markdown tasks 1:1.
- External sync adapter interface is defined and wired (even if stubbed).
