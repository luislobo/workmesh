---
kind: task
labels:
- index
- dx
- ci
priority: P1
status: Done
relationships: []
title: 'Index: stop tracking derived index files; remove absolute paths from tasks index'
parent: []
phase: Phase4
dependencies: []
assignee: []
child: []
discovered_from: []
blocked_by: []
uid: 01KGWR2RNX7CGQHFGR7A4XE0P5
id: task-main-055
updated_date: 2026-02-07 13:29
---
Description:
--------------------------------------------------
- Ensure derived task indexes never leak absolute local paths (like `/home/user/...`).
- Keep derived index files untracked and churn-free in git.

Acceptance Criteria:
--------------------------------------------------
- `workmesh/.index/tasks.jsonl` entries use repo-relative `path` values (e.g. `workmesh/tasks/...`).
- `index-refresh` upgrades an old index (with absolute paths) into a new index with repo-relative paths.
- Index directory remains ignored and untracked.

Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.

Notes:
- Index entries now store repo-relative paths (no /home/...); refresh/verify operate on normalized relative keys. Added test to prevent regression.