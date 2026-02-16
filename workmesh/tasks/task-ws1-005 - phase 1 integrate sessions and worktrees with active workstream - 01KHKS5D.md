---
id: task-ws1-005
uid: 01KHKS5D96NZJGB6JQEXRYJ5S9
title: Phase 1: integrate sessions and worktrees with active workstream
kind: task
status: To Do
priority: P1
phase: PhaseWS1
dependencies: [task-ws1-002, task-ws1-003]
labels: [phase1, workstreams, sessions, worktrees]
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
- Integrate global sessions and worktree bindings into the active workstream lifecycle.
- Ensure saves/resumes and attach/detach flows update stream state deterministically.
- Preserve existing workflows while adding workstream-aware behavior.

Acceptance Criteria:
--------------------------------------------------
- Session and worktree operations can associate with an active workstream.
- Associations are persisted safely and survive process restarts.
- Existing non-workstream flows remain functional.

Definition of Done:
--------------------------------------------------
- Integration behavior is stable and tested.
- Acceptance criteria are fully met.
- Multi-stream usage no longer requires manual cross-file coordination.
