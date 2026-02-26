---
id: task-main-056
uid: 01KHTA0MMQD6KJYAQNJJ9B4KET
title: Phase 5: task quality and completion guardrails
kind: task
status: Done
priority: P1
phase: Phase5
dependencies: []
labels: [phase5, quality, tasks, workflow]
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
- Deliver Phase 5 end-to-end: enforce task quality standards before completion, keep CLI/MCP behavior aligned, add migration support for legacy task bodies, and document the policy in user docs and skills.
- Ensure completion reflects outcomes and acceptance criteria, not only hygiene checklist items.

Acceptance Criteria:
--------------------------------------------------
- Required sections (`Description`, `Acceptance Criteria`, `Definition of Done`) are enforced for task completion.
- Status transitions to `Done` are blocked when task quality requirements are not met, across CLI and MCP mutation paths.
- Legacy task structures can be detected and normalized with migration tooling.
- Documentation and skills clearly state the quality policy and completion expectations.

Definition of Done:
--------------------------------------------------
- Phase 5 goals are met across code, tests, and documentation.
- Acceptance criteria in this task are satisfied and validated.
- Code/config committed.
- Docs updated where policy/behavior changed.
