---
id: task-ws1-007
uid: 01KHKS5DA319CT3J0QRBYBSZ00
title: Phase 1: concurrency and parity test gate
kind: task
status: To Do
priority: P1
phase: PhaseWS1
dependencies: [task-ws1-004, task-ws1-005, task-ws1-006]
labels: [phase1, workstreams, testing, parity]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-16 11:50
---
Description:
--------------------------------------------------
- Expand test gates for workstream concurrency, crash/recovery safety, and CLI/MCP parity.
- Include contention scenarios and restart safety assertions.
- Verify no regressions in existing Phase 0 guarantees.

Acceptance Criteria:
--------------------------------------------------
- New workstream tests cover concurrent mutations and restore safety.
- CLI/MCP parity tests cover core workstream flows.
- Full suite passes without weakening existing coverage.

Definition of Done:
--------------------------------------------------
- Test gate is comprehensive for Phase 1 scope.
- Acceptance criteria are fully met.
- Results provide confidence for multi-agent parallel workflows.
