---
id: task-svc-005
title: "Phase SVC1: auth middleware and login flow"
kind: task
status: Done
priority: P1
phase: PhaseSVC1
dependencies: [task-svc-004]
labels: [svc1, service, auth]
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
- Enforce token-based access for LAN usage with browser login support.

Acceptance Criteria:
--------------------------------------------------
- Protected routes reject unauthorized requests and accept valid token/session cookie.

Definition of Done:
--------------------------------------------------
- Non-loopback bind requires token and auth paths work.
