---
id: task-cif-004
title: 'Phase 0: migrate worktree registry write paths to storage primitives'
status: To Do
priority: P1
phase: Phase0
dependencies: [task-cif-001, task-cif-002]
labels: [phase0, concurrency, worktrees]
assignee: []
kind: task
relationships:
  blocked_by: [task-cif-001, task-cif-002]
  parent: []
  child: []
  discovered_from: []
---

Description:
--------------------------------------------------
- Migrate `~/.workmesh/worktrees/registry.json` writes in `worktrees.rs` to lock-safe RMW + atomic write.
- Apply CAS/version semantics where mutable snapshots are updated.
- Ensure concurrent upsert/remove operations cannot lose records.

Acceptance Criteria:
--------------------------------------------------
- Registry writes are fully lock-guarded and atomic.
- Parallel update tests demonstrate no lost updates.
- Version/conflict handling is deterministic and typed.
- Existing `worktree` command behavior remains functionally consistent.

Definition of Done:
--------------------------------------------------
- Worktree registry path satisfies Phase 0 tracking safety contract.
- Acceptance criteria are met and tested.
- No behavior regression in `worktree list/create/attach/detach/doctor` flows.

Notes:
- This is implementation sequence step 3.
