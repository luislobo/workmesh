---
id: task-cif-003
title: 'Phase 0: migrate global sessions write paths to storage primitives'
status: Done
priority: P1
phase: Phase0
dependencies: [task-cif-001, task-cif-002]
labels: [phase0, concurrency, sessions]
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
- Migrate all global sessions writes in `global_sessions.rs` to the new storage module primitives.
- Cover target files:
  - `~/.workmesh/sessions/events.jsonl`
  - `~/.workmesh/sessions/current.json`
  - `~/.workmesh/.index/sessions.jsonl` rebuild/refresh paths
- Enforce lock-safe append and atomic snapshot/index writes.

Acceptance Criteria:
--------------------------------------------------
- No direct unlocked writes remain on global session tracking files.
- Session event append is lock-safe under concurrent writers.
- Current pointer and index writes are atomic and version-aware where applicable.
- Concurrency tests validate no event loss under parallel session saves.

Definition of Done:
--------------------------------------------------
- Global session storage behavior matches Phase 0 safety guarantees.
- Acceptance criteria are met with passing unit/integration tests.
- No regression in existing session CLI/MCP workflows.

Notes:
- Migrated global session writes to storage primitives with global lock keys: sessions/events.jsonl append_jsonl_locked_with_key, sessions/current.json CAS versioned write, .index/sessions.jsonl atomic write under lock. Added parallel append test and tolerant/recovery tests.
- This is implementation sequence step 2.
