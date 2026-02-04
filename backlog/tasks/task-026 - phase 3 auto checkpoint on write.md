---
id: task-026
title: Phase 3: Auto checkpoint on write
status: To Do
priority: P3
phase: Phase3
dependencies: [task-022]
labels: [phase3, resume, cli]
assignee: []
prd: docs/projects/workmesh/prds/phase-3-session-continuity.md
---
Description:
--------------------------------------------------
- Automatically append a checkpoint after mutating CLI/MCP commands.
Acceptance Criteria:
--------------------------------------------------
- Best-effort only; never blocks writes.
- Configurable via flag or env.
Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.
