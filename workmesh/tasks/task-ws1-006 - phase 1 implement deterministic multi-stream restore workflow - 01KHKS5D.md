---
id: task-ws1-006
uid: 01KHKS5D9NGN4ZMF922051MK2M
title: Phase 1: implement deterministic multi-stream restore workflow
kind: task
status: Done
priority: P1
phase: PhaseWS1
dependencies: [task-ws1-003, task-ws1-005]
labels: [phase1, workstreams, resume, dx]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-16 15:00
---
Description:
--------------------------------------------------
- Implement deterministic restore workflow for multiple active streams after reboot.
- Provide commands/output that clearly indicate what to open/resume next per stream.
- Ensure restore guidance aligns with Codex-first operation.

Acceptance Criteria:
--------------------------------------------------
- User can list active streams and resume each with unambiguous next commands.
- Restore output includes essential stream context (path/session/objective/next task).
- Behavior is deterministic across repeated invocations.

Definition of Done:
--------------------------------------------------
- Multi-stream recovery workflow works end-to-end.
- Acceptance criteria are fully met.
- Tests prove stable output and no missing critical state.
