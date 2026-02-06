---
id: task-018
title: Phase 3: Auto refresh index on writes
status: Done
priority: P2
phase: Phase3
dependencies: [task-016]
labels: [phase3, index, core]
assignee: []
prd: docs/projects/workmesh/prds/phase-3-agent-graph.md
updated_date: 2026-02-04 12:30
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
