---
id: task-main-066
uid: 01KJDADV3GYX4ET3C0A38SPZME
title: Implement extensible tool-host registry and namespaced dispatch
kind: task
status: To Do
priority: P1
phase: Phase6
dependencies: [task-main-064]
labels: [phase6, platform, toolhost]
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
- Introduce tool-host/provider registry abstraction for multiple tool domains in one service runtime.
- Implement namespaced routing and provider metadata discovery.
Acceptance Criteria:
--------------------------------------------------
- Service can register and dispatch at least one non-task provider alongside core WorkMesh provider.
- Namespace collisions and unsupported tools return clear errors.
- Provider contract is documented for future additions.
Definition of Done:
--------------------------------------------------
- Multi-provider hosting works in a single runtime with deterministic dispatch.
- Provider interface is clear enough for future tool-domain onboarding.
- Code/config committed for provider registry implementation.
- Docs updated if needed.
