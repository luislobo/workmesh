---
id: task-main-061
uid: 01KHTA0VTKCH7YFR2ZN92PQVK4
title: Add migration helper to normalize legacy tasks to required sections
kind: task
status: Done
priority: P2
phase: Phase5
dependencies: [task-main-057]
labels: [phase5, migration, tasks, tooling]
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
- Add migration-audit detection and apply support to normalize legacy tasks missing required quality sections.
- Introduce a migration action for task section normalization and execute it via existing migration flow.
- Ensure normalization is safe and test-covered.

Acceptance Criteria:
--------------------------------------------------
- Migration audit reports legacy task section issues and proposes `task_section_normalization`.
- Migration apply performs section scaffolding for affected tasks and reports changes.
- Core helper exists to normalize required sections without corrupting task content.
- Tests validate migration detection/apply behavior.

Definition of Done:
--------------------------------------------------
- Legacy tasks can be migrated to required section structure through tooling.
- Migration plan/apply flows include and exercise the new action.
- Code/config committed.
- Docs updated where migration actions are documented.
