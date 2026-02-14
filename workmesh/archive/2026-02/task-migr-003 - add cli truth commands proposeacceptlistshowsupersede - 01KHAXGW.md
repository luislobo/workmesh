---
id: task-migr-003
uid: 01KHAXGWSA5S7M9QC37NDA7CTB
title: Add CLI truth commands (propose/accept/list/show/supersede)
kind: task
status: Done
priority: P1
phase: Phase1
dependencies: [task-migr-002]
labels: [truth, cli]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-13 09:22
---
Description:
--------------------------------------------------
- Add CLI command surface for truth workflows: propose, accept, show/list, and supersede.
- Ensure CLI UX supports both human-readable and JSON outputs suitable for agent automation.
- Wire command validation/errors to core domain semantics without duplicating business logic.
Acceptance Criteria:
--------------------------------------------------
- CLI commands can create and transition truth records with stable JSON output contracts.
- CLI rejects malformed IDs, missing required fields, and invalid transitions with clear errors.
- CLI tests validate command behavior, output shape, and parity with core transition rules.
Definition of Done:
--------------------------------------------------
- Task goals in Description are met and all Acceptance Criteria are satisfied in CLI integration tests.
- CLI implementation remains thin orchestration over core truth APIs.
- Code/config committed.
- Docs updated if needed.
