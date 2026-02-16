---
id: task-cif-006
title: 'Phase 0: JSONL robustness and recovery utilities'
status: To Do
priority: P1
phase: Phase0
dependencies: [task-cif-001]
labels: [phase0, recovery, jsonl]
assignee: []
kind: task
relationships:
  blocked_by: [task-cif-001]
  parent: []
  child: []
  discovered_from: []
---

Description:
--------------------------------------------------
- Implement tolerant JSONL readers that handle trailing malformed partial lines for event streams.
- Add safe recovery utilities that trim only trailing invalid JSONL lines and preserve valid history.
- Integrate recovery behavior for truth/session event stores where needed.

Acceptance Criteria:
--------------------------------------------------
- Trailing-partial-line scenario is tolerated by readers.
- Recovery only removes trailing malformed data and does not drop valid lines.
- Recovery behavior is test-covered with malformed input fixtures.
- Rebuild/projection flows operate correctly after recovery.

Definition of Done:
--------------------------------------------------
- Crash-safety and recovery requirements in the plan are implemented for JSONL paths.
- Acceptance criteria are met with explicit tests.
- Recovery behavior is deterministic and safe by default.

Notes:
- This task is a prerequisite for `doctor --fix-storage` implementation.
