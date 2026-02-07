---
id: task-048
uid: 01KGVAM68Q7YN23EXKE65D3F3G
title: CI: add coverage measurement + run full workspace tests
kind: task
status: Done
priority: P1
phase: Phase4
dependencies: []
labels: [ci, coverage, dx]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-06 23:54
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
- CI now runs cargo test --workspace on PR/push for ubuntu/macos/windows. Added ubuntu-only coverage job using cargo-llvm-cov + llvm-tools-preview, producing lcov.info artifact.
