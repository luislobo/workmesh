---
id: task-migr-005
uid: 01KHAXGWSR0TQ8ZKDS3T5SQYMZ
title: Integrate truth context into session/worktree resume flows
kind: task
status: Done
priority: P1
phase: Phase2
dependencies: [task-migr-002, task-migr-003, task-migr-004]
labels: [truth, sessions, worktree]
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
- Integrate accepted truth context into session/worktree continuity flows so resumed work starts from validated decisions.
- Ensure different agent roles/sessions in the same feature/worktree can consume a shared truth baseline.
- Keep this integration additive and backward-compatible with existing context/session behavior.
Acceptance Criteria:
--------------------------------------------------
- Session save/resume pathways can surface relevant accepted truths for current feature/worktree scope.
- Resume guidance includes deterministic references to truth records (not free-form summaries only).
- Tests verify truth context is available across multiple saved sessions bound to the same feature/worktree.
Definition of Done:
--------------------------------------------------
- Task goals in Description are met and all Acceptance Criteria are validated through automated tests.
- Existing session/worktree workflows remain functional without regression.
- Code/config committed.
- Docs updated if needed.
