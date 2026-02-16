---
id: task-ws2-002
title: "Phase WS2: clone-to-worktree adoption helper"
kind: task
status: Done
priority: P1
phase: PhaseWS2
dependencies: [task-ws2-001]
labels: [ws2, worktrees, adoption, tooling]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-16 16:50
---
Description:
--------------------------------------------------
- Provide a first-class helper to adopt an existing standalone clone directory into a git worktree
  workflow (backup + `git worktree add`) with safe defaults.
- Support both plan (dry-run) and apply modes.
- Register the created worktree in the global worktree registry.
- Provide CLI + MCP parity.

Acceptance Criteria:
--------------------------------------------------
- CLI: `worktree adopt-clone --from <path> [--to <path>] [--branch <target-branch>] [--allow-dirty] [--apply]`.
- MCP: `worktree_adopt_clone` with equivalent behavior and JSON output.
- Default behavior is dry-run plan output.
- Apply mode backs up the original clone directory before creating the worktree.
- Dirty clones are refused unless explicitly allowed.
- Worktree registry is updated when apply succeeds.

Definition of Done:
--------------------------------------------------
- Tool is safe by default (no mutation unless apply).
- Clear, deterministic output (plan actions) is available to agents and humans.
- Tests cover core behavior where feasible without requiring external repos.
