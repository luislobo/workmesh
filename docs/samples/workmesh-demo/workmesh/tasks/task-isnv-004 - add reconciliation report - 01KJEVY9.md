---
id: task-isnv-004
uid: 01KJEVY9FC0K885MF9WFD5KSBD
title: Add reconciliation report
kind: task
status: Done
priority: P2
phase: Phase2
dependencies: [task-isnv-003]
labels: []
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-27 00:24
---

Description:
--------------------------------------------------
- Add a reconciliation report to compare remote vs. local inventory snapshots.
- Include summary counts, deltas, and missing item diagnostics.
- Keep reporting logic separate from sync adapter responsibilities.

Acceptance Criteria:
--------------------------------------------------
- Report outputs totals, deltas, and a list of mismatched items.
- Supports a time-bounded run (last sync window).
- Can be exported to a simple CSV/JSON artifact.

Definition of Done:
--------------------------------------------------
- Description goals met and acceptance criteria satisfied.
- Code/config committed.
- Docs updated if needed.
