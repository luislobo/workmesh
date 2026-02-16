---
id: task-cif-009
title: 'Phase 0: concurrency and crash-safety test gate'
status: To Do
priority: P1
phase: Phase0
dependencies: [task-cif-003, task-cif-004, task-cif-005, task-cif-006, task-cif-007, task-cif-008]
labels: [phase0, testing, concurrency]
assignee: []
kind: task
relationships:
  blocked_by: [task-cif-003, task-cif-004, task-cif-005, task-cif-006, task-cif-007, task-cif-008]
  parent: []
  child: []
  discovered_from: []
---

Description:
--------------------------------------------------
- Implement required Phase 0 test gate:
  - unit tests for atomicity, locked append, CAS conflicts, unversioned migration
  - integration tests for parallel claims, parallel session saves, parallel worktree updates
  - forced crash/restart simulation for recovery correctness
- Ensure existing CLI/MCP parity suite remains green.

Acceptance Criteria:
--------------------------------------------------
- All required unit and integration scenarios are implemented and passing.
- No known lost-update race remains in critical tracking paths.
- Existing parity tests continue passing without regressions.
- Test coverage explicitly exercises contention and restart safety behavior.

Definition of Done:
--------------------------------------------------
- Phase 0 test gate requirements are fully met.
- Acceptance criteria are validated in CI/local runs.
- Results provide confidence for multi-agent parallel safety.

Notes:
- This is implementation sequence step 7.
