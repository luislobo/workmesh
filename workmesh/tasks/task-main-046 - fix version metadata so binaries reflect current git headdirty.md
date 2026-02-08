---
id: task-main-046
uid: 01KGV5SYMAC5ZSKC6PGGEM4JRJ
title: Fix version metadata so binaries reflect current git HEAD/dirty
kind: task
status: Done
priority: P2
phase: Phase4
dependencies: []
labels:
- dx
- build
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-06 22:31
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
- Fixed Rust build.rs version metadata: rerun-if-changed now watches the real git HEAD/index (works from crate subdirs). Version now includes current git sha and marks dirty when git status is non-empty. Added unit tests to assert FULL includes current sha.