---
prd: docs/projects/workmesh/prds/phase-3-session-continuity.md
id: task-main-026
title: 'Phase 3: Auto checkpoint on write'
assignee: []
phase: Phase3
dependencies:
- task-main-022
updated_date: 2026-02-04 00:09
status: Done
labels:
- phase3
- resume
- cli
priority: P3
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