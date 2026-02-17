---
id: task-ws3-003
title: "Phase WS3: workstream show restore view"
kind: task
status: Done
priority: P2
phase: PhaseWS3
dependencies: [task-ws3-001]
labels: [ws3, workstreams, restore]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-16 18:28
---
Description:
--------------------------------------------------
- Improve day-2 resumption by exposing a single-stream restore view (resume commands, issues, next
  task) via `workstream show`.

Acceptance Criteria:
--------------------------------------------------
- CLI: `workstream show --restore` includes resume commands for the selected stream.
- MCP: `workstream_show` supports `restore=true` and returns the same restore view.
- Works for both id and key selection (consistent with existing workstream selection rules).

Definition of Done:
--------------------------------------------------
- Output is stable and deterministic.
- Docs mention the single-stream restore path.

