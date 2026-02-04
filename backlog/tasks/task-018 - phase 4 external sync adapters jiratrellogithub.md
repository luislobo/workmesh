---
id: task-018
title: Phase 4: External sync adapters (Jira/Trello/GitHub)
status: To Do
priority: P3
phase: Phase4
dependencies: []
labels: [phase4, sync]
assignee: []
prd: docs/projects/workmesh/prds/phase-4-sync-engine.md
---
Description:
--------------------------------------------------
- Implement real adapters for Jira/Trello/GitHub.
- Wire `sync_pull`, `sync_push`, `sync_status`, `list_conflicts`, `resolve_conflict`.
- Persist external comments/events/conflicts into docs folders.
Acceptance Criteria:
--------------------------------------------------
- At least one real adapter can pull and update local tasks.
- Conflicts are recorded and resolvable via MCP.
- External IDs stored in task front matter under `external`.
Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.
