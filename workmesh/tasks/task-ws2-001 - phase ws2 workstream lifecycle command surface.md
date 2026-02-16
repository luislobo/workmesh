---
id: task-ws2-001
title: "Phase WS2: workstream lifecycle command surface"
kind: task
status: Done
priority: P1
phase: PhaseWS2
dependencies: []
labels: [ws2, workstreams, lifecycle, parity]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-16 16:50
---
Description:
--------------------------------------------------
- Add workstream lifecycle operations so streams can be managed intentionally:
  - pause, close, reopen, rename, set (key/notes/context snapshot)
- Ensure CLI and MCP parity for the lifecycle surface.
- Make `workstream create` support binding to an existing worktree checkout (`--existing`).
- Ensure key derivation is deterministic and deduplicated within a repo when not provided.

Acceptance Criteria:
--------------------------------------------------
- CLI supports: `workstream pause|close|reopen|rename|set` with stable JSON output.
- MCP supports: `workstream_pause|close|reopen|rename|set` with equivalent behavior.
- `workstream create --existing --path <path> [--branch <branch>]` works and seeds context best-effort.
- Workstream keys are derived and unique within a repo when not provided.

Definition of Done:
--------------------------------------------------
- Behavior is implemented and covered by tests where practical.
- CLI/MCP parity tests remain green (or updated with explicit new coverage).
- Docs are updated where command surfaces changed.
