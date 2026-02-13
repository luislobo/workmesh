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
- Use context to scope work and reduce thrash between sessions.
- For parallel agent work, prefer separate git worktrees and attach sessions to those worktrees.
- Claim before changes in multi-agent workflows.
- Keep task metadata complete and current: `Description`, `Acceptance Criteria`, and `Definition of Done`.
- Move a task to `Done` only when the task goals in `Description` are met and all `Acceptance Criteria` are satisfied.
- Treat `Code/config committed` and `Docs updated if needed` as hygiene checks, not the core completion criteria.
- Do not commit derived artifacts like `workmesh/.index/`.
