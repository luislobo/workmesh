---
id: task-055
uid: 01KGWR2RNX7CGQHFGR7A4XE0P5
title: Index: stop tracking derived index files; remove absolute paths from tasks index
kind: task
status: Done
priority: P1
phase: Phase4
dependencies: []
labels: [index, dx, ci]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
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
