---
id: task-main-033
uid: 01KGSAEQ62W4A559CR56NG8NJT
title: Add task kind (Jira-friendly) + filtering
status: Done
priority: P2
phase: Phase4
dependencies: []
labels:
- schema
- dx
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-06 05:18
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
- Implemented kind field on Task (default: task). Added CLI+MCP filtering (list --kind / list_tasks.kind). Included kind in graph export and JSON/JSONL exports. Documented Jira-friendly suggested values in README.