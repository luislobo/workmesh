---
id: task-main-068
uid: 01KJDADV4GD8S95WMPAZA7MCW5
title: Add LAN-safe auth, limits, and service observability baseline
kind: task
status: To Do
priority: P1
phase: Phase6
dependencies: [task-main-065]
labels: [phase6, security, observability]
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
- Add baseline security and operations controls for LAN-capable service mode.
- Implement token auth, request limits, structured logs, and metrics/status endpoints.
Acceptance Criteria:
--------------------------------------------------
- Localhost remains default bind behavior; LAN exposure requires explicit opt-in.
- Non-localhost access requires authentication by default when enabled.
- Metrics/logging/status data are available for basic operations and troubleshooting.
Definition of Done:
--------------------------------------------------
- Service can be exposed on LAN with sensible guardrails and observability.
- Operational troubleshooting data is available without ad-hoc instrumentation.
- Code/config committed for security and observability baseline.
- Docs updated if needed.
