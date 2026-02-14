---
id: task-migr-002
uid: 01KHAXGESA0CC6BSHEZMFR6M9Y
title: Implement truth domain core (events/current model and transition rules)
kind: task
status: Done
priority: P1
phase: Phase1
dependencies: [task-migr-001]
labels: [truth, core]
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
- Implement the core truth domain model and persistence layer (append-only events + current-state projection).
- Enforce transition invariants for truth lifecycle, including supersede semantics and immutable history.
- Keep the core module transport-agnostic so CLI and MCP layers consume the same domain logic.
Acceptance Criteria:
--------------------------------------------------
- Core APIs exist for propose, accept, reject, supersede, and list/query truth records by feature/worktree/session context.
- Invalid transitions are rejected with deterministic, test-covered error paths.
- Unit tests cover happy paths, invalid transitions, projection rebuild behavior, and data compatibility for persisted records.
Definition of Done:
--------------------------------------------------
- Task goals in Description are met and all Acceptance Criteria are satisfied via automated tests.
- Core abstractions are cleanly separated from CLI/MCP adapters and are reusable by both.
- Code/config committed.
- Docs updated if needed.
