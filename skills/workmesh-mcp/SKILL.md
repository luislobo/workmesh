---
name: workmesh-mcp
description: MCP-first WorkMesh workflow. Use when WorkMesh MCP tools are available.
---

# WorkMesh MCP Skill

Read `OPERATING_MODEL.md` in this folder first. It is the shared doctrine for router, CLI, and MCP operation.

## MCP mode rules
- Prefer MCP tools over shell commands when parity exists.
- Use `render_*` tools for human-friendly structured output.
- Treat mutation tools as acknowledgement-first APIs.

## Bootstrap contract
Use `doctor`, `quickstart`, `migrate_*`, `context_show`, `truth_list`, and `next_tasks` according to repo state.

## MCP helpers
- `workstream_restore`
- `workstream_show` with `restore=true`
