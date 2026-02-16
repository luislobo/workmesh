# Plan: Phase 0 Concurrency Integrity Foundation (Mandatory Prerequisite)

## Summary

Before any additional multi-agent features land, implement a Concurrency Integrity Foundation that
guarantees tracking-file safety under parallel agents/processes.
This becomes a hard prerequisite gate for all future orchestration additions (workstreams, stronger
guardrails, etc).

## Why this must be first

Current writes to key state are not uniformly lock-safe/atomic, so new multi-agent features would
amplify race conditions.
Phase 0 establishes the storage contract first, then all new features build on it.

## Scope (Phase 0 only)

- Locking + atomic write infrastructure
- Migration of all critical tracking writes to that infrastructure
- Conflict/version semantics
- Recovery + doctor checks
- Tests proving safety under parallel contention

## Non-goals (Phase 0)

- No new user workflow commands yet (workstream feature set waits until this is complete)
- No behavior redesign for task lifecycle beyond storage safety

---

## 1. New Core Storage Safety Module

### New module

- crates/workmesh-core/src/storage.rs

### Public interfaces (new)

- with_resource_lock(resource_key, timeout, f)
  Executes closure under exclusive cross-process lock.
- atomic_write_json(path, value)
  tmp -> fsync(tmp) -> rename -> fsync(parent).
- atomic_write_text(path, text)
  Same atomic protocol for non-JSON files.
- append_jsonl_locked(path, line)
  Exclusive lock + append + flush + fsync.
- read_modify_write_json(path, merge_fn)
  Lock-guarded RMW helper with parse + atomic write.
- VersionedState<T> wrapper:
    - { version: u64, updated_at: String, payload: T }
- cas_update_json(path, expected_version, next_payload)
  Compare-and-swap update under lock.

### Lock namespace convention

- Repo-local: <backlog_dir>/.locks/<resource>.lock
- Global: <WORKMESH_HOME>/.locks/<resource>.lock

---

## 2. Mandatory Migration Targets (write paths)

### Repo-local tracking

- workmesh/context.json (context.rs)
- workmesh/.index/tasks.jsonl (index.rs)
- workmesh/truth/events.jsonl and projection writes (truth.rs)
- workmesh/.audit.log append path (audit.rs)

### Global tracking

- ~/.workmesh/worktrees/registry.json (worktrees.rs)
- ~/.workmesh/sessions/events.jsonl (global_sessions.rs)
- ~/.workmesh/sessions/current.json (global_sessions.rs)
- ~/.workmesh/.index/sessions.jsonl rebuild/refresh paths (global_sessions.rs)

### Rule

No direct fs::write or unlocked append remains on tracking state after Phase 0 (except strictly
derived/rebuildable artifacts where lock policy explicitly allows unlocked rebuild with temp+rename).

---

## 3. Version/Conflict Semantics

### Versioning

- Upgrade mutable snapshot files to include top-level version.
- Increment on each successful write.

### Conflict behavior

- RMW operations require expected version when applicable.
- On mismatch: return typed conflict error (StorageConflict), never silent overwrite.
- CLI/MCP adapters map to deterministic user-facing conflict responses.

### Backward compatibility

- If file lacks version, treat as version 0, migrate in-place on first safe write.

---

## 4. Crash Safety + Recovery

### JSONL robustness

- Reader tolerates trailing malformed partial line.
- Recovery utility trims only trailing invalid lines.
- Never drops valid historical events.

### Doctor extensions

Add storage integrity checks to doctor output:

- lock-path accessibility
- malformed JSONL count
- projection/event divergence
- version monotonicity checks for versioned snapshots

### Recovery command additions

- doctor --fix-storage (CLI)
- MCP equivalent optional follow-up (doctor with fix_storage=true)
- Scope: safe truncation of trailing corrupt JSONL, projection rebuild

---

## 5. Test Plan (Required gate)

### Unit tests

- Atomic write leaves either old or new file, never partial.
- Locked append from N concurrent writers yields N valid JSONL lines.
- CAS update fails on stale version.
- Version migration from unversioned file succeeds.

### Concurrency integration tests

- Parallel claim attempts on same task do not corrupt file.
- Parallel session saves preserve all events.
- Parallel worktree registry updates do not lose records.
- Forced crash simulation during write/restart recovers cleanly.

### Regression tests

- Existing CLI/MCP parity suite remains green.
- Truth/session/index rebuild behavior unchanged functionally.

### Acceptance criteria for Phase 0 done

1. All critical tracking writes use storage primitives.
2. No known lost-update race in tracked paths.
3. Doctor detects and reports storage integrity anomalies.
4. Full test suite passes including new concurrency tests.

---

## 6. Implementation Sequence (Strict)

1. Build storage.rs primitives + typed errors.
2. Migrate global_sessions.rs writes/reads.
3. Migrate worktrees.rs registry writes.
4. Migrate context.rs, truth.rs, audit.rs.
5. Migrate index.rs write path.
6. Add doctor integrity checks + optional fix mode.
7. Add/expand concurrency tests.
8. Freeze Phase 0 with release note.
9. Only then start next feature set (workstreams + richer parallel orchestration).

---

## 7. Documentation updates required in same phase

- README.md + README.json: add "Concurrency Integrity Foundation" guarantees.
- docs/reference/commands.md: document conflict errors and recovery behavior.
- docs/README.md: add storage/integrity policy section.
- Skills docs: add invariant:
    - "Do not bypass storage primitives for tracking files."

---

## Assumptions and Defaults

- Locking backend: file-based cross-process locks (portable crate-backed).
- Lock timeout default: short bounded wait (for example 5s), configurable later.
- Tracking files are local filesystem (no remote DB introduced).
- Existing command surface remains backward-compatible where possible; conflicts become explicit
  errors instead of silent overwrite.
