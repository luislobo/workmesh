---
id: task-isnv-003
uid: 01KJEVY57CVQ7CCF51HB567R8J
title: Implement sync adapter
kind: task
status: Done
priority: P1
phase: Phase2
dependencies: [task-isnv-002]
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
- Implement the sync adapter against the contract using a clean interface.
- Keep responsibilities separated (adapter vs. transport vs. validation).
- Add unit coverage for add/update/delete scenarios.

Acceptance Criteria:
--------------------------------------------------
- Adapter handles create/update/delete events and maps to internal model.
- Validation rejects malformed payloads with actionable errors.
- Unit tests cover happy path and one failure case per event type.

Definition of Done:
--------------------------------------------------
- Description goals met and acceptance criteria satisfied.
- Code/config committed.
- Docs updated if needed.
