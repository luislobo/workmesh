# WorkMesh PRD: Phase 1 Workstream Orchestration

Date: 2026-02-16
Owner: WorkMesh
Status: Draft

## Summary

Phase 0 delivered storage safety and recovery guarantees.  
Phase 1 builds the orchestration layer for real parallel development streams, centered on a first-class `workstream` model that ties together:

- objective and scope
- worktree and branch
- active session pointers
- task set
- truth references

Goal: make parallel work deterministic to start, track, pause, and resume across reboots and multiple agents.

## Problem

Current capabilities are strong but fragmented across context, sessions, worktrees, and truth. Users can still lose mental state across many concurrent terminals/agents.

## Goals

1. Introduce first-class `workstream` runtime model for feature streams.
2. Provide predictable resume flow after reboot (all active streams).
3. Keep CLI/MCP command behavior aligned.
4. Preserve Phase 0 storage guarantees for all new tracking paths.

## Non-goals

1. No remote database.
2. No external integration dependency.
3. No redesign of task lifecycle semantics in this phase.

## Proposed Scope

1. `workstream` registry (versioned/CAS, lock-safe).
2. CLI commands:
   - `workstream list`
   - `workstream create`
   - `workstream show`
   - `workstream switch`
   - `workstream doctor`
3. MCP parity for the same command set.
4. Session/worktree auto-linking into active workstream.
5. Resume workflow:
   - enumerate active streams
   - per-stream restore hints (path, session, context, next task)
6. Documentation + skills guidance for stream-based operation.

## Workstream Domain Contract (Model + Ownership)

This section is the Phase 1 contract. Implementation should follow this contract closely to avoid redesign churn.

### Glossary

- Repo root: the repository root directory for the current checkout (worktree).
- Worktree: a git worktree directory (may be the main checkout).
- Stream / workstream: a durable WorkMesh record that represents one parallel line of work in a repo.
- Active workstream: the workstream associated with the current worktree's runtime context.
- Context: the local, repo-scoped intent/scope pointer (`workmesh/context.json`).
- Session: a global continuity record (`$WORKMESH_HOME/sessions/`).
- Truth: durable decision record (`workmesh/truth/`).

### Workstream Registry

#### Storage location (source of truth)

- Registry is global, per-user, and cross-repo:
  - `$WORKMESH_HOME/workstreams/registry.json`
- Rationale:
  - Workstreams must be visible from any worktree checkout of the same repo.
  - The registry must not be a tracked git file (it contains local paths and session pointers).
  - A global registry can store multiple repos by including `repo_root` per record (same pattern as worktrees registry).

#### Registry shape

Workstream registry is a versioned snapshot protected by Phase 0 CAS semantics and locks.

Top-level:
- `version` (u32): schema version (currently `1`).
- `workstreams` (array): all known workstreams across repos.

Workstream record (field-level contract):
- `id` (string): stable unique id (ULID). Primary key.
- `repo_root` (string): canonicalized absolute path string to the repo root this stream belongs to.
- `key` (optional string): short human-friendly key for daily use (e.g. `oca`, `tapestry-upgrade`).
  - Uniqueness: must be unique within a given `repo_root` (case-insensitive).
  - If not provided, the CLI will derive a key from `name` and deduplicate.
- `name` (string): human label (free text). Not required to be unique.
- `status` (string enum): `active` | `paused` | `closed`.
  - `active`: expected to be in use (normal default).
  - `paused`: intentionally inactive but not closed.
  - `closed`: completed or abandoned; kept for historical restore/search.
- `created_at` (RFC3339 string): creation time.
- `updated_at` (RFC3339 string): updated on every successful mutation.
- `worktree` (optional object): preferred worktree binding.
  - `id` (optional string): WorkMesh worktree registry id when known.
  - `path` (string): absolute path to worktree directory.
  - `branch` (optional string): expected branch name (best-effort, informational).
- `session_id` (optional string): last-known global agent session id for this workstream.
- `context` (optional object): persisted context snapshot for this workstream.
  - Shape: matches `ContextState` payload fields:
    - `project_id` (optional string)
    - `objective` (optional string)
    - `scope.mode` (`none` | `epic` | `tasks`)
    - `scope.epic_id` (optional string)
    - `scope.task_ids` (array of strings)
  - Ownership: this is the durable per-stream context (used when switching/restoring).
