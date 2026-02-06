---
id: task-037
uid: 01KGT72T1X6VKMP4HE8Z21NTFH
title: Release binaries: add Linux arm64 (aarch64) target
kind: task
status: Done
priority: P2
phase: Phase4
dependencies: []
labels: [ci, release, arm]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-06 13:33
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
- Updated release workflow to build and publish Linux arm64 (aarch64-unknown-linux-gnu) artifacts. Adds gcc-aarch64-linux-gnu linker install on ubuntu runners and sets CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER for builds.
