---
id: task-cif-001
title: 'Phase 0: implement storage primitives and typed errors'
status: To Do
priority: P1
phase: Phase0
dependencies: []
labels: [phase0, concurrency, storage]
assignee: []
kind: task
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
---

Description:
--------------------------------------------------
- Implement `crates/workmesh-core/src/storage.rs` as the canonical storage safety module for Phase 0.
- Add explicit primitives from the approved plan: `with_resource_lock(resource_key, timeout, f)`, `atomic_write_json`, `atomic_write_text`, `append_jsonl_locked`, and `read_modify_write_json`.
- Introduce typed storage errors including `StorageConflict` and timeout-related lock errors.
- Implement lock namespace convention:
  - Repo-local: `<backlog_dir>/.locks/<resource>.lock`
  - Global: `<WORKMESH_HOME>/.locks/<resource>.lock`

Acceptance Criteria:
--------------------------------------------------
- All plan-listed primitives exist with deterministic behavior and unit coverage.
- Lock key to lock-file path mapping is explicit and tested.
- Typed errors are returned and propagated (no stringly-only conflict signaling).
- Atomic writes include durable flush semantics (`fsync` for temp and parent dir).

Definition of Done:
--------------------------------------------------
- Phase 0 primitive contract in the approved plan is fully implemented for this task scope.
- Acceptance criteria are demonstrably met via tests and code review.
- Interfaces are ready to migrate all target write paths without redesign.

Notes:
- This task is the prerequisite root for all migration and doctor/recovery tasks.
