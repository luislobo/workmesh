---
id: task-023
title: Phase 3: Resume command
status: Done
priority: P2
phase: Phase3
dependencies: [task-022]
labels: [phase3, resume, cli]
assignee: []
prd: docs/projects/workmesh/prds/phase-3-session-continuity.md
updated_date: 2026-02-04 00:09
---
Description:
--------------------------------------------------
- Add CLI/MCP resume command to load the latest checkpoint.
- Output concise summary + next actions.
Acceptance Criteria:
--------------------------------------------------
- `resume` works without extra context after a restart.
- Supports selecting a specific checkpoint by timestamp/id.
Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.
