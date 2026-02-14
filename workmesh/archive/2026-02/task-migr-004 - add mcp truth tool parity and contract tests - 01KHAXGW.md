---
id: task-migr-004
uid: 01KHAXGWSF8SP5KKKWWD9G5QSP
title: Add MCP truth tool parity and contract tests
kind: task
status: Done
priority: P1
phase: Phase1
dependencies: [task-migr-002]
labels: [truth, mcp]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-13 09:22
---
Description:
--------------------------------------------------
- Add MCP tool parity for truth workflows so agents can perform the same operations available in CLI.
- Keep MCP tool schemas explicit and stable for multi-agent interoperability.
- Ensure MCP behavior and error semantics align with CLI/core behavior.
Acceptance Criteria:
--------------------------------------------------
- MCP tools exist for proposing, accepting, listing/showing, and superseding truth records.
- Tool schemas are discoverable via tool metadata and include practical usage examples.
- Parity tests validate MCP/CLI behavioral equivalence for core truth scenarios.
Definition of Done:
--------------------------------------------------
- Task goals in Description are met and all Acceptance Criteria are satisfied via parity tests.
- MCP layer delegates lifecycle rules to core truth logic (no duplicated state machines).
- Code/config committed.
- Docs updated if needed.
