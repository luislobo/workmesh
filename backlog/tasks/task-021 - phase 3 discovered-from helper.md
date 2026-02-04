---
id: task-021
title: Phase 3: Discovered-from helper
status: Done
priority: P3
phase: Phase3
dependencies: [task-011]
labels: [phase3, graph, cli]
assignee: []
prd: docs/projects/workmesh/prds/phase-3-agent-graph.md
updated_date: 2026-02-04 12:55
---
Description:
--------------------------------------------------
- Add helper command to create a task linked via relationships.discovered_from.
- Support CLI and MCP.
Acceptance Criteria:
--------------------------------------------------
- New command creates task + sets discovered_from relationship.
- Works with both explicit task IDs and auto-generated IDs.
Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.
