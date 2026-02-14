---
name: workmesh-mcp
description: MCP-first WorkMesh workflow. Use when WorkMesh MCP tools are available.
---

# WorkMesh MCP Skill

Use this skill for tool-call-first WorkMesh workflows.

## Baseline tool calls
```json
{"tool":"doctor","format":"json"}
```
```json
{"tool":"context_show","format":"json"}
```
```json
{"tool":"worktree_list","format":"json"}
```
```json
{"tool":"truth_list","states":["accepted"],"limit":20,"format":"json"}
```

## Progressive loops

Stage 1: Start
```text
quickstart -> context_set -> next_tasks -> claim_task -> set_status(In Progress) -> add_note -> set_status(Done) -> release_task
```

Stage 2: Parallelize
```text
worktree_create -> session_save -> worktree_attach -> context_show -> next_tasks -> claim_task
```

Stage 3: Recover
```text
worktree_list -> session_resume -> context_show -> truth_list(accepted) -> next_tasks
```

Stage 4: Consolidate clones
```text
audit sibling clones (manual today) -> create canonical worktree per stream -> session_save/worktree_attach -> retire old clones later
```

## High-signal loop
- `{"tool":"next_tasks","format":"json","limit":10}`
- `{"tool":"claim_task","task_id":"task-123","owner":"agent","minutes":60,"touch":true}`
- `{"tool":"set_status","task_id":"task-123","status":"In Progress","touch":true}`
- `{"tool":"add_note","task_id":"task-123","note":"...","section":"notes","touch":true}`
- `{"tool":"set_status","task_id":"task-123","status":"Done","touch":true}`
- `{"tool":"release_task","task_id":"task-123","touch":true}`

## Defaults and overrides
- Worktree guidance defaults to ON (`worktrees_default`).
- Auto session updates should run in interactive local workflows by default (`auto_session_default`).
- Explicit override remains available through environment:
  - `WORKMESH_AUTO_SESSION=1` (force on)
  - `WORKMESH_AUTO_SESSION=0` (force off)

## MCP rules
- If server starts inside a repo, `root` is optional; otherwise provide `root`.
- Prefer `next_tasks` when choosing among candidates.
- Keep dependencies and blocked state up to date.
- Persist durable decisions using truth tools with available scope context.
- Keep task metadata complete: `Description`, `Acceptance Criteria`, `Definition of Done`.
- Move to `Done` only when description goals + acceptance criteria are satisfied.
- Treat `Code/config committed` and `Docs updated if needed` as hygiene checks.
