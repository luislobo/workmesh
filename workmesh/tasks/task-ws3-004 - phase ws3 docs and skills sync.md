---
id: task-ws3-004
title: "Phase WS3: docs and skills sync"
kind: task
status: Done
priority: P2
phase: PhaseWS3
dependencies: [task-ws3-001, task-ws3-002, task-ws3-003]
labels: [ws3, docs, skills]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-16 18:28
---
Description:
--------------------------------------------------
- Keep docs and skills consistent with the WS3 command/tool surface changes.
- Ensure `README.md` and `README.json` remain synchronized.

Acceptance Criteria:
--------------------------------------------------
- README and README.json mention:
  - `worktrees_dir`
  - config helper commands
  - `workstream create` default behavior
  - single-stream restore (`workstream show --restore`)
- Command reference includes config commands/tools and updated workstream show syntax.
- Skills mention config helpers and the single-stream restore command.

Definition of Done:
--------------------------------------------------
- Docs are navigable in GitHub with correct relative links.
- Tests remain green.

