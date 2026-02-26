---
id: task-main-057
uid: 01KHTA0VRS2EF7H313CEAZX1PP
title: Implement strict task section validator (Description, Acceptance Criteria, Definition of Done)
kind: task
status: Done
priority: P1
phase: Phase5
dependencies: [task-main-056]
labels: [phase5, quality, tasks, validator]
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
- Implement strict task quality evaluation in `workmesh-core` for required sections and completion readiness.
- Add structured quality report data used by validation and Done gating logic.
- Ensure incomplete placeholders and hygiene-only Definition of Done text are flagged.

Acceptance Criteria:
--------------------------------------------------
- `evaluate_task_quality` identifies missing/incomplete required sections.
- `ensure_task_quality_for_done` blocks completion for tasks missing required quality.
- `validate` reports warnings for non-Done tasks and errors for Done tasks that violate quality policy.
- Default task template DoD includes outcome-oriented completion language.

Definition of Done:
--------------------------------------------------
- Validator behavior is implemented and exercised by tests.
- Task completion gate relies on quality checks, not only status mutation intent.
- Code/config committed.
- Docs updated where quality policy behavior is described.
