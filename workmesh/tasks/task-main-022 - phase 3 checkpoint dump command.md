---
prd: docs/projects/workmesh/prds/phase-3-session-continuity.md
status: Done
title: 'Phase 3: Checkpoint dump command'
dependencies:
- task-main-010
updated_date: 2026-02-04 00:09
id: task-main-022
phase: Phase3
labels:
- phase3
- resume
- cli
priority: P2
assignee: []
---
Description:
--------------------------------------------------
- Add CLI/MCP checkpoint command that writes JSON + Markdown snapshots.
- Include current task, ready list, leases, git status summary, changed files,
  top-level directories touched, and recent audit events.
Acceptance Criteria:
--------------------------------------------------
- Outputs go under `docs/projects/<project>/updates/` with timestamped filenames.
- Missing optional data does not fail the command.
Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.