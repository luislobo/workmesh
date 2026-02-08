---
updated_date: 2026-02-04 12:00
priority: P2
status: Done
assignee: []
prd: docs/projects/workmesh/prds/phase-3-agent-graph.md
labels:
- phase3
- graph
- core
title: 'Phase 3: Dependency relationship model'
phase: Phase3
dependencies: []
id: task-main-011
---
Description:
--------------------------------------------------
- Define relationship types (parent/child/blocked_by/discovered_from).
- Extend task front matter + parsing to support relationships.
Acceptance Criteria:
--------------------------------------------------
- 

Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.

Notes:
- Implemented relationships model in core: new relationships fields (blocked_by/parent/child/discovered_from), parsing from nested or flat front matter, JSON output, and template defaults. Added tests for both formats.