---
id: task-svc-003
title: "Phase SVC1: read model aggregation across sessions workstreams worktrees"
kind: task
status: Done
priority: P1
phase: PhaseSVC1
dependencies: [task-svc-002]
labels: [svc1, service, data]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-18 00:00
---
Description:
--------------------------------------------------
- Aggregate global registries into a unified snapshot for UI/API consumption.

Acceptance Criteria:
--------------------------------------------------
- Snapshot includes sessions, workstreams, worktrees, repos, and warnings.

Definition of Done:
--------------------------------------------------
- Aggregation is deterministic and covered by tests.
