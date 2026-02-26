---
id: task-main-059
uid: 01KHTA0VSWQ7GVWM13AD9PZEJN
title: Add MCP parity and tests for task-quality guardrails
kind: task
status: Done
priority: P1
phase: Phase5
dependencies: [task-main-057, task-main-058]
labels: [phase5, mcp, tests, parity]
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
- Add/adjust test coverage for task-quality guardrails in core and MCP parity suites.
- Verify CLI and MCP enforce equivalent completion gating semantics.
- Keep existing integration paths green after introducing stricter task quality rules.

Acceptance Criteria:
--------------------------------------------------
- Unit tests cover quality evaluation, Done gating, and normalization helpers.
- MCP parity tests assert matching CLI/MCP rejection when quality is insufficient for `Done`.
- Existing integration tests are updated to use valid required sections and continue passing.
- Workspace test suite passes with new guardrails.

Definition of Done:
--------------------------------------------------
- Test suite demonstrates guardrail correctness and CLI/MCP parity.
- No regressions remain in impacted core/CLI/MCP tests.
- Code/config committed.
- Docs updated if test-facing behavior changed.
