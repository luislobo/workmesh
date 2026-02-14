---
phase: Phase4
labels:
- ci
- coverage
- dx
child: []
blocked_by: []
relationships: []
parent: []
dependencies: []
id: task-main-048
title: 'CI: add coverage measurement + run full workspace tests'
updated_date: 2026-02-06 23:54
kind: task
assignee: []
discovered_from: []
uid: 01KGVAM68Q7YN23EXKE65D3F3G
status: Done
priority: P1
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