---
priority: P1
prd: docs/projects/workmesh/prds/phase-1-conversion.md
dependencies:
- task-main-004
updated_date: 2026-02-03 16:33
labels:
- phase1
- rust
- tests
phase: Phase1
assignee: []
title: 'Phase 1: Rust parity tests + fixtures'
id: task-main-009
status: Done
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
- Added CLI parity tests with sample backlog fixtures; covers list/show/next behavior. CLI tests run via cargo test -p workmesh.
- Adjusted validation parity: dependencies optional (warning only) + added test coverage.