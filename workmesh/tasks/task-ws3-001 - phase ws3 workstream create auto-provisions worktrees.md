---
id: task-ws3-001
title: "Phase WS3: workstream create auto-provisions worktrees"
kind: task
status: Done
priority: P1
phase: PhaseWS3
dependencies: []
labels: [ws3, workstreams, worktrees]
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
- Improve day-2 workflow by making it possible to start a new workstream from the canonical checkout
  without manually specifying a worktree path/branch every time.
- Use safe defaults:
  - only auto-provision when the repo has a real `HEAD` commit
  - respect effective `worktrees_default`
  - do not auto-provision when already inside a non-canonical git worktree checkout

Acceptance Criteria:
--------------------------------------------------
- CLI `workstream create --name "..."` auto-provisions a git worktree when:
  - invoked from the canonical checkout
  - `worktrees_default=true`
  - and the repo has a real `HEAD` commit
- Worktree path is deterministic (config override + fallback default).
- Branch base is deterministic and deduped as needed.
- MCP `workstream_create` follows the same behavior.

Definition of Done:
--------------------------------------------------
- Tests are green.
- Docs are updated to explain the behavior and how to override it.

