---
kind: task
phase: Phase4
status: Done
labels:
- sessions
- index
uid: 01KGV3B9QPGCJK1D266MB0RJ8Y
title: 'Global sessions: JSONL index rebuild/refresh/verify for fast list'
parent: []
child: []
blocked_by: []
id: task-main-043
dependencies: []
updated_date: 2026-02-06 22:16
relationships: []
assignee: []
priority: P2
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

Notes:
- Implemented global sessions JSONL index under WORKMESH_HOME/.index/sessions.jsonl with rebuild/refresh/verify. Session list/show/resume use the index when present and fall back to source events.