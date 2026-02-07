---
id: task-040
uid: 01KGV3B9P3BSD3M9TM71Q9QYW0
title: Global sessions: core model + storage under ~/.workmesh
kind: task
status: To Do
priority: P1
phase: Phase4
dependencies: []
labels: [sessions, core]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
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
