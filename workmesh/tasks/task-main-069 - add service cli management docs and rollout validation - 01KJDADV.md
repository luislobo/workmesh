---
id: task-main-069
uid: 01KJDADV50MXKA1YP0C6YCE8DQ
title: Add service CLI management, docs, and rollout validation
kind: task
status: To Do
priority: P2
phase: Phase6
dependencies: [task-main-067, task-main-068]
labels: [phase6, docs, dx, rollout]
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
- Add service-management command surface and complete user/operator documentation.
- Validate phased rollout and fallback behavior for local development usage.
Acceptance Criteria:
--------------------------------------------------
- CLI includes service lifecycle commands and clear diagnostics for misconfiguration.
- Documentation covers local setup, LAN setup, auth, reload, and troubleshooting.
- Validation runbook confirms parity, stability, and rollback steps.
Definition of Done:
--------------------------------------------------
- Developers can operate service mode end-to-end using documented commands and runbooks.
- Rollout can be executed safely with clear fallback path.
- Code/config committed for management surface and validation artifacts.
- Docs updated if needed.
