---
id: task-svc-002
title: "Phase SVC1: create workmesh-service crate and wiring"
kind: task
status: Done
priority: P1
phase: PhaseSVC1
dependencies: [task-svc-001]
labels: [svc1, service, crate]
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
- Create a new workspace crate for the standalone service and wire build/version/runtime scaffolding.

Acceptance Criteria:
--------------------------------------------------
- `cargo build -p workmesh-service` works and service starts with health endpoint.

Definition of Done:
--------------------------------------------------
- Crate is in workspace and buildable.
