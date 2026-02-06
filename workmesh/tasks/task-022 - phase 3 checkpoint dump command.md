---
id: task-022
title: Phase 3: Checkpoint dump command
status: Done
priority: P2
phase: Phase3
dependencies: [task-009]
labels: [phase3, resume, cli]
assignee: []
prd: docs/projects/workmesh/prds/phase-3-session-continuity.md
updated_date: 2026-02-04 00:09
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
