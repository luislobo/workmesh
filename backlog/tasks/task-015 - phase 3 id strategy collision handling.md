---
id: task-015
title: Phase 3: ID strategy + collision handling
status: Done
priority: P2
phase: Phase3
dependencies: []
labels: [phase3, ids, core]
assignee: []
prd: docs/projects/workmesh/prds/phase-3-sync-and-graph.md
updated_date: 2026-02-03 18:30
---
Description:
--------------------------------------------------
- Introduce collision-safe task IDs (ULID or namespaced).
- Handle duplicate IDs gracefully on sync/import.
Acceptance Criteria:
--------------------------------------------------
- 

Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.

Notes:
- Added uid field (ULID) for collision-safe identity; new tasks get uid in front matter; validation warns on duplicate ids when uids unique and errors on duplicate uids; JSON/graph export include uid.
