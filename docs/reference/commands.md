# Command Reference

This file is command-surface only. For workflow guidance, use [`docs/getting-started.md`](../getting-started.md).

Run/install/agent setup guidance:
- [`docs/setup/run-modes-and-agent-mcp.md`](../setup/run-modes-and-agent-mcp.md)

## Global CLI flags
All subcommands support:
- `--root <path>` (required)
- `--auto-checkpoint`
- `--auto-session-save`
- `--no-auto-session-save`

CLI parity notes:
- The CLI accepts MCP-style aliases in either underscore or hyphen form.
- Examples:
  - `help` -> `--help`
  - `list_tasks` -> `list`
  - `show_task` -> `show`
  - `config_show` -> `config show`
  - `truth_list` -> `truth list`
  - `workstream_list` -> `workstream list`
  - `worktree_list` -> `worktree list`
  - `render_table` -> `render table`

## Defaults and config
Global config:
- `~/.workmesh/config.toml` (or `$WORKMESH_HOME/config.toml`)

Project config:
- `.workmesh.toml` (preferred)

Keys:
- `worktrees_default = true|false`
- `worktrees_dir = "<path>"` (absolute or repo-relative; used for auto-provisioned worktrees; default: `<repo_parent>/<repo_name>.worktrees/`)
- `auto_session_default = true|false`

Precedence:
1. CLI flags
2. Environment variables
3. Project config
4. Global config
5. Built-in defaults

Environment overrides:
- `WORKMESH_AUTO_CHECKPOINT=1|0`
- `WORKMESH_AUTO_SESSION=1|0`

## Config
CLI:
- `config show [--json]`
- `config set --scope project|global --key worktrees_default|worktrees_dir|auto_session_default|root_dir|do_not_migrate --value <value> [--json]`
- `config unset --scope project|global --key worktrees_default|worktrees_dir|auto_session_default|root_dir|do_not_migrate [--json]`

MCP:
- `config_show`
- `config_set`
- `config_unset`

## Bootstrap and diagnostics
CLI:
- `readme [--json]`
- `tool-info <tool-name> [--json]`
- `skill-content [--name <skill>] [--json]`
- `project-management-skill [--name <skill>] [--json]`
- `bootstrap [--project-id <id>] [--feature "..."] [--objective "..."] [--json]`
- `quickstart <project-id> [--name "..."] [--feature "..."] [--agents-snippet]`
- `project-init <project-id> [--name "..."]`
- `doctor [--fix-storage] [--json]`
- `validate [--json]`

MCP:
- `readme`
- `tool_info`
- `skill_content`
- `project_management_skill`
- `bootstrap`
- `quickstart`
- `project_init`
- `doctor`
- `validate`

`tool-info` note:
- CLI `tool-info` mirrors the shared metadata/examples from `workmesh-tools`.
- MCP `tool_info` remains the canonical source for the full MCP input schema.

Doctor storage fix behavior:
- `--fix-storage` (CLI) / `fix_storage=true` (MCP) performs safe remediation only:
  - trim trailing malformed JSONL lines for sessions/truth event streams
  - rebuild truth projection when applicable
  - rebuild sessions index when applicable
- Non-trailing malformed JSONL is reported but not auto-trimmed.
- Doctor output includes storage integrity checks:
  - lock-path accessibility
  - malformed JSONL counts
  - truth projection/event divergence
  - versioned snapshot state

Conflict semantics:
- Versioned snapshot writes use compare-and-swap behavior.
- Stale writes surface explicit conflict errors; they are not silently overwritten.
- Legacy unversioned snapshots are treated as version `0` and migrated on first safe write.

## Mutation response policy
- MCP mutation tools return minimal acknowledgements by default to save tokens.
- Pass `verbose=true` when you need richer post-write state in the same response.
- Prefer dedicated read tools (`show_task`, `truth_show`, `session_show`, `workstream_show`, `context_show`) when you need the full current object.
- Typical defaults:
  - single-record mutation: `{"ok": true, "id": "..."}`
  - field/status mutation: `{"ok": true, "id": "...", "status": "Done"}`
  - bulk mutation: `{"ok": false, "updated_count": 3, "failed_count": 1, "failed_ids": ["task-009"]}`

