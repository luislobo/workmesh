---
name: workmesh-mcp
description: MCP-first WorkMesh workflow. Use when WorkMesh MCP tools are available.
---

# WorkMesh MCP Skill

Use this skill when WorkMesh MCP tools are available.

Read `../../../skills/workmesh-shared/OPERATING_MODEL.md` before executing feature work. It is the canonical shared doctrine for router, CLI, and MCP operation.

## MCP mode rules
- Prefer MCP tools over shell commands when parity exists.
- Treat JSON as the canonical data contract.
- Use `render_*` tools for human-friendly structured output.
- Treat mutation tools as acknowledgement-first APIs.

## Bootstrap intent handling
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
- do not block feature work
- recommend canonical repo + worktree migration path

## MCP-specific helpers
### Multi-stream restore
```json
{"tool":"workstream_restore","format":"json"}
{"tool":"workstream_show","id":"<id-or-key>","restore":true,"format":"json"}
```

### High-signal loop
```json
{"tool":"next_tasks","format":"json","limit":10}
{"tool":"claim_task","task_id":"task-123","owner":"agent","minutes":60,"touch":true}
{"tool":"set_status","task_id":"task-123","status":"In Progress","touch":true}
{"tool":"add_note","task_id":"task-123","note":"...","section":"notes","touch":true}
{"tool":"set_status","task_id":"task-123","status":"Done","touch":true}
{"tool":"release_task","task_id":"task-123","touch":true}
```
