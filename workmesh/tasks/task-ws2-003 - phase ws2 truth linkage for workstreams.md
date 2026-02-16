---
id: task-ws2-003
title: "Phase WS2: truth linkage for workstreams"
kind: task
status: Done
priority: P2
phase: PhaseWS2
dependencies: [task-ws2-001]
labels: [ws2, truth, workstreams]
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
- Improve Truth Ledger ergonomics so agents can capture durable decisions tied to the current
  workstream and later discover them quickly.
- Add a `workstream_id` context field in Truth records and expose it in list filters.
- Improve `workstream show` to optionally list accepted Truth records linked to that stream.

Acceptance Criteria:
--------------------------------------------------
- `TruthContext` supports `workstream_id` (optional, backward compatible).
- CLI:
  - `truth propose` accepts `--workstream-id` and `--current` convenience behavior.
  - `truth list` supports filtering by `--workstream-id`.
  - `workstream show --truth` lists accepted Truth records linked to that workstream.
- MCP parity:
  - `truth_propose` and `truth_list` accept `workstream_id`.
  - `workstream_show` can include linked truths.

Definition of Done:
--------------------------------------------------
- Truth storage and projection remain valid and backward compatible.
- Tests cover query behavior for the new field.
- CLI/MCP parity remains green.
