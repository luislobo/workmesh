---
id: task-045
uid: 01KGV3B9RRCRDS1GB7F6G910CB
title: Docs: global sessions workflow + examples
kind: task
status: To Do
priority: P2
phase: Phase4
dependencies: []
labels: [sessions, docs]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
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
