---
id: task-migr-007
uid: 01KHAXGWT47FVCBDG41VNTVQR3
title: Document truth workflow in README, commands reference, and skills
kind: task
status: Done
priority: P2
phase: Phase2
dependencies: [task-migr-003, task-migr-004, task-migr-005, task-migr-006]
labels: [truth, docs, skills]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-13 09:22
---
Description:
--------------------------------------------------
- Document the truth workflow and operational guidance for humans and agents across README, command reference, and skills.
- Clarify role separation between context, truth records, sessions, and worktrees.
- Provide examples for proposing/accepting/superseding truths in multi-agent feature development.
Acceptance Criteria:
--------------------------------------------------
- README.md and README.json are updated in sync with truth commands/tools, concepts, and examples.
- docs/reference/commands.md includes complete CLI/MCP truth command coverage.
- Relevant skills reference truth workflows and expected Definition-of-Done standards.
Definition of Done:
--------------------------------------------------
- Task goals in Description are met and all Acceptance Criteria are satisfied with no doc/source contradictions.
- Documentation is sufficient for a new agent session to execute the truth workflow without tribal knowledge.
- Code/config committed.
- Docs updated if needed.
