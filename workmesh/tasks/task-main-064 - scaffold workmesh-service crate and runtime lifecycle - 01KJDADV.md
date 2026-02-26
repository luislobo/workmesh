---
id: task-main-064
uid: 01KJDADV2GDKKKB1B5T8YCRGJH
title: Scaffold workmesh-service crate and runtime lifecycle
kind: task
status: To Do
priority: P1
phase: Phase6
dependencies: [task-main-063]
labels: [phase6, service, runtime]
assignee: [luis]
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-26 09:53
---
Description:
--------------------------------------------------
- Create `workmesh-service` crate scaffold with config loading, lifecycle hooks, and base HTTP server.
- Provide health/readiness endpoints and stable startup/shutdown behavior.
Acceptance Criteria:
--------------------------------------------------
- New crate builds in workspace and starts a service process from CLI/service entrypoint.
- Health/readiness endpoints return deterministic responses.
- Runtime uses existing storage/concurrency guarantees from `workmesh-core` where state is touched.
Definition of Done:
--------------------------------------------------
- Service runtime foundation exists and can be started/stopped reliably in local development.
- Foundation enables transport and provider work without architectural rework.
- Code/config committed for implemented runtime scaffold.
- Docs updated if needed.
