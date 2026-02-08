---
title: 'Coverage: add MCP tools unit tests and raise overall coverage'
child: []
kind: task
updated_date: 2026-02-07 00:52
uid: 01KGVDS5MK1W9G4BRQJQEKXHGT
blocked_by: []
dependencies: []
phase: Phase4
parent: []
id: task-main-050
status: Done
discovered_from: []
assignee: []
priority: P1
relationships: []
labels:
- coverage
- tests
- mcp
- dx
---
Description:
--------------------------------------------------
- 

Acceptance Criteria:
--------------------------------------------------
- 

Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.

Notes:
- Added unit tests inside workmesh-mcp tools module (list_tasks all/archive, set_status touch default, add_task) plus archive-aware loaders tests. Coverage now includes workmesh-mcp/src/tools.rs (no longer 0%) and overall line coverage increased (~50% locally).