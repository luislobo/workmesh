# PRD: Phase 4 - Global agent sessions (cross-repo continuity)

Date: 2026-02-07
Owner: Luis Lobo
Status: Draft

## Problem
Developers routinely work across multiple repositories for different companies and personal
projects. When a workstation reboots (or the developer switches OS, e.g., gaming on Windows),
the agent "session context" becomes fragmented and hard to recover:
- Which repo and working directory (PWD) was active?
- What was the objective?
- Which tasks were in progress and why?
- What was the last meaningful checkpoint?
- What changed recently (files/dirs) and what should happen next?

Per-repo WorkMesh tasks help manage project work, but they do not provide a global,
cross-repo "what was I doing?" database.

## Goals
- Provide a global, developer-local database of "agent sessions" that spans many repos.
- Make sessions easy to capture ("save state") and easy to resume ("what do I do next?").
- Keep storage deterministic, git-friendly, and offline-first.
- Provide CLI + MCP parity for all session commands.
- Support automation: default-on auto-save in interactive local workflows, with explicit overrides.

## Non-goals
- External syncing (Jira/Trello/GitHub). Deferred.
- Centralized cloud storage. (Local-first; export can come later.)
- Full replay of terminal history. (We capture "work context", not a shell recorder.)

## Requirements
### Data model
An "agent session" is a single record identified by a collision-safe ID (ULID).

Minimum fields:
- `id` (ulid), `created_at`, `updated_at`
- `cwd` (absolute path at save time)
- `repo_root` (resolved via WorkMesh root discovery; optional)
- `project_id` (if detected from `docs/projects/<id>`; optional)
- `objective` (free text; required for `session save`)
- `working_set` (list of task IDs; optional)
- `git` snapshot (optional):
  - `branch`, `head_sha`, `dirty` flag
- `checkpoint` reference (optional):
  - path + timestamp if found
- `notes` (free text; optional)
- `recent_changes` (optional; lightweight):
  - top-level dirs touched, plus a small list of changed files (best effort)

### Storage
Default global store:
- `~/.workmesh/sessions/` (or `$WORKMESH_HOME/sessions/` if configured)

File format:
- JSONL, append-friendly. Each line is a snapshot event:
  - `type: "session_saved"`
  - `session: { ... }`

Index:
- Optional derived index for fast listing/filtering:
  - `~/.workmesh/.index/sessions.jsonl`
  - Rebuildable from source JSONL.

### CLI commands
Add a `session` command group:
- `workmesh session save --objective "..." [--cwd <path>] [--project <id>] [--tasks task-001,task-002] [--notes "..."]`
- `workmesh session list [--limit N] [--repo <path>] [--project <id>] [--status <...>] [--json]`
- `workmesh session show <session-id> [--json]`
- `workmesh session resume <session-id> [--json]`
  - Returns:
    - A concise summary
    - A suggested "resume script" (e.g., `cd ...`, then `workmesh --root ... resume --project ...`)

Automation (default + overrides):
- Built-in default:
  - interactive + non-CI: auto session updates enabled
  - CI/non-interactive: auto session updates disabled
- Explicit override:
  - enable: `--auto-session-save` or `WORKMESH_AUTO_SESSION=1`
  - disable: `--no-auto-session-save` or `WORKMESH_AUTO_SESSION=0`
- Config defaults:
  - `auto_session_default = true|false` in `.workmesh.toml` or `~/.workmesh/config.toml`
- Effect:
  - mutating commands update the \"current session\" best-effort snapshot.

### MCP tools
Expose the same features via MCP tools with parity behavior:
- `session_save`
- `session_list`
- `session_show`
- `session_resume`

### Privacy & safety
- Global session store is local to the developer.
- Session contents must never include secrets by default.
- Redact common secret patterns from captured notes/objective if feasible (best effort).

### Testing
- Unit tests for:
  - storage path resolution (`~/.workmesh` vs config override)
  - session serialization/deserialization
  - index rebuild/verify (if implemented)
- Integration tests for CLI + MCP parity:
  - create session, list, show, resume
  - deterministic output ordering (stable by `updated_at` then `id`)

## Acceptance criteria
- A developer can save and resume sessions across unrelated repos after a reboot.
- Sessions are queryable and deterministic (ordering and JSON output stable).
- CLI and MCP produce equivalent results for the same operations.
- The feature remains local-first and does not require git in the global store.
