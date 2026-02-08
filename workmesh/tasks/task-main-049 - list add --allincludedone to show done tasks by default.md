---
title: 'List: add --all/include_done to show Done tasks by default'
id: task-main-049
uid: 01KGVARRW963WCSGW41ECY10QE
assignee: []
blocked_by: []
updated_date: 2026-02-07 00:02
relationships: []
phase: Phase4
discovered_from: []
kind: task
priority: P2
parent: []
status: Done
dependencies: []
child: []
labels:
- dx
- cli
- mcp
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