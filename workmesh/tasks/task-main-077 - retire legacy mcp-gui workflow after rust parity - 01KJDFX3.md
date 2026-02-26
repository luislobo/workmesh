---
id: task-main-077
uid: 01KJDFX3W4SW9S5Q4X4FAZ2XFK
title: Retire legacy mcp-gui workflow after Rust parity
kind: task
status: Done
priority: P1
phase: Phase7
dependencies: [task-main-075, task-main-076]
labels: [phase7, deprecation, migration]
assignee: [luis]
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-26 11:40
---
Description:
--------------------------------------------------
- Finalize migration from external Node `mcp-gui` workflow to native Rust `render` provider in WorkMesh.
- Remove or deprecate stale references and update migration notes/checklists.
- Close phase status once parity and documentation gates are complete.
Acceptance Criteria:
--------------------------------------------------
- Migration and deprecation guidance is documented and actionable.
- No active docs point to Node `mcp-gui` as the primary renderer path.
- Phase closure evidence (tests/docs/status) is recorded.
Definition of Done:
--------------------------------------------------
- Native Rust renderer path is the authoritative workflow.
- Legacy guidance is retired or explicitly marked deprecated.
- Description goals are met and all Acceptance Criteria are satisfied.
- Code/config committed.
- Docs updated if needed.
