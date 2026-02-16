---
id: task-cif-005
title: 'Phase 0: migrate repo-local tracking write paths'
status: Done
priority: P1
phase: Phase0
dependencies: [task-cif-001, task-cif-002]
labels: [phase0, concurrency, repo-local]
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
- Migrate repo-local critical tracking writes to storage primitives:
  - `workmesh/context.json` (`context.rs`)
  - `workmesh/.index/tasks.jsonl` (`index.rs`)
  - `workmesh/truth/events.jsonl` + projection writes (`truth.rs`)
  - `workmesh/.audit.log` append (`audit.rs`)
- Ensure no direct `fs::write` or unlocked append remains on these paths after migration.

Acceptance Criteria:
--------------------------------------------------
- All listed files use centralized storage primitives for write paths.
- Truth events/projection and index writes preserve functional behavior.
- Concurrent operations do not corrupt tracking artifacts.
- Rule from the plan is enforced in code and tests.

Definition of Done:
--------------------------------------------------
- All repo-local critical paths satisfy Phase 0 safety policy.
- Acceptance criteria are met and validated by tests.
- Behavior remains backward-compatible except explicit conflict surfacing.

Notes:
- Migrated repo-local critical writes: context save/clear use repo-local lock + CAS; index write uses repo-local lock + atomic write; truth events append and projection rebuild/write use storage primitives and shared lock key; audit append uses strict JSONL append with repo-local key.
- This task consolidates implementation sequence steps 4 and 5.
