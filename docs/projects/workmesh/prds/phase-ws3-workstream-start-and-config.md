# WorkMesh PRD: Phase WS3 - Workstream Auto-Provision + Config Helpers

Date: 2026-02-16
Owner: Luis Lobo
Status: Implemented

## Problem

Workstreams are the primary abstraction for parallel work, but two day-to-day friction points remained:

1. Starting a new stream from the canonical checkout required specifying a worktree path and branch
   manually, which is repetitive and easy to get wrong.
2. Important defaults (worktrees guidance, worktree directory, and auto session updates) required
   editing config files by hand, which is poor developer experience for chat-driven workflows.

## Goals

- Make starting a new workstream from the canonical checkout a single command with safe defaults.
- Provide first-class config helpers (CLI + MCP) so agents can read and update defaults without
  manual file edits.
- Improve stream resumption ergonomics by exposing a single-stream restore view from `workstream show`.

## Non-goals

- No new background daemons.
- No changes to core storage safety guarantees (Phase 0 already defines that contract).
- No change to Truth Ledger semantics beyond what was completed in WS2.

## Requirements

### Workstream create auto-provision (CLI + MCP)

- When invoked from the canonical checkout (repo root resolved via git common dir), with:
  - `worktrees_default = true` (effective)
  - a real `HEAD` commit (git repository has at least one commit)
  - and no explicit `--path/--branch` (CLI) / `path/branch` (MCP)
- Then `workstream create` should automatically:
  - derive/dedupe a stream key
  - pick a deterministic worktree directory
  - pick a deterministic new branch name (deduped)
  - create the git worktree
  - register the worktree
  - create the workstream bound to that worktree
  - seed context best-effort

Defaults:
- Worktree directory:
  - use `worktrees_dir` when set (absolute or repo-relative)
  - otherwise default to `<repo_parent>/<repo_name>.worktrees/`
- Branch base: `ws/<workstream_key>` (deduped as needed).

### Config helpers (CLI + MCP)

Provide tooling to show and mutate:
- `worktrees_default`
- `worktrees_dir`
- `auto_session_default`
- `root_dir`
- `do_not_migrate`

Including:
- visibility into project/global configs and effective values + sources.

### Restore view for a single stream

- CLI: `workstream show --restore` returns a per-stream restore view including resume commands.
- MCP: `workstream_show` supports `restore=true` and returns the same view.

## Acceptance Criteria

- Starting a new stream from the canonical checkout can be done with one command:
  - CLI: `workstream create --name "..."`
  - MCP: `workstream_create { name: "..." }`
- Config can be read/written without editing files:
  - CLI: `config show|set|unset`
  - MCP: `config_show|config_set|config_unset`
- Workstream show can return single-stream resume commands:
  - CLI: `workstream show --restore`
  - MCP: `workstream_show { restore: true }`
- Full test suite remains green.

