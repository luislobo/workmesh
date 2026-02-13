---
name: workmesh-mcp
description: MCP-first WorkMesh workflow. Use when WorkMesh MCP tools are available.
---

# WorkMesh MCP Skill

Use this skill when interacting with WorkMesh through MCP tool calls.

## When to use
- WorkMesh MCP server is enabled.
- You want structured JSON tool responses and parity with CLI behavior.

## Setup
Run once per project:
```bash
# install MCP-focused skill in the current project
workmesh --root . install --skills --profile mcp --scope project
```

## Baseline tool calls
```json
{"tool":"doctor","format":"json"}
```
```json
{"tool":"context_show","format":"json"}
```

## High-signal tool loop
- Next candidates: `{"tool":"next_tasks","format":"json","limit":10}`
- Claim work: `{"tool":"claim_task","task_id":"task-123","owner":"agent","minutes":60,"touch":true}`
- Mark active: `{"tool":"set_status","task_id":"task-123","status":"In Progress","touch":true}`
- Capture note: `{"tool":"add_note","task_id":"task-123","note":"...","section":"notes","touch":true}`
- Finish: `{"tool":"set_status","task_id":"task-123","status":"Done","touch":true}`
- Release: `{"tool":"release_task","task_id":"task-123","touch":true}`

## Grammar-style workflows
Notation:
- `[]` optional
- `{}` repeatable
- `->` then

Bootstrap:
```text
quickstart -> context_set -> next_tasks
```

Daily loop:
```text
context_show -> next_tasks -> claim_task -> set_status(In Progress) -> work -> add_note -> set_status(Done) -> release_task
```

Continuity:
```text
session_save -> stop -> session_resume -> context_show -> next_tasks -> claim_task
```

Parallel worktree loop:
```text
worktree_create -> worktree_attach -> context_set -> next_tasks -> claim_task
```

Hygiene:
```text
doctor -> blockers -> board(focus=true) -> validate -> index_refresh
```

## MCP-specific rules
- Root handling:
  - If server starts inside a repo, `root` is optional.
  - Otherwise provide `root`.
- Prefer `next_tasks` over `next_task` when the agent should choose among candidates.
- Keep dependencies and `blocked_by` updated so blockers views remain useful.
- Keep task metadata complete and current: `Description`, `Acceptance Criteria`, and `Definition of Done`.
- Move a task to `Done` only when the task goals in `Description` are met and all `Acceptance Criteria` are satisfied.
- Treat `Code/config committed` and `Docs updated if needed` as hygiene checks, not the core completion criteria.
