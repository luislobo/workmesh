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
- `migrate [--to workmesh] [--yes]`

MCP:
- `quickstart`
- `project_init`
- `doctor`
- `migrate_backlog`

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

## Focus (keep agents scoped)
CLI:
- `focus show [--json]`
- `focus set --project-id <pid> [--epic-id task-123] [--objective "..."]`
- `focus clear`

MCP:
- `focus_show`
- `focus_set`
- `focus_clear`

## Archive and hygiene
CLI:
- `archive [--before 30d|YYYY-MM-DD] [--status Done] [--json]`
- `validate [--json]`
- `fix-ids [--apply] [--json]`

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
- `skill show [--name workmesh] [--json]`
- `skill install [--scope user|project] [--agent codex|claude|cursor|all] [--force] [--json]`
- `skill install-global [--force] [--json]`
- `skill install-global-auto [--force] [--json]`

MCP:
- `skill_content`
- `project_management_skill`

## MCP meta tools
MCP:
- `help`
- `tool_info`
- `version`
- `readme`

