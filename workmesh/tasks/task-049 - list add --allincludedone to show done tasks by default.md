---
id: task-049
uid: 01KGVARRW963WCSGW41ECY10QE
title: List: add --all/include_done to show Done tasks by default
kind: task
status: Done
priority: P2
phase: Phase4
dependencies: []
labels: [dx, cli, mcp]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-07 00:02
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
- Added archive-aware listing: CLI list supports --all to include tasks under archive/, MCP list_tasks supports all=true. Implemented core loader load_tasks_with_archive and tests in core/cli/mcp. Updated README + repo skill.
