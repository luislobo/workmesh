---
status: Done
assignee: []
dependencies: []
title: 'Phase 3: Structured index + rebuild'
priority: P2
labels:
- phase3
- index
- core
prd: docs/projects/workmesh/prds/phase-3-agent-graph.md
updated_date: 2026-02-04 12:16
id: task-main-016
phase: Phase3
---
Description:
--------------------------------------------------
- Add structured index (JSONL/sqlite) derived from Markdown.
- Implement rebuild/verify workflow.
Acceptance Criteria:
--------------------------------------------------
- 

Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.

Notes:
--------------------------------------------------
- Implemented JSONL index at `backlog/.index/tasks.jsonl` with rebuild/refresh/verify (CLI + MCP).