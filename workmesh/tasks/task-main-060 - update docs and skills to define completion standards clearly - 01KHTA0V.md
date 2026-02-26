---
id: task-main-060
uid: 01KHTA0VT74DEH52PWF9XPN3SG
title: Update docs and skills to define completion standards clearly
kind: task
status: Done
priority: P2
phase: Phase5
dependencies: [task-main-059]
labels: [phase5, docs, skills, quality]
assignee: [luis]
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-26 09:50
---

Description:
--------------------------------------------------
- Update user docs and agent skills to state task quality standards clearly.
- Keep `README.md` and `README.json` synchronized with the new completion policy and migration behavior.
- Document that meeting description goals and acceptance criteria is mandatory for completion.

Acceptance Criteria:
--------------------------------------------------
- `README.md` and `README.json` both include task quality policy and are consistent.
- Command reference docs describe Done-gate behavior and conflict semantics.
- Getting-started and docs index include quality expectations for task lifecycle usage.
- WorkMesh skills include the same completion standards and parity expectation.

Definition of Done:
--------------------------------------------------
- Documentation and skills communicate the same completion policy without contradictions.
- Users and agents can discover completion requirements from primary docs.
- Code/config committed.
- Docs updated where behavior/policy changed.
