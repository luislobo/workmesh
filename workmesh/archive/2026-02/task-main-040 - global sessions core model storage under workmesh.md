---
updated_date: 2026-02-06 22:01
parent: []
dependencies: []
status: Done
title: 'Global sessions: core model + storage under ~/.workmesh'
assignee: []
priority: P1
phase: Phase4
uid: 01KGV3B9P3BSD3M9TM71Q9QYW0
id: task-main-040
labels:
- sessions
- core
blocked_by: []
child: []
relationships: []
kind: task
discovered_from: []
---
Description:
--------------------------------------------------
- Add a new global storage root (default: `~/.workmesh/`) and a session record model.
- Persist sessions as JSONL events under `~/.workmesh/sessions/` (or config override).
- Capture best-effort contextual fields: cwd, repo_root, project_id, objective, working_set,
  git snapshot, latest checkpoint reference, recent changes.

Acceptance Criteria:
--------------------------------------------------
- Global storage path resolution is deterministic and test-covered.
- Session records are collision-safe (ULID).
- Session writes do not require being inside a repo; repo_root/project_id are optional.

Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.

Notes:
- Implemented global sessions core module + storage + tests.