# WorkMesh PRD: Phase WS2 - Workstream Lifecycle + Clone-to-Worktree Adoption

Date: 2026-02-16
Owner: Luis Lobo
Status: Implemented

## Problem

Phase 1 introduced workstreams as durable, global records that tie together worktree path, last
known session id, and a per-stream context snapshot.

Two gaps remain for real day-to-day use:

1. Workstreams need a lifecycle (pause/close/reopen/rename/set) so streams can be intentionally
   managed without editing registry files.
2. Users who currently keep multiple full clones (one per "stream") need a safe adoption path
   into git worktrees so they can stop paying the cost of multiple clones and recover streams
   deterministically after reboot.

## Goals

- Add workstream lifecycle commands with CLI + MCP parity.
- Make `context set` keep the active workstream's context snapshot updated (best-effort).
- Provide a safe, deterministic "clone -> worktree" adoption helper:
  - Generates a plan (dry-run by default).
  - Applies the plan only when explicitly requested.
  - Preserves safety by backing up the original clone directory before creating the worktree.
- Make Truth Ledger easy to attach to the current workstream (and discover from `workstream show`).

## Non-goals

- No external system integrations.
- No UI or background daemons.
- No redesign of task lifecycle beyond the new workstream lifecycle surface.

## Requirements

### Workstream lifecycle (CLI + MCP)

Commands/tools:
- `workstream pause [<id-or-key>]`
- `workstream close [<id-or-key>]`
- `workstream reopen [<id-or-key>]`
- `workstream rename [<id-or-key>] --name "..." `
- `workstream set [<id-or-key>] [--key ...] [--notes ...] [--project ...] [--epic ...] [--objective ...] [--tasks "..."]`

Contracts:
- Commands default to the active workstream when no id/key is provided.
- `pause` and `close` clear the repo-local active pointer (`context.json.workstream_id`) when the
  paused/closed stream was active in the current worktree.
- `set` updates the workstream record and (when updating the active workstream) keeps `context.json`
  in sync with the updated snapshot.

### Workstream create: existing worktrees

Add support for creating a workstream that binds to an existing worktree checkout without running
`git worktree add`:
- CLI: `workstream create ... --existing --path <path> [--branch <branch>]`
- MCP: `workstream_create { existing: true, path: "...", branch?: "..." }`

### Clone adoption helper (CLI + MCP)

Command/tool:
- CLI: `worktree adopt-clone --from <path> [--to <path>] [--branch <target-branch>] [--allow-dirty] [--apply]`
- MCP: `worktree_adopt_clone`

Behavior:
- Default output is a deterministic plan (no filesystem mutation).
- Apply requires `--apply` / `apply=true`.
- If adopting in-place (default), move the clone directory to a timestamped backup path, then create
  the worktree at the original path.
- If the clone is dirty, refuse to apply unless `allow_dirty=true`.
- After creating the worktree, register it in the global worktree registry.

### Truth Ledger integration

- Add optional `workstream_id` field to Truth context.
- `truth propose` supports:
  - explicit `--workstream-id`
  - `--current` to prefer the active workstream id for `feature` defaulting
- `truth list` supports `--workstream-id`.
- `workstream show --truth` lists accepted Truth records linked to the workstream.

## Acceptance Criteria

- CLI and MCP expose the same workstream lifecycle operations with equivalent behavior and stable
  JSON outputs.
- `context set` updates the referenced workstream snapshot when `context.json.workstream_id` is set.
- Clone adoption produces a clear plan and applies safely when requested (backup + worktree add).
- Truth records can be created and queried by `workstream_id`, and are visible via `workstream show --truth`.
