---
title: 'Global sessions: opt-in auto session save (flag/env)'
dependencies: []
discovered_from: []
priority: P2
blocked_by: []
id: task-main-044
parent: []
child: []
uid: 01KGV3B9R7KRPM54C3HMW7ZMA6
kind: task
phase: Phase4
status: Done
labels:
- sessions
- dx
updated_date: 2026-02-06 22:25
relationships: []
assignee: []
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

Notes:
- Implemented opt-in auto session updates: CLI flag --auto-session-save and env WORKMESH_AUTO_SESSION=1. When enabled, mutating commands update the current global session pointer with repo_root/project_id/working_set + git and checkpoint/recent_changes best-effort.