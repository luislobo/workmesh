---
prd: docs/projects/workmesh/prds/phase-3-agent-graph.md
phase: Phase3
priority: P2
dependencies:
- task-main-011
- task-main-013
id: task-main-012
labels:
- phase3
- query
- core
status: Done
assignee: []
updated_date: 2026-02-04 12:10
title: 'Phase 3: Ready work query (CLI/MCP)'
---
Description:
--------------------------------------------------
- Implement ready-work query using dependencies + status + leases.
- Expose via CLI and MCP with deterministic ordering.
Acceptance Criteria:
--------------------------------------------------
- 

Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.

Notes:
- Added ready_tasks in core using dependencies + relationships.blocked_by; CLI ready command and MCP ready_tasks tool with deterministic ordering + optional limit; added tests.