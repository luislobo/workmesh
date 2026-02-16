---
id: task-cif-007
title: 'Phase 0: doctor integrity checks and --fix-storage'
status: Done
priority: P1
phase: Phase0
dependencies: [task-cif-003, task-cif-004, task-cif-005, task-cif-006]
labels: [phase0, doctor, integrity]
assignee: []
kind: task
relationships:
  blocked_by: [task-cif-003, task-cif-004, task-cif-005, task-cif-006]
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-15 23:49
---
Description:
--------------------------------------------------
- Extend `doctor` output with storage integrity checks:
  - lock-path accessibility
  - malformed JSONL count
  - projection/event divergence
  - version monotonicity checks for versioned snapshots
- Add CLI `doctor --fix-storage` for safe remediation:
  - trailing malformed JSONL truncation
  - projection rebuild

Acceptance Criteria:
--------------------------------------------------
- `doctor` reports all planned integrity dimensions.
- `--fix-storage` performs only safe remediations within defined scope.
- Fix behavior is test-covered, including no-overreach guarantees.
- Diagnostics are deterministic and machine-readable in JSON mode.

Definition of Done:
--------------------------------------------------
- Doctor/fix capabilities from the plan are fully implemented.
- Acceptance criteria are met and validated with tests.
- Operators can detect and remediate storage anomalies reliably.

Notes:
- Clarification: storage doctor now includes --fix-storage safe remediation for trailing malformed JSONL and projection/index rebuilds.
- Extended doctor diagnostics with storage integrity checks: lock-path accessibility, JSONL malformed counts (including trailing vs non-trailing), truth projection mismatch/transition errors, and versioned snapshot checks. Added  pathway (safe trailing trim + rebuild flows) and core test coverage.
- This is implementation sequence step 6.
