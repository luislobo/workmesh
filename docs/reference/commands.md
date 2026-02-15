# Command Reference

This file is command-surface only. For workflow guidance, use [`docs/getting-started.md`](../getting-started.md).

## Global CLI flags
All subcommands support:
- `--root <path>` (required)
- `--auto-checkpoint`
- `--auto-session-save`
- `--no-auto-session-save`

## Defaults and config
Global config:
- `~/.workmesh/config.toml` (or `$WORKMESH_HOME/config.toml`)

Project config:
- `.workmesh.toml` (preferred)

Keys:
- `worktrees_default = true|false`
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

## Bootstrap and diagnostics
CLI:
- `bootstrap [--project-id <id>] [--feature "..."] [--objective "..."] [--json]`
- `quickstart <project-id> [--name "..."] [--feature "..."] [--agents-snippet]`
- `project-init <project-id> [--name "..."]`
- `doctor [--json]`
- `validate [--json]`

MCP:
- `bootstrap`
- `quickstart`
- `project_init`
- `doctor`
- `validate`

## Task selection and read views
CLI:
- `list [--status "To Do"] [--kind bug] [--search "..."] [--sort id] [--all] [--json]`
- `show <task-id> [--full] [--json]`
- `next [--json]`
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

## Context
CLI:
- `context show [--json]`
- `context set --project <pid> [--epic task-123] [--objective "..."] [--tasks task-001,task-002]`
- `context clear`

MCP:
- `context_show`
- `context_set`
- `context_clear`

## Truth Ledger
CLI:
- `truth propose --title "..." --statement "..." [--project <pid>] [--epic task-123] [--feature <name>] [--session-id <id>] [--worktree-id <id>] [--worktree-path <path>] [--constraints "a,b"] [--tags "x,y"] [--json]`
- `truth accept <truth-id> [--note "..."] [--json]`
- `truth reject <truth-id> [--note "..."] [--json]`
- `truth supersede <truth-id> --by <accepted-truth-id> [--reason "..."] [--json]`
- `truth show <truth-id> [--json]`
- `truth list [--state proposed|accepted|rejected|superseded] [--project <pid>] [--epic task-123] [--feature <name>] [--session-id <id>] [--worktree-id <id>] [--worktree-path <path>] [--tag <tag>] [--limit N] [--json]`
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

## Worktree runtime
CLI:
- `worktree list [--json]`
- `worktree create --path <path> --branch <branch> [--from <ref>] [--project <pid>] [--epic task-123] [--objective "..."] [--tasks task-001,task-002] [--json]`
- `worktree attach [--session-id <id>] [--path <path>] [--json]`
- `worktree detach [--session-id <id>] [--json]`
- `worktree doctor [--json]`

MCP:
- `worktree_list`
- `worktree_create`
- `worktree_attach`
- `worktree_detach`
- `worktree_doctor`

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
