# Command Reference

WorkMesh exposes the same core capabilities via:
- CLI: `workmesh ...`
- MCP tools: `{"tool":"...", ...}`

Naming:
- CLI uses kebab-case subcommands (e.g. `set-status`, `graph-export`).
- MCP uses snake_case tool names (e.g. `set_status`, `graph_export`).

All examples assume you run from a repo root and pass `--root .` in the CLI.

## Bootstrap and diagnostics
CLI:
- `quickstart <project-id> [--agents-snippet]`
- `project-init <project-id> [--name "..."]`
- `doctor [--json]`
- `migrate audit [--json]`
- `migrate plan [--include ...] [--exclude ...] [--json]`
- `migrate apply [--include ...] [--exclude ...] [--apply] [--backup] [--json]`

MCP:
- `quickstart`
- `project_init`
- `doctor`
- `migrate_backlog`
- `migrate_audit`
- `migrate_plan`
- `migrate_apply`

## Read views (pick the next work)
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
- `next_task` / `next_tasks`
- `ready_tasks`
- `board`
- `blockers`
- `stats`

## Task write operations
CLI:
- `add --title "..." [--id task-...] [--status "..."] [--priority P2] [--phase Phase1] [--labels "..."] [--dependencies "..."] [--assignee "..."] [--json]`
- `add-discovered --from <task-id> --title "..." ...`
- `set-status <task-id> "In Progress" | "To Do" | Done`
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
- `add_label` / `remove_label`
- `add_dependency` / `remove_dependency`
- `add_note`
- `set_body`
- `set_section`
- `claim_task`
- `release_task`

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
- `bulk_add_label` / `bulk_remove_label`
- `bulk_add_dependency` / `bulk_remove_dependency`
- `bulk_add_note`

## Context (keep agents scoped)
CLI:
- `context show [--json]`
- `context set --project <pid> [--epic task-123] [--objective "..."] [--tasks task-001,task-002]`
- `context clear`
- Deprecated alias: `focus show|set|clear`

MCP:
- `context_show`
- `context_set`
- `context_clear`
- Deprecated alias: `focus_show|focus_set|focus_clear`

## Truth ledger (durable decisions)
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

Rules:
- Lifecycle is strict: `proposed -> accepted|rejected`, and `accepted -> superseded`.
- Truth data lives in `workmesh/truth/events.jsonl` (append-only) and `workmesh/truth/current.jsonl` (projection).

## Worktree runtime (parallel agent execution)
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

## Archive and hygiene
CLI:
- `archive [--before 30d|YYYY-MM-DD] [--status Done] [--json]`
- `archive` defaults to `--before 30d`, so only tasks with `task_date <= (today - 30 days)` are moved.
- `task_date` uses `updated_date`, then `created_date`, then today.
- `archive --before 0d` archives all matching `Done` tasks dated today or earlier.
- `Archived 0 tasks` is expected when nothing matches the threshold.
- `validate [--json]`
- `fix list [--json]`
- `fix uid|deps|ids [--check|--apply] [--json]`
- `fix all [--only uid,deps,ids] [--exclude uid,deps,ids] [--check|--apply] [--json]`
- `fix-ids [--apply] [--json]` (legacy alias for id-only fixer)
- `validate [--json]` includes truth store consistency checks.

Migration action keys:
- `layout_backlog_to_workmesh`
- `focus_to_context`
- `truth_backfill`
- `session_handoff_enrichment`
- `config_cleanup`

MCP:
- `archive_tasks`
- `validate`
- `fix_ids`

## Rekeying task IDs (agent-assisted)
CLI:
- `rekey-prompt [--all] [--include-body] [--json] > rekey-prompt.txt`
- `rekey-apply [--mapping mapping.json] [--apply] [--all] [--strict] [--json]`

MCP:
- `rekey_prompt`
- `rekey_apply`

## Index (derived JSONL)
CLI:
- `index-rebuild [--json]`
- `index-refresh [--json]`
- `index-verify [--json]`

MCP:
- `index_rebuild`
- `index_refresh`
- `index_verify`

## Exports and reporting
CLI:
- `export [--pretty]`
- `issues-export [--output path] [--include-body]`
- `graph-export [--pretty]`
- `gantt` / `gantt-file` / `gantt-svg`

MCP:
- `export_tasks`
- `issues_export`
- `graph_export`
- `gantt_text` / `gantt_file` / `gantt_svg`

## Sessions (continuity)
CLI (repo-local):
- `checkpoint [--project <id>] [--id <checkpoint-id>] [--json]`
- `resume [--project <id>] [--id <checkpoint-id>] [--json]`
- `checkpoint-diff [--project <id>] [--id <checkpoint-id>] [--json]`
- `working-set [--project <id>] [--tasks "task-001,task-002"] [--note "..."] [--json]`
- `session-journal [--project <id>] [--task <id>] [--next "..."] [--note "..."] [--json]`

CLI (global sessions):
- `session save --objective "..." [--project <id>] [--tasks "task-..."]`
- `session list`
- `session show <session-id>`
- `session resume [--session-id <id>]`

Session/truth integration:
- `session save` and worktree attach/detach refresh scoped accepted `truth_refs`.
- `session resume` includes those `truth_refs` and suggests a scoped `truth list --state accepted ...` command.

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

## Skills (Agent Skills standard)
CLI:
- `install --skills [--profile hybrid|cli|mcp|all] [--scope project|user] [--agent codex|claude|cursor|all] [--force] [--json]`
- `uninstall --skills [--profile hybrid|cli|mcp|all] [--scope project|user] [--agent codex|claude|cursor|all] [--json]`
- `skill show [--name workmesh] [--json]`
- `skill install [--scope user|project] [--agent codex|claude|cursor|all] [--force] [--json]`
- `skill uninstall [--scope user|project] [--agent codex|claude|cursor|all] [--json]`
- `skill install-global [--force] [--json]`
- `skill uninstall-global [--json]`

Skill names:
- `workmesh` (router)
- `workmesh-cli` (CLI-first)
- `workmesh-mcp` (MCP-first)

MCP:
- `skill_content`
- `project_management_skill`

## MCP meta tools
MCP:
- `help`
- `tool_info`
- `version`
- `readme`
