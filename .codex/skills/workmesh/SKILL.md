---
name: workmesh
description: Project management workflow for Markdown-backed backlogs using the WorkMesh CLI + MCP.
---

# WorkMesh skill

Use this skill to manage Markdown-backed backlogs with explicit dependencies, deterministic ordering,
and agent-safe coordination (leases/claims).

## Golden rules
- Keep tasks small (1-3 days) and outcome-based.
- Record blockers as dependencies (or `blocked_by`) so "ready work" is queryable.
- Prefer `--json` outputs in agent workflows.
- For multi-agent work: always `claim` before making changes.
- Do not commit derived index artifacts like `workmesh/.index/` (they are rebuildable).
- `.audit.log` is ignored by default; optionally commit it if you want a full, versioned PM history in the repo.

## Focus first (agent-scoping)
`focus` is the lightweight, repo-local state that keeps an agent scoped to the right project/epic.

Workflow:
```text
focus_show -> [focus_set] -> ready -> claim -> work -> release
```

Commands:
- `workmesh --root . focus show --json`
- `workmesh --root . focus set --project-id <pid> [--epic-id task-123] [--objective "..."]`
- `workmesh --root . focus clear`

Focus auto-updates:
- When focus exists, WorkMesh auto-updates `focus.working_set` on task mutations:
  - `set-status "In Progress"` adds the task
  - `set-status "To Do"` / `Done` removes the task
  - `claim` adds the task (active lease implies active work)
- When `focus.epic_id` is set and that epic becomes fully complete, WorkMesh auto-cleans focus by
  clearing `epic_id` and `working_set` (project_id is preserved).

## Dependencies (optional, but recommended)
- Dependencies are optional, but if you know a task is blocked, record it.
- Keep dependencies up to date as status changes.

## MCP usage (root optional)
- If the MCP server is started inside a repo, `root` can be omitted.
- Otherwise, include `root`.

Example (MCP call shape):
```json
{"tool": "list_tasks", "root": "/path/to/repo", "status": ["To Do"]}
```

## High-signal commands
Ready work:
- Use when: picking the next task or triaging "what is unblocked".
- Workflow: `ready_tasks --json` or `next_tasks --json` -> pick -> claim -> set status.
- Command: `workmesh --root /path ready --json`

Next tasks (recommended for agents):
- Use when: you want candidates and let the agent decide.
- Focus-aware: tasks in `focus.working_set` are recommended first.
- MCP: `{"tool":"next_tasks","format":"json","limit":10}`

Claim/release (leases):
- Use when: multiple agents may pick the same task or work spans multiple sessions.
- Workflow: `claim` -> work -> update status/notes -> `release` when done or paused.
- Commands: `workmesh --root /path claim task-042 you --minutes 60` / `workmesh --root /path release task-042`

Graph export:
- Use when: you need dependency/relationship visualization or to feed another tool.
- Command: `workmesh --root /path graph-export --pretty`

Index (JSONL):
- Use when: bulk edits happened or index may be stale, or you want fast queries.
- Commands: `workmesh --root /path index-rebuild|index-refresh|index-verify`
- Note: index files are derived. Keep them ignored by git.

Discovered work:
- Use when: you find new work while executing another task.
- Workflow: create discovered task -> link to source task -> continue current work.
- Command: `workmesh --root /path add-discovered --from task-042 --title "New bug"`

Global sessions:
- Use when: you need cross-repo continuity (reboot, OS switch, machine switch).
- Workflow: `session save` -> later `session resume`.
- Commands: `workmesh --root /path session save --objective "..."` / `workmesh --root /path session resume`

Epic completion rule:
- When `kind: epic`, WorkMesh refuses to set status to `Done` unless:
  - `dependencies` and `blocked_by` are Done
  - all child tasks are Done (inferred via `relationships.parent`, and optional `relationships.child`)

## Workflow sequences (grammar-style)
Notation:
- `[]` optional, `{}` repeatable, `->` then

Bootstrapping:
```text
quickstart -> [index-rebuild] -> add -> focus_set -> ready
```

Daily execution:
```text
focus_show -> ready -> claim -> set-status(In Progress) -> work -> note/set-section -> [set-status(Done)] -> release
```

Rekey IDs (agent-assisted):
```text
rekey-prompt -> agent produces mapping.json -> rekey-apply (dry-run) -> rekey-apply --apply
```
Notes:
- Default behavior rewrites structured references and free-text body mentions.
- Use `--strict` (or mapping `"strict": true`) for structured-only rewrites.

Discovered work:
```text
work -> add-discovered(from=task-x) -> continue
```

Multi-agent safety:
```text
ready -> claim -> work -> release
```

Index maintenance:
```text
index-rebuild (first time) -> {index-refresh} after edits -> index-verify when results look off
```

Resume after restart:
```text
session resume -> focus_show -> ready -> claim -> continue
```
