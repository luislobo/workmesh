---
id: task-ws1-004
uid: 01KHKS5D8QE87EQQCA94VH3PE2
title: Phase 1: add MCP parity for workstream commands
kind: task
status: Done
priority: P1
phase: PhaseWS1
dependencies: [task-ws1-003]
labels: [phase1, workstreams, mcp, parity]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-16 13:49
---
Description:
--------------------------------------------------
- Implement MCP tools for the same workstream operations exposed by CLI.
- Ensure contract parity for fields, semantics, and error handling.
- Add regression coverage to prevent CLI/MCP drift.

Acceptance Criteria:
--------------------------------------------------
- MCP tool set supports all in-scope workstream operations.
- CLI and MCP behavior match for equivalent requests.
- Parity tests cover success, validation failures, and conflict/error paths.

Definition of Done:
--------------------------------------------------
- MCP parity is objectively test-backed.
- Acceptance criteria are fully met.
- No unresolved contract mismatch remains between interfaces.
