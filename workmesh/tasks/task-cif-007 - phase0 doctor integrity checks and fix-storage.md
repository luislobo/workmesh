---
id: task-cif-007
title: 'Phase 0: doctor integrity checks and --fix-storage'
status: To Do
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
- This is implementation sequence step 6.
