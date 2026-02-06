---
id: task-036
uid: 01KGT6ST76ZTKRQ6GP31HR7J4A
title: GitHub Actions: CI + cross-platform release binaries
kind: task
status: Done
priority: P2
phase: Phase4
dependencies: []
labels: [ci, release, dx]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-06 13:28
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
