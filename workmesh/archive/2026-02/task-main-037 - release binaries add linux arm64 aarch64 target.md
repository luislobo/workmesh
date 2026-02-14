---
dependencies: []
uid: 01KGT72T1X6VKMP4HE8Z21NTFH
status: Done
labels:
- ci
- release
- arm
title: 'Release binaries: add Linux arm64 (aarch64) target'
id: task-main-037
relationships: []
blocked_by: []
phase: Phase4
priority: P2
assignee: []
updated_date: 2026-02-06 13:33
discovered_from: []
kind: task
parent: []
child: []
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