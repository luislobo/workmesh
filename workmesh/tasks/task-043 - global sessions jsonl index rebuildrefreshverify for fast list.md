---
id: task-043
uid: 01KGV3B9QPGCJK1D266MB0RJ8Y
title: Global sessions: JSONL index rebuild/refresh/verify for fast list
kind: task
status: To Do
priority: P2
phase: Phase4
dependencies: []
labels: [sessions, index]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
---

Description:
--------------------------------------------------
- Add an optional derived index for sessions (JSONL) for fast list/query:
  - rebuild: scan source JSONL events and materialize the latest session snapshots.
  - refresh: incremental update when possible.
  - verify: detect drift between index and source.

Acceptance Criteria:
--------------------------------------------------
- Index can be rebuilt from scratch from source JSONL.
- Listing can use the index when present, but must fall back to source if missing/stale.

Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.
