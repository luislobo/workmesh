---
status: Done
priority: P2
assignee: []
title: 'Phase 3: Assignee + lease coordination'
phase: Phase3
prd: docs/projects/workmesh/prds/phase-3-agent-graph.md
id: task-main-013
updated_date: 2026-02-04 12:05
dependencies: []
labels:
- phase3
- coordination
- core
uid: 01KH5KY56HFV8ESB8068JRF8KA
---
Description:
--------------------------------------------------
- Add assignee + lease fields.
- Implement claim/release operations for multi-agent coordination.
Acceptance Criteria:
--------------------------------------------------
- 

Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.

Notes:
- Added lease coordination: lease parsing (nested/flat), claim/release CLI + MCP tools, ready tasks exclude active leases, JSON includes lease, tests added.