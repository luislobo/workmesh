---
id: task-main-073
uid: 01KJDFX3H97K87YX0EYY1P03T5
title: Integrate render provider namespace into workmesh-service
kind: task
status: Done
priority: P1
phase: Phase7
dependencies: [task-main-072]
labels: [phase7, service, provider]
assignee: [luis]
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-26 11:40
---
Description:
--------------------------------------------------
- Register a new `render` namespace provider in `workmesh-service` using existing toolhost abstractions.
- Wire provider tool catalog into `/v1/providers` and invocation path into `/v1/mcp/invoke`.
- Keep existing `workmesh` and `system` provider behavior unchanged.
Acceptance Criteria:
--------------------------------------------------
- `/v1/providers` includes namespace `render` with core tool metadata.
- `/v1/mcp/invoke` successfully dispatches core render tools through namespace `render`.
- Existing providers (`workmesh`, `system`) continue to pass current tests.
Definition of Done:
--------------------------------------------------
- `render` provider is integrated with no regression in existing provider flows.
- Service integration tests cover provider discovery and invocation.
- Description goals are met and all Acceptance Criteria are satisfied.
- Code/config committed.
- Docs updated if needed.
