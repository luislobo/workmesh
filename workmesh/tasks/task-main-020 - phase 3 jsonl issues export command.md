---
assignee: []
prd: docs/projects/workmesh/prds/phase-3-agent-graph.md
dependencies:
- task-main-016
priority: P3
id: task-main-020
title: 'Phase 3: JSONL issues export command'
phase: Phase3
status: Done
labels:
- phase3
- export
- cli
updated_date: 2026-02-04 12:50
---
Description:
--------------------------------------------------
- Add CLI/MCP command to emit canonical JSONL issues snapshot.
- Output should include full task snapshot with relationships and UID.
Acceptance Criteria:
--------------------------------------------------
- Command emits JSONL to stdout or file.
- Output is deterministic and sorted.
Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.