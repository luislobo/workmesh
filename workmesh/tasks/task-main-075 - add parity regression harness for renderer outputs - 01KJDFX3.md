---
id: task-main-075
uid: 01KJDFX3PR6NTCE4TT5P214E85
title: Add parity regression harness for renderer outputs
kind: task
status: Done
priority: P1
phase: Phase7
dependencies: [task-main-074]
labels: [phase7, testing, regression]
assignee: [luis]
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-26 11:40
---
Description:
--------------------------------------------------
- Build a regression harness to protect render output stability and detect behavior drift.
- Add fixture-based tests for representative inputs across all renderer tools.
- Make parity deltas explicit when behavior intentionally differs from legacy Node implementation.
Acceptance Criteria:
--------------------------------------------------
- Regression fixtures exist for each render tool with deterministic assertions.
- CI test path fails on unintended output regressions.
- Any intentional deltas are documented in test cases or migration notes.
Definition of Done:
--------------------------------------------------
- Regression harness reliably protects renderer output contracts.
- Test suite coverage is sufficient to detect common regressions.
- Description goals are met and all Acceptance Criteria are satisfied.
- Code/config committed.
- Docs updated if needed.