## Renderer tools
CLI:
- `render table|kv|stats|list|progress|tree|diff|logs|alerts|chart-bar|sparkline|timeline`
- input: one of `--data <value>`, `--data-file <path>`, or `--stdin`
- optional: `--format <value>`
- optional: one of `--configuration <json>` or `--config-file <path>`

## Task selection and read views
CLI:
- `list [--status "To Do"] [--kind bug] [--search "..."] [--sort id] [--all] [--json]`
- `show <task-id> [--full] [--json]`
- `next [--json]`
- `next-tasks [--limit N] [--json]`
- `ready [--limit N] [--json]`
- `board [--by status|phase|priority] [--focus] [--all] [--json]`
- `blockers [--epic-id task-123] [--all] [--json]`
- `stats [--json]`

MCP:
- `list_tasks`
- `show_task`
- `next_task`, `next_tasks`
- `ready_tasks`
- `board`
- `blockers`
- `stats`

## Task mutations
CLI:
- `add --title "..." [--id task-...] [--status "..."] [--priority P2] [--phase Phase1] [--labels "..."] [--dependencies "..."] [--assignee "..."] [--json]`
- `add-discovered --from <task-id> --title "..." ...`
- `set-status <task-id> "In Progress"|"To Do"|Done`
- `set-field <task-id> <field> <value>`
- `label-add <task-id> <label>` / `label-remove <task-id> <label>`
- `dep-add <task-id> <dependency-id>` / `dep-remove <task-id> <dependency-id>`
- `note <task-id> "..." [--section notes|impl]`
- `set-body <task-id> [--text "..."] [--file path]`
- `set-section <task-id> <section> [--text "..."] [--file path]`
- `claim <task-id> <owner> [--minutes 60]`
- `release <task-id>`

MCP:
- `add_task`
- `add_discovered`
- `set_status`
- `set_field`
- `add_label`, `remove_label`
- `add_dependency`, `remove_dependency`
- `add_note`
- `set_body`, `set_section`
- `claim_task`, `release_task`

MCP mutation response contract:
- default: minimal acknowledgement
- opt-in: `verbose=true` for richer post-write state
- examples:
  - `set_status` default: `{"ok": true, "id": "task-001", "status": "Done"}`
  - `set_status` verbose: includes the refreshed `task`
  - `add_task` default: `{"ok": true, "id": "task-123", "path": "..."}`
  - `add_task` verbose: includes `task`, `hints`, and `next_steps`

Task quality guardrails:
- Required task-body sections: `Description`, `Acceptance Criteria`, `Definition of Done`.
- `Definition of Done` must include outcome-based criteria (not only hygiene bullets).
- `Done` transitions are gated across all status mutation paths:
  - `set-status ... Done`
  - `set-field ... status Done`
  - `bulk set-status --status Done`
  - `bulk set-field --field status --value Done`
- `validate` behavior:
  - non-`Done` tasks with missing/incomplete sections produce warnings
  - `Done` tasks with missing/incomplete sections (or hygiene-only DoD) produce errors

## Bulk operations
CLI:
- `bulk set-status --tasks task-001,task-002 --status "In Progress" [--json]`
- `bulk set-field --tasks ... --field priority --value P1 [--json]`
- `bulk label-add --tasks ... --label docs [--json]`
- `bulk label-remove --tasks ... --label docs [--json]`
- `bulk dep-add --tasks ... --dependency task-123 [--json]`
- `bulk dep-remove --tasks ... --dependency task-123 [--json]`
- `bulk note --tasks ... --note "..." [--section notes|impl] [--json]`

MCP:
- `bulk_set_status`
- `bulk_set_field`
- `bulk_add_label`, `bulk_remove_label`
- `bulk_add_dependency`, `bulk_remove_dependency`
- `bulk_add_note`

MCP mutation response contract:
- default: summary only (`ok`, `updated_count`, `failed_count`, `failed_ids`)
- opt-in: `verbose=true` for full updated/missing lists

## Context
CLI:
- `context show [--json]`
- `context set --project <pid> [--epic task-123] [--objective "..."] [--tasks task-001,task-002]`
- `context clear`

