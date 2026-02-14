---
prd: docs/projects/workmesh/prds/phase-3-agent-graph.md
status: Done
dependencies:
- task-main-011
title: 'Phase 3: Discovered-from helper'
updated_date: 2026-02-04 12:55
labels:
- phase3
- graph
- cli
id: task-main-021
assignee: []
priority: P3
phase: Phase3
uid: 01KH5KY5GPHWES58WWCPBMKVPP
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