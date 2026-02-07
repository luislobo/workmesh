---
id: task-051
uid: 01KGVDZZM7QEZMNYREFMXH0BZY
title: CI: stabilize coverage run (avoid profraw corruption, deterministic env)
kind: task
status: Done
priority: P2
phase: Phase4
dependencies: []
labels: [ci, coverage]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-07 00:53
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
