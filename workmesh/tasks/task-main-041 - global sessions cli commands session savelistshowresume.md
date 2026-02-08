---
priority: P1
uid: 01KGV3B9PKWMS2YYF7JNXYA54X
updated_date: 2026-02-06 22:07
labels:
- sessions
- cli
- dx
blocked_by: []
dependencies: []
kind: story
assignee: []
relationships: []
discovered_from: []
id: task-main-041
phase: Phase4
title: 'Global sessions: CLI commands session save/list/show/resume'
status: Done
child: []
parent: []
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

Notes:
- Added global session CLI commands: session save/list/show/resume. Persists to WORKMESH_HOME (~/.workmesh), captures best-effort repo context (repo_root/project_id/working set/git/checkpoint). Added CLI integration tests.