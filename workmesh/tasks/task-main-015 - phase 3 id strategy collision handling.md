---
dependencies: []
status: Done
updated_date: 2026-02-04 12:14
priority: P2
phase: Phase3
id: task-main-015
title: 'Phase 3: ID strategy + collision handling'
labels:
- phase3
- ids
- core
prd: docs/projects/workmesh/prds/phase-3-agent-graph.md
assignee: []
---
Description:
--------------------------------------------------
- Introduce collision-safe task IDs (ULID or namespaced).
- Handle duplicate IDs gracefully on import/merge.
Acceptance Criteria:
--------------------------------------------------
- 

Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.

Notes:
- Added uid field (ULID) for collision-safe identity; new tasks get uid in front matter; validation warns on duplicate ids when uids unique and errors on duplicate uids; JSON/graph export include uid.