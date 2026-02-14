---
id: task-migr-006
uid: 01KHAXGWSYSG9S4H2GSEKJTJGP
title: Add truth migration/backfill and validation tooling
kind: task
status: Done
priority: P2
phase: Phase2
dependencies: [task-migr-002, task-migr-005]
labels: [truth, migration, quality]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-13 09:22
---
Description:
--------------------------------------------------
- Provide migration/backfill tooling to detect legacy decision notes and bootstrap structured truth records where possible.
- Add validation/doctor checks for truth consistency, stale projections, and malformed records.
- Make migration behavior safe by default (dry-run first, explicit apply for writes).
Acceptance Criteria:
--------------------------------------------------
- Audit/plan/apply style tooling exists for truth migration/backfill with clear dry-run output.
- Validation detects malformed truth records, invalid state transitions in history, and projection mismatches.
- Tests cover migration dry-run/apply paths, no-op safety, and validation failure scenarios.
Definition of Done:
--------------------------------------------------
- Task goals in Description are met and all Acceptance Criteria are satisfied with reproducible test coverage.
- Migration tooling avoids destructive behavior by default and provides actionable diagnostics.
- Code/config committed.
- Docs updated if needed.
