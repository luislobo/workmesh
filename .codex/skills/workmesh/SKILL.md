---
name: workmesh
description: Project management workflow for Markdown-backed backlogs using the WorkMesh CLI + MCP.
---

# WorkMesh skill

Use this skill to manage Markdown-backed workmesh tasks with explicit dependencies.

## Core workflow
- Keep tasks small (1-3 days) and outcome-based.
- Track blockers with dependencies.
- Move tasks through: To Do → In Progress → Done.
- Capture context in Notes/Implementation Notes.
- Use leases/claims for multi-agent coordination.

## Dependencies
- Add dependencies whenever a task is blocked by other work.
- Keep dependencies updated as status changes.
- Always explore interdependencies (why a task is blocked and what unblocks it).

## MCP usage (root optional)
- If the MCP server is started inside a repo, `root` can be omitted.
- Otherwise, include `root`.
- Example: `list_tasks(root="/path/to/repo")`.

## High-signal commands
- Ready work
  - Use when: picking the next task or triaging “what’s unblocked.”
  - Workflow: run `ready --json`, pick smallest ID or highest priority, set status to In Progress.
  - Command: `workmesh --root /path ready --json`
- Claim/release (leases)
  - Use when: multiple agents may pick the same task or work spans multiple sessions.
  - Workflow: claim → work → update status/notes → release when done or paused.
  - Commands: `workmesh --root /path claim task-042 you --minutes 60` / `workmesh --root /path release task-042`
- Graph export
  - Use when: you need dependency/relationship visualization or to feed another tool.
  - Workflow: export → analyze nodes/edges → adjust dependencies if needed.
  - Command: `workmesh --root /path graph-export --pretty`
- Index (JSONL)
  - Use when: bulk edits happened or index may be stale, or you want fast queries.
  - Workflow: rebuild once on new repo, refresh after edits, verify if results look off.
  - Commands: `workmesh --root /path index-rebuild|index-refresh|index-verify`
- Migration
  - Use when: a repo still has `backlog/` or `tasks/` at root.
  - Workflow: `migrate` → verify new `workmesh/` layout → continue.
  - Command: `workmesh --root /path migrate`
- Archive
  - Use when: Done tasks are older and should be moved out of active lists.
  - Workflow: archive by date → keep `workmesh/tasks` lean.
  - Command: `workmesh --root /path archive --before 2024-12-31`
- JSONL issues export
  - Use when: you want a canonical, machine-readable snapshot of tasks.
  - Workflow: export → consume in another tool/report → discard (source remains Markdown).
  - Command: `workmesh --root /path issues-export --output issues.jsonl`
- Discovered work
  - Use when: you find new work while executing another task.
  - Workflow: create discovered task → link to source task → continue current work.
  - Command: `workmesh --root /path add-discovered --from task-042 --title "New bug"`
- Bulk updates
  - Use when: you need to apply the same change to many tasks quickly.
  - Workflow: choose tasks → run bulk command → review summary → re-run validate if needed.
  - Commands: `workmesh --root /path bulk set-status --tasks task-001,task-002 --status "In Progress"` or `bulk label-add`, `bulk dep-add`, `bulk note` (also supports `bulk-set-status` etc.)
- Quickstart
  - Use when: bootstrapping a new repo or enabling WorkMesh in an existing repo.
  - Workflow: run once → review scaffold → add first real tasks.
  - Command: `workmesh --root /path quickstart project-id --agents-snippet`

## CLI quickstart
- List: `workmesh --root /path list --status "To Do"`
- Next: `workmesh --root /path next`
- Start: `workmesh --root /path set-status task-042 "In Progress"`
- Blocked: `workmesh --root /path dep-add task-042 task-017`
- Done: `workmesh --root /path set-status task-042 Done`

## JSON-friendly usage
- Most commands support `--json` for machine-readable output.
- Prefer `--json` in agent workflows.

## Workflow sequences (grammar-style)
Notation:
- `[]` optional, `{}` repeatable, `->` then

Bootstrapping:
- `quickstart -> [index-rebuild] -> add-task -> ready`

Daily execution:
- `ready -> claim -> set-status(In Progress) -> work -> note/set-section -> [set-status(Done)] -> release`

Discovered work:
- `work -> add-discovered(from=task-x) -> continue`

Multi-agent safety:
- `ready -> claim -> work -> release`

Index maintenance:
- `index-rebuild` (first time) -> `{index-refresh}` after edits -> `index-verify` when results look off

Reporting/export:
- `graph-export -> analyze`  
- `issues-export -> consume -> discard`

Bulk updates:
- `select tasks -> bulk-set-status|bulk-set-field|bulk-label-add|bulk-dep-add|bulk-note`

Resume after restart:
- Repo resume (checkpoints):
  - Use when: you are continuing inside the same repo after compaction/restart.
  - Workflow: `checkpoint -> resume -> ready -> claim -> continue`
- Global resume (cross-repo sessions):
  - Use when: you rebooted, switched OS, or you have many repos and want a global "what was I doing?" index.
  - Workflow: `session save -> (reboot) -> session list -> session resume -> ready -> claim -> continue`
  - Commands:
    - `workmesh --root /path session save --objective "..." --json`
    - `workmesh --root /path session list --limit 20 --json`
    - `workmesh --root /path session resume [session_id] --json`
  - Opt-in auto updates:
    - CLI: `--auto-session-save`
    - Env: `WORKMESH_AUTO_SESSION=1`

## Review cadence
- Weekly task review and priority updates.
- Run `validate` before planning and fix errors/warnings.
