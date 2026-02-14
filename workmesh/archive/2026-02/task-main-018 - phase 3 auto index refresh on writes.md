---
id: task-main-018
status: Done
assignee: []
prd: docs/projects/workmesh/prds/phase-3-agent-graph.md
title: 'Phase 3: Auto refresh index on writes'
phase: Phase3
priority: P2
dependencies:
- task-main-016
labels:
- phase3
- index
- core
updated_date: 2026-02-04 12:30
uid: 01KH5KY5CY4R4NPRQX5CNMHN6J
---
Description:
--------------------------------------------------
- Refresh JSONL index after CLI/MCP write operations.
- Keep index optional; failure to refresh should not block writes.
Acceptance Criteria:
--------------------------------------------------
- Mutating CLI/MCP commands attempt index refresh (best-effort).
- Index remains consistent after task edits without manual rebuild.
Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.