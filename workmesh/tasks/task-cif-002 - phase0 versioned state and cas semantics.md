---
id: task-cif-002
title: 'Phase 0: versioned state and CAS semantics'
status: To Do
priority: P1
phase: Phase0
dependencies: [task-cif-001]
labels: [phase0, concurrency, versioning]
assignee: []
kind: task
relationships:
  blocked_by: [task-cif-001]
  parent: []
  child: []
  discovered_from: []
---

Description:
--------------------------------------------------
- Implement `VersionedState<T>` wrapper with `version`, `updated_at`, and `payload`.
- Add `cas_update_json(path, expected_version, next_payload)` semantics under lock.
- Define backward-compat migration: unversioned snapshots treated as `version = 0`, migrated in-place on first safe write.
- Ensure conflict mismatches return typed `StorageConflict` errors and are not silently overwritten.

Acceptance Criteria:
--------------------------------------------------
- Versioned wrapper is available and used by mutable snapshot files in-scope for Phase 0.
- CAS rejects stale versions consistently with a typed conflict error.
- Unversioned legacy snapshots migrate without data loss.
- Unit tests cover CAS success, CAS stale conflict, and version migration behavior.

Definition of Done:
--------------------------------------------------
- Version/conflict semantics from the approved plan are implemented and test-backed.
- Acceptance criteria are met and verified with deterministic tests.
- API behavior is stable for CLI/MCP adapters to present explicit conflict responses.

Notes:
- This task defines the anti-lost-update contract for multi-agent writes.