- `truth_refs` (array of strings): list of truth ids associated with this stream (convenience pointer only).
  - Ownership: truth records are canonical; this list is non-authoritative and may be rebuilt.
- `notes` (optional string): free text, small. For handoff reminders.

Normalization rules:
- Paths are stored as canonicalized strings where possible and compared case-insensitively on Windows.
- `repo_root` must be normalized consistently with existing `worktrees` registry normalization rules.
- `key` comparisons are case-insensitive; canonical stored form should be lower-case.

Concurrency contract:
- All registry writes must use `workmesh-core::storage` CAS update helpers with a global registry lock key.
- Lost updates are not acceptable; concurrent writers must preserve all records.

### Active Workstream Pointer (Context)

`workmesh/context.json` remains the primary orchestration state for commands that scope work by objective and tasks.

To support multiple parallel streams, context gains one additional responsibility:
- identify which workstream is active for the current worktree checkout.

Contract:
- `ContextState` payload gains an optional `workstream_id` (string).
  - When present, it is the active workstream for this worktree.
  - When absent, the worktree has no active workstream and behavior remains unchanged.
- When `workstream_id` is set:
  - `context set` / other context mutations should also persist the updated context snapshot into the referenced workstream record.
  - `workstream switch` should update context.json (objective + scope) from the workstream's stored `context` snapshot.

Rationale:
- A per-worktree pointer avoids global "current workstream" contention when multiple terminals are active.
- Existing commands continue to work against `context.json` without needing a redesign in Phase 1.

### Source-of-Truth Ownership Matrix

This is the "who owns what" contract to avoid ambiguous state duplication:

- `workmesh/context.json` (repo-local, per worktree):
  - Owns: active objective + scope for commands like `next`, `board`, `blockers`, and the active `workstream_id` pointer.
  - Does not own: the list of all workstreams for a repo.

- `$WORKMESH_HOME/workstreams/registry.json` (global):
  - Owns: the set of workstreams and durable per-stream pointers (worktree path, last session id, per-stream context snapshot).
  - Does not own: truth record contents, session event history, git worktree existence.

- `$WORKMESH_HOME/worktrees/registry.json` (global):
  - Owns: known worktree records and attached session id pointers.
  - Does not own: stream membership (a worktree can exist without being part of a workstream).

- `$WORKMESH_HOME/sessions/events.jsonl` and `$WORKMESH_HOME/sessions/current.json` (global):
  - Owns: session continuity history and the last-used session pointer (global).
  - Does not own: per-stream state (the workstream record stores `session_id`).

- `workmesh/truth/` (repo-local):
  - Owns: truth event history and projections.
  - Does not own: workstream identity (workstream may reference truth ids; truth may optionally include feature tags).

Derived/rebuildable:
- `workmesh/.index/*` and `$WORKMESH_HOME/.index/*` remain derived indexes and must never be treated as authoritative state.

## Command Contracts (CLI + MCP)

This section freezes Phase 1 behavior and error semantics. CLI and MCP must behave consistently.

Common conventions:
- IDs:
  - `id` is the canonical identifier.
  - `key` may be accepted where unambiguous within the repo; ambiguity must be an error (no silent selection).
- JSON mode:
  - CLI: `--json` returns a JSON value that matches MCP tool output shape as closely as possible.
  - MCP: tools return JSON text (string) containing a JSON value, consistent with existing tools.
- Side effects:
  - Any command that changes durable state must use Phase 0 storage primitives only.
  - Commands must not implicitly write derived indexes except via existing index-refresh policy.

### `workstream list`

Purpose:
- List known workstreams for the current repo root.

Behavior:
- Returns all records where `repo_root` matches current repo root.
- Indicates which workstream is active for the current worktree (based on `context.json.workstream_id`).
- Includes worktree path and branch (best effort).

JSON shape (array of views):
- `id`, `key`, `name`, `status`
- `active` (bool)
- `worktree_path` (optional string)
- `branch` (optional string)
- `session_id` (optional string)
- `updated_at`

### `workstream show [<id-or-key>]`

Purpose:
- Show one workstream.

