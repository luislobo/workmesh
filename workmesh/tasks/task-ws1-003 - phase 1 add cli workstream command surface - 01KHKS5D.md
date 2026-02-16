---
id: task-ws1-003
uid: 01KHKS5D89E1TM9AB2JKEENAHW
title: Phase 1: add CLI workstream command surface
kind: task
status: To Do
priority: P1
phase: PhaseWS1
dependencies: [task-ws1-001, task-ws1-002]
labels: [phase1, workstreams, cli]
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
- Add CLI workstream command surface for creating, listing, showing, switching, and diagnosing streams.
- Integrate command behavior with context/session/worktree references.
- Ensure command UX remains concise and deterministic for daily use.

Acceptance Criteria:
--------------------------------------------------
- CLI command surface exists and is documented with JSON/text output behavior.
- Operations are wired to core workstream registry safely.
- Error behavior is explicit and consistent with existing CLI patterns.

Definition of Done:
--------------------------------------------------
- CLI commands are usable end-to-end with no critical gaps.
- Acceptance criteria are fully met.
- Implementation is covered by targeted CLI tests.
