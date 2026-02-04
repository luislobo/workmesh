---
id: task-017
title: Phase 3: Graph export command
status: Done
priority: P2
phase: Phase3
dependencies: [task-011]
labels: [phase3, graph, cli]
assignee: []
prd: docs/projects/workmesh/prds/phase-3-agent-graph.md
updated_date: 2026-02-04 12:18
---
Description:
--------------------------------------------------
- Add graph export command (CLI) and MCP tool.
- Output property-graph JSON (nodes + edges).
- Include dependencies + relationships.
Acceptance Criteria:
--------------------------------------------------
- 

Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.

Notes:
- Implemented graph export command + MCP tool backed by core graph_export(). Added tests for edge output.