MCP:
- `context_show`
- `context_set`
- `context_clear`

MCP mutation response contract:
- `context_set` / `context_clear` default to compact acknowledgements
- pass `verbose=true` to include richer context payloads

## Truth Ledger
CLI:
- `truth propose --title "..." --statement "..." [--project <pid>] [--epic task-123] [--feature <name>] [--workstream-id <id>] [--current] [--session-id <id>] [--worktree-id <id>] [--worktree-path <path>] [--constraints "a,b"] [--tags "x,y"] [--json]`
- `truth accept <truth-id> [--note "..."] [--json]`
- `truth reject <truth-id> [--note "..."] [--json]`
- `truth supersede <truth-id> --by <accepted-truth-id> [--reason "..."] [--json]`
- `truth show <truth-id> [--json]`
- `truth list [--state proposed|accepted|rejected|superseded] [--project <pid>] [--epic task-123] [--feature <name>] [--workstream-id <id>] [--session-id <id>] [--worktree-id <id>] [--worktree-path <path>] [--tag <tag>] [--limit N] [--json]`
- `truth validate [--json]`
- `truth migrate audit|plan|apply [--apply] [--json]`

MCP:
- `truth_propose`
- `truth_accept`
- `truth_reject`
- `truth_supersede`
- `truth_show`
- `truth_list`
- `truth_validate`
- `truth_migrate_audit`
- `truth_migrate_plan`
- `truth_migrate_apply`

MCP mutation response contract:
- truth mutations default to compact `{ ok, truth_id, state, version }` style responses
- pass `verbose=true` for the full truth record or full migration result

## Workstream runtime
CLI:
- `workstream list [--json]`
- `workstream restore [--all] [--json]`
- `workstream create --name "..." [--key <key>] [--existing] [--path <path> --branch <branch> --from <ref>] [--project <pid>] [--epic task-123] [--objective "..."] [--tasks task-001,task-002] [--json]`
- `workstream show [<id-or-key>] [--truth] [--restore] [--json]`
- `workstream switch <id-or-key> [--json]`
- `workstream pause [<id-or-key>] [--json]`
- `workstream close [<id-or-key>] [--json]`
- `workstream reopen [<id-or-key>] [--json]`
- `workstream rename [<id-or-key>] --name "..." [--json]`
- `workstream set [<id-or-key>] [--key <key>] [--notes "..."] [--project <pid>] [--epic task-123] [--objective "..."] [--tasks task-001,task-002] [--json]`
- `workstream doctor [--json]`

MCP:
- `workstream_list`
- `workstream_create`
- `workstream_show` (supports `truths=true`, `restore=true`)
- `workstream_switch`
- `workstream_pause`
- `workstream_close`
- `workstream_reopen`
- `workstream_rename`
- `workstream_set`
- `workstream_doctor`
- `workstream_restore`

MCP mutation response contract:
- workstream mutations default to compact success metadata
- pass `verbose=true` to include the refreshed workstream object or full creation details

Notes:
- Active workstream pointer is per-worktree: `workmesh/context.json.workstream_id`.
- When `workstream_id` is set, these commands keep workstream pointers up to date:
- `session save` updates the active workstream `session_id` and worktree binding.
- `worktree attach` updates the active workstream `session_id` and worktree binding.
- `worktree detach` clears the active workstream `session_id` when it matches the detached session.
- `context set` preserves `workstream_id` and persists the updated context snapshot into the workstream record (best-effort).
- `workstream pause` and `workstream close` clear `context.json.workstream_id` when the paused/closed stream was active in this worktree.
- `workstream create` can auto-provision a new git worktree when invoked from the canonical checkout and `worktrees_default=true` (requires a real `HEAD` commit). Override by passing `--existing` or explicit `--path/--branch`.
- `workstream create` is idempotent for a given target worktree path: if that path is already bound, it returns the existing workstream (`already_exists=true`) instead of creating a duplicate.

## Worktree runtime
CLI:
- `worktree list [--json]`
- `worktree create --path <path> --branch <branch> [--from <ref>] [--project <pid>] [--epic task-123] [--objective "..."] [--tasks task-001,task-002] [--json]`
- `worktree adopt-clone --from <path> [--to <path>] [--branch <target-branch>] [--allow-dirty] [--apply] [--json]`
- `worktree attach [--session-id <id>] [--path <path>] [--json]`
- `worktree detach [--session-id <id>] [--json]`
- `worktree doctor [--json]`

