# PRD: Phase 3 - Sync engine (Jira/Trello/GitHub)

Date: 2026-02-01
Owner: Luis Lobo
Status: Draft

## Problem
Developers need WorkMesh to keep Jira, Trello, and GitHub in sync with local Markdown
without manual management, preserving full PM history inside the repo.

## Goals
- Bi-directional sync for tasks, comments, and status updates.
- MCP-first UX: "pull task updates" -> local updates + conflict detection/resolution.
- Full history stored in repo (`docs/projects/<project>/comments`, `events`, `conflicts`).

## Non-goals
- Real-time UI.
- Advanced analytics.

## Requirements
- External IDs stored in task front matter under `external`.
- Sync policies: per-field precedence + timestamp conflict window.
- MCP tools: `sync_pull`, `sync_push`, `sync_status`, `list_conflicts`, `resolve_conflict`.
- Adapter interfaces for Jira/Trello/GitHub.

## Acceptance criteria
- Simulated sync adapter can pull and update local tasks.
- Conflicts are recorded and resolvable via MCP.
