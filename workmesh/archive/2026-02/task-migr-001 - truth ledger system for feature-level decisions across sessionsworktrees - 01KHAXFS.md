---
id: task-migr-001
uid: 01KHAXFSR0X3SR12W7B4M6VFD5
title: Truth Ledger system for feature-level decisions across sessions/worktrees
kind: epic
status: Done
priority: P1
phase: Phase1
dependencies: []
labels: [truth, decision-log, multi-agent]
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
- Establish the feature-level Truth Ledger capability so accepted decisions become durable, queryable source-of-truth across sessions, worktrees, and agent roles.
- Define a clear boundary: `context` remains intent/state pointer, while truth records capture validated decisions, constraints, assumptions, and contracts.
- Produce the implementation blueprint that all downstream tasks in this sequence will follow.
Acceptance Criteria:
--------------------------------------------------
- A documented architecture exists for truth events + materialized current state, including identity/versioning and lifecycle states (`proposed`, `accepted`, `superseded`, `rejected`).
- Integration points are explicitly defined for CLI, MCP, session resume output, and worktree-aware workflows.
- Sequenced implementation tasks are created with dependency links and no circular dependencies.
Definition of Done:
--------------------------------------------------
- Task goals in Description are met and all Acceptance Criteria are satisfied and reviewable in-repo.
- The plan is implementable without ambiguous ownership or undefined lifecycle behavior.
- Code/config committed.
- Docs updated if needed.
