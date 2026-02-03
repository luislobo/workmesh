---
id: task-008
title: Phase 1: Rust parity tests + fixtures
status: In Progress
priority: P1
phase: Phase1
dependencies: [task-003]
labels: [phase1, rust, tests]
assignee: []
prd: docs/projects/workmesh/prds/phase-1-conversion.md
updated_date: 2026-02-03 16:29
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
