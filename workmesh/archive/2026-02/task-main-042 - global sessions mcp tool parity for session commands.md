---
status: Done
relationships: []
parent: []
discovered_from: []
phase: Phase4
updated_date: 2026-02-06 22:14
kind: task
dependencies: []
assignee: []
blocked_by: []
priority: P1
uid: 01KGV3B9Q46WWZ4K3RV98S0YE3
title: 'Global sessions: MCP tool parity for session commands'
labels:
- sessions
- mcp
child: []
id: task-main-042
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

Notes:
- Implemented MCP tools: session_save/session_list/session_show/session_resume backed by ~/.workmesh (WORKMESH_HOME). Added CLI<->MCP parity test for global sessions.