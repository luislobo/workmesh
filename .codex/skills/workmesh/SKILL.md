---
name: workmesh
description: Router skill for WorkMesh. Selects CLI-first or MCP-first workflow based on available capabilities and user preference.
---

# WorkMesh Router Skill

Use this skill to pick the right WorkMesh operating mode.

## Mode selection
- Use `workmesh-mcp` when MCP tools are available and you want tool-call workflows.
- Use `workmesh-cli` when running through shell commands is preferred (token-lean, no MCP server).
- If the user explicitly requests one mode, do not mix modes unless asked.

## Install skills
Install one or more embedded skills from the WorkMesh binary:
```bash
# install all profiles (router + cli + mcp) into project skill folders
workmesh --root . install --skills --profile all --scope project

# install only CLI profile
workmesh --root . install --skills --profile cli --scope project

# install only MCP profile
workmesh --root . install --skills --profile mcp --scope project
```

## Ground rules (applies to all modes)
- Keep tasks small and outcome-based.
- Record dependencies and blockers so ready work is queryable.
- Use focus to scope work and reduce context thrash.
- Claim before changes in multi-agent workflows.
- Do not commit derived artifacts like `workmesh/.index/`.
