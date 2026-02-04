---
id: task-010
title: Phase 3: Sync engine + adapters (Jira/Trello/GitHub)
status: Done
priority: P2
phase: Phase3
dependencies: [task-009, task-011, task-012, task-013, task-014, task-015, task-016]
labels: [phase3, sync]
assignee: []
prd: docs/projects/workmesh/prds/phase-3-sync-and-graph.md
---
Description:
--------------------------------------------------
- Implement sync engine + adapter interfaces (Jira/Trello/GitHub).
- Add agent-ready graph capabilities (relationships, ready query, leases, audit log, ID strategy, structured index).
- Keep Markdown tasks as source of truth.
Acceptance Criteria:
--------------------------------------------------
- Subtasks task-011 through task-016 are complete.
- Sync engine abstraction is in place with at least one adapter stub.
- Docs updated for new capabilities.
Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.

Notes:
--------------------------------------------------
- Added sync engine scaffold in core with adapter trait + stub adapters (jira/trello/github).
