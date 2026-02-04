---
id: task-018
title: Phase 3: Auto refresh index on writes
status: To Do
priority: P2
phase: Phase3
dependencies: [task-016]
labels: [phase3, index, core]
assignee: []
prd: docs/projects/workmesh/prds/phase-3-agent-graph.md
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
