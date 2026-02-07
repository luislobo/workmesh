---
id: task-041
uid: 01KGV3B9PKWMS2YYF7JNXYA54X
title: Global sessions: CLI commands session save/list/show/resume
kind: story
status: To Do
priority: P1
phase: Phase4
dependencies: []
labels: [sessions, cli, dx]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
---

Description:
--------------------------------------------------
- Implement CLI commands:
  - `workmesh session save`
  - `workmesh session list`
  - `workmesh session show`
  - `workmesh session resume`
- Ensure deterministic ordering (e.g., updated_at desc, then id).
- `session resume` should print a concise summary plus a suggested "resume script"
  (cd, then recommended WorkMesh commands).

Acceptance Criteria:
--------------------------------------------------
- Commands work outside a repo; repo_root discovery is best-effort.
- All commands support `--json` for agent-friendly use.
- Output is stable and test-covered (CLI integration tests).

Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.
