---
id: task-042
uid: 01KGV3B9Q46WWZ4K3RV98S0YE3
title: Global sessions: MCP tool parity for session commands
kind: task
status: To Do
priority: P1
phase: Phase4
dependencies: []
labels: [sessions, mcp]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
---

Description:
--------------------------------------------------
- Add MCP tools with CLI parity:
  - `session_save`
  - `session_list`
  - `session_show`
  - `session_resume`
- Ensure root is optional (these operate on the global store).
- Add parity tests (CLI vs MCP) for session lifecycle.

Acceptance Criteria:
--------------------------------------------------
- MCP tools return deterministic JSON.
- CLI/MCP parity tests cover save/list/show/resume end-to-end.

Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.
