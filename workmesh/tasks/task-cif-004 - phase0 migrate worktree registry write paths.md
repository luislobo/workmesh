---
id: task-cif-004
title: 'Phase 0: migrate worktree registry write paths to storage primitives'
status: Done
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
updated_date: 2026-02-15 23:43
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
- Migrated worktree registry writes to CAS + global lock key worktrees.registry. load path supports legacy and versioned formats. Added concurrent upsert test to verify no lost updates.
- This is implementation sequence step 3.
