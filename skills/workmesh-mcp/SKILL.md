---
name: workmesh-mcp
description: MCP-first WorkMesh workflow. Use when WorkMesh MCP tools are available.
---

# WorkMesh MCP Skill

Use this skill when WorkMesh MCP tools are available.

## Bootstrap intent handling
If user says `bootstrap workmesh`, execute this flow:

1. Discover state:
```json
{"tool":"doctor","format":"json"}
```

2. If no WorkMesh structure:
```json
{"tool":"quickstart","project_id":"<project-id>","feature":"<feature-name>","agents_snippet":true,"format":"json"}
{"tool":"context_set","project_id":"<project-id>","objective":"<objective>","format":"json"}
```

3. If legacy structure exists:
```json
{"tool":"migrate_audit","format":"json"}
{"tool":"migrate_plan","format":"json"}
{"tool":"migrate_apply","apply":true,"format":"json"}
```

4. If modern structure exists:
```json
{"tool":"context_show","format":"json"}
{"tool":"truth_list","states":["accepted"],"limit":20,"format":"json"}
{"tool":"next_tasks","format":"json","limit":10}
```

5. If clone-based stream workflow is detected:
- Do not block feature work.
- Recommend canonical repo + worktree migration path.

## Feature work contract
When user says to use WorkMesh for feature development:
- maintain PRD/task documentation continuously
- keep context current
- maintain acceptance criteria and definition of done quality
- capture stable decisions as truths

## High-signal loop
- `{"tool":"next_tasks","format":"json","limit":10}`
- `{"tool":"claim_task","task_id":"task-123","owner":"agent","minutes":60,"touch":true}`
- `{"tool":"set_status","task_id":"task-123","status":"In Progress","touch":true}`
- `{"tool":"add_note","task_id":"task-123","note":"...","section":"notes","touch":true}`
- `{"tool":"set_status","task_id":"task-123","status":"Done","touch":true}`
- `{"tool":"release_task","task_id":"task-123","touch":true}`

## Rules
- Keep task metadata complete: `Description`, `Acceptance Criteria`, `Definition of Done`.
- Move to `Done` only when goals and criteria are fully met.
- Keep dependencies and blockers current.
