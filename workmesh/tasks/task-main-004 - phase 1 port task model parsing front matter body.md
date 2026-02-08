---
updated_date: 2026-02-01 18:13
status: Done
phase: Phase1
title: 'Phase 1: Port task model + parsing (front matter, body)'
assignee: []
id: task-main-004
prd: docs/projects/workmesh/prds/phase-1-conversion.md
priority: P1
dependencies:
- task-main-003
labels:
- phase1
- rust
- core
---
Description:
--------------------------------------------------
- 

Acceptance Criteria:
--------------------------------------------------
- 

Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.

Notes:
- Implemented core task ops (filter/sort/validate/update/notes/sections) with tests; ready for CLI wiring.
- Added backlog root detection + initial task parsing model in workmesh-core; next: parsing utilities, filtering, and update ops.