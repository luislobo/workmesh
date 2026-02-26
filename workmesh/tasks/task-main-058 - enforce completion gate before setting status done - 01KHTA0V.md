---
id: task-main-058
uid: 01KHTA0VSFM8M7NW76J8493DYH
title: Enforce completion gate before setting status Done
kind: task
status: Done
priority: P1
phase: Phase5
dependencies: [task-main-057]
labels: [phase5, quality, workflow, done-gate]
assignee: [luis]
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-26 09:50
---

Description:
--------------------------------------------------
- Enforce the completion quality gate on every status mutation path that can set a task to `Done`.
- Align CLI and MCP behavior for `set-status`, `set-field status`, and bulk status mutations.
- Return explicit, deterministic quality conflict errors when completion requirements are not met.

Acceptance Criteria:
--------------------------------------------------
- CLI blocks `Done` transitions through both direct status and status-field updates when task quality is insufficient.
- MCP tools block the same transitions with equivalent checks and errors.
- Bulk status and bulk field status updates apply the same quality gate.
- Behavior parity is covered by tests.

Definition of Done:
--------------------------------------------------
- All status mutation paths consistently enforce Done gating.
- Completion failures are explicit and reproducible in CLI and MCP.
- Code/config committed.
- Docs updated to describe enforced behavior.
