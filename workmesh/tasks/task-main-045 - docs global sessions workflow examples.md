---
kind: task
uid: 01KGV3B9RRCRDS1GB7F6G910CB
updated_date: 2026-02-06 22:27
status: Done
phase: Phase4
blocked_by: []
parent: []
priority: P2
title: 'Docs: global sessions workflow + examples'
id: task-main-045
relationships: []
assignee: []
labels:
- sessions
- docs
discovered_from: []
dependencies: []
child: []
---
Description:
--------------------------------------------------
- Document global sessions in `README.md` and in project docs:
  - Typical workflow: save state before reboot, then list/resume later.
  - How to use with Codex/Claude via MCP.
  - Example `session resume` output and "resume script".
- Update WorkMesh skill docs to include session workflows.

Acceptance Criteria:
--------------------------------------------------
- A new user can install from releases and use `session save/list/resume` without reading code.

Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.

Notes:
- Updated README and repo skill to document global agent sessions (WORKMESH_HOME store, session save/list/resume, opt-in auto updates via --auto-session-save/WORKMESH_AUTO_SESSION).