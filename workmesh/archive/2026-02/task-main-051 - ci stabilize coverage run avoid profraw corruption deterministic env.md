---
blocked_by: []
title: 'CI: stabilize coverage run (avoid profraw corruption, deterministic env)'
id: task-main-051
discovered_from: []
assignee: []
phase: Phase4
updated_date: 2026-02-07 00:53
uid: 01KGVDZZM7QEZMNYREFMXH0BZY
labels:
- ci
- coverage
child: []
status: Done
parent: []
relationships: []
kind: task
priority: P2
dependencies: []
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
- Coverage job now sets CARGO_TARGET_DIR and LLVM_PROFILE_FILE to a dedicated dir (target/llvm-cov-ci), creating profraw dir first. This avoids profraw collisions/corruption and makes coverage runs deterministic.