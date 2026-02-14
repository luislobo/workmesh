---
relationships: []
discovered_from: []
status: Done
updated_date: 2026-02-06 13:28
blocked_by: []
assignee: []
phase: Phase4
title: 'GitHub Actions: CI + cross-platform release binaries'
priority: P2
labels:
- ci
- release
- dx
uid: 01KGT6ST76ZTKRQ6GP31HR7J4A
kind: task
parent: []
child: []
dependencies: []
id: task-main-036
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
- Added GitHub Actions workflows: CI (tests on ubuntu/macos/windows) and Release (on tag v* builds + packages workmesh/workmesh-mcp for linux x86_64, macos x86_64+arm64, windows x86_64 and uploads assets to GitHub Release).