---
id: task-020
title: Phase 3: JSONL issues export command
status: To Do
priority: P3
phase: Phase3
dependencies: [task-016]
labels: [phase3, export, cli]
assignee: []
prd: docs/projects/workmesh/prds/phase-3-agent-graph.md
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