MCP:
- `worktree_list`
- `worktree_create`
- `worktree_adopt_clone`
- `worktree_attach`
- `worktree_detach`
- `worktree_doctor`

MCP mutation response contract:
- worktree mutations default to compact success metadata
- pass `verbose=true` to include the full worktree/adoption/session payload

## Sessions and continuity
Repo-local CLI:
- `checkpoint [--project <id>] [--id <checkpoint-id>] [--json]`
- `resume [--project <id>] [--id <checkpoint-id>] [--json]`
- `checkpoint-diff [--project <id>] [--id <checkpoint-id>] [--json]`
- `working-set [--project <id>] [--tasks "task-001,task-002"] [--note "..."] [--json]`
- `session-journal [--project <id>] [--task <id>] [--next "..."] [--note "..."] [--json]`

Global sessions CLI:
- `session save --objective "..." [--project <id>] [--tasks "task-..."]`
- `session list [--limit N]`
- `session show <session-id>`
- `session resume [<session-id>]`
- `session index-rebuild|index-refresh|index-verify`

MCP:
- `checkpoint`
- `resume`
- `checkpoint_diff`
- `working_set`
- `session_journal`
- `session_save`
- `session_list`
- `session_show`
- `session_resume`

MCP mutation response contract:
- `session_save` defaults to `{ ok, session_id, cwd, repo_root }`
- pass `verbose=true` to receive the full saved session object

## Migration actions
`migrate audit|plan|apply` may produce the following action ids:
- `layout_backlog_to_workmesh`
- `focus_to_context`
- `task_section_normalization`
- `truth_backfill`
- `session_handoff_enrichment`
- `config_cleanup`

## Index and exports
CLI:
- `index-rebuild [--json]`
- `index-refresh [--json]`
- `index-verify [--json]`
- `export [--pretty]`
- `issues-export [--output path] [--include-body]`
- `graph-export [--pretty]`
- `gantt`, `gantt-file`, `gantt-svg`

MCP:
- `index_rebuild`
- `index_refresh`
- `index_verify`
- `export_tasks`
- `issues_export`
- `graph_export`
- `gantt_text`, `gantt_file`, `gantt_svg`

## Renderer tools (MCP)
Available over MCP stdio:
- `render_table`, `render_kv`, `render_stats`, `render_list`, `render_progress`
- `render_tree`, `render_diff`, `render_logs`, `render_alerts`
- `render_chart_bar`, `render_sparkline`, `render_timeline`

All render tools accept:
- `data` (required)
- `format` (optional)
- `configuration` (optional)

They return rendered text content.

## Archive and maintenance
CLI:
- `archive [--before 30d|YYYY-MM-DD] [--status <state>]... [--json]`
- default status filter (when omitted): `Done`, `Cancelled`, `Canceled`, `Won't Do`, `Wont Do`
- override behavior: pass one or more `--status` values to archive any specific state, including non-terminal states
- `fix list [--json]`
- `fix uid|deps|ids [--check|--apply] [--json]`
- `fix all [--only uid,deps,ids] [--exclude uid,deps,ids] [--check|--apply] [--json]`

MCP:
- `archive_tasks`
- `archive_tasks` accepts optional `status` (string or list); when omitted it uses the same default terminal status filter as CLI
- `fix_ids`

MCP mutation response contract:
- `archive_tasks` defaults to summary counts and archive path metadata
- pass `verbose=true` to include full archived/skipped lists

## Legacy migration (minimal)
Use only when a repo still has deprecated structures.

CLI:
- `migrate audit [--json]`
- `migrate plan [--include ...] [--exclude ...] [--json]`
- `migrate apply [--include ...] [--exclude ...] [--apply] [--backup] [--json]`

MCP:
- `migrate_audit`
- `migrate_plan`
- `migrate_apply`

MCP mutation response contract:
- `migrate_apply` defaults to summary counts
- pass `verbose=true` for the full applied/skipped/backup result
