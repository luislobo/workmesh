---
id: task-044
uid: 01KGV3B9R7KRPM54C3HMW7ZMA6
title: Global sessions: opt-in auto session save (flag/env)
kind: task
status: To Do
priority: P2
phase: Phase4
dependencies: []
labels: [sessions, dx]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
---

Description:
--------------------------------------------------
- Add opt-in automation to keep the global session up to date:
  - CLI: `--auto-session-save`
  - Env: `WORKMESH_AUTO_SESSION=1`
- When enabled, mutating commands should best-effort update the session record
  (cwd/repo_root/working set/last checkpoint).

Acceptance Criteria:
--------------------------------------------------
- Default remains off.
- Automation never blocks the primary command; failures are non-fatal.

Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.