Behavior:
- If an argument is provided: resolve by id first, then by key.
- If no argument is provided:
  - show the active workstream for this worktree if set in context
  - otherwise return a "no active workstream" error
- Includes a computed `issues` list (missing worktree path, missing session id target, etc).

### `workstream create`

Purpose:
- Create a new workstream record and (optionally) provision a new git worktree for it.

Contract inputs (CLI flags; MCP args mirror):
- `--name "..."` (required)
- `--key <key>` (optional; derived if omitted)
- Optional worktree provisioning:
  - `--path <path>` and `--branch <branch>` (optional)
  - `--from <ref>` (optional starting ref)
- Optional initial context seed:
  - `--project <pid>`
  - `--objective "..."`
  - `--epic <task-id>` OR `--tasks <csv>`

Behavior:
- Always creates or updates a workstream record in the global registry.
- If `--path` + `--branch` provided:
  - creates a git worktree (delegates to the same core APIs as `worktree create`)
  - creates/updates a worktree registry record
  - seeds the new worktree with `context.json` including:
    - initial objective/scope (from flags or inferred)
    - `workstream_id` set to the created workstream id
  - returns the worktree path in output so users can open a terminal there
- If worktree provisioning is omitted:
  - binds the workstream to the current working directory as its worktree path (best effort)
  - sets/updates `context.json.workstream_id` in the current worktree

### `workstream switch <id-or-key>`

Purpose:
- Switch the current worktree's active workstream and restore its scoped context.

Behavior:
- Resolves target workstream (id first, then key).
- Writes `workmesh/context.json` in the current worktree:
  - sets `workstream_id` to the selected stream id
  - restores objective/scope from the workstream record's stored `context` snapshot (if present)
  - if no `context` snapshot is present, keeps existing objective/scope unchanged
- Updates the global current session pointer to the stream's `session_id` (if present).
  - This is a convenience only; it must not be relied on as per-stream state.
- Returns the selected stream and recommended `worktree_path` so the caller can open a terminal there when needed.

### `workstream doctor`

Purpose:
- Diagnose workstream registry health for the current repo root.

Behavior:
- Verifies:
  - all workstreams reference a valid repo_root
  - referenced worktree paths exist
  - referenced session ids exist (best-effort validation via sessions index/events)
  - active workstream pointer in context.json refers to a known stream
- Does not mutate state by default.
- May offer fix suggestions (future enhancement), but Phase 1 doctor is primarily diagnostic.

## Compatibility and Migration Expectations

Backwards compatibility contract:
- If the workstream registry does not exist, all existing commands behave as today.
- Workstreams are additive; no existing workflow becomes mandatory in Phase 1.

Existing repos:
- Repos without `$WORKMESH_HOME` state:
  - `bootstrap`/`doctor` already create global dirs as needed.
- Repos with existing `context.json` but no `workstream_id`:
  - treated as "no active workstream".
  - `workstream create` and `workstream switch` are the only ways to set `workstream_id` by default.

Migration from clone-based parallel work (user-managed multiple clones):
- WorkMesh will not attempt to merge clones automatically.
- Phase 1 documentation and skills should recommend converting clones into worktrees:
  - one canonical repo root
  - N worktrees as feature streams
- Worktree doctor can help detect stale/missing registry entries during this transition.

## Storage And Safety Invariants

1. New workstream tracking files must use `workmesh-core::storage` primitives only.
2. Mutable snapshots must be versioned and CAS-updated.
3. Event/rebuild flows must tolerate trailing malformed JSONL where applicable.
4. `doctor` must surface integrity signals and safe remediation scope.

## Acceptance Criteria

1. User can manage N parallel feature streams from one canonical repo/worktree setup.
2. User can restore active stream state after reboot using deterministic commands.
3. CLI/MCP parity tests pass for workstream commands.
4. Concurrency tests show no lost updates/corruption in workstream tracking.
5. Docs and skills clearly recommend and explain workstream-based workflow.

## Risks

1. Command-surface sprawl.
2. Ambiguity with existing context/session commands.
3. Potential duplication of registry/state unless contracts are explicit.

## Mitigations

1. Keep command set small and role-specific.
2. Define source-of-truth ownership for each state file.
3. Enforce storage invariants in code review, tests, and docs.
