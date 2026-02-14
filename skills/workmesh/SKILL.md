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
- Worktree guidance is default-on; users can disable globally via `~/.workmesh/config.toml` (`worktrees_default = false`) and override per repo in `.workmesh.toml`.
- Claim before changes in multi-agent workflows.
- Capture stable feature decisions in the Truth Ledger (`truth propose|accept|supersede`) so knowledge survives session/worktree churn.
- On resume, rehydrate accepted truths for the active scope before coding (`truth list --state accepted ...`).
- Keep task metadata complete and current: `Description`, `Acceptance Criteria`, and `Definition of Done`.
- Move a task to `Done` only when the task goals in `Description` are met and all `Acceptance Criteria` are satisfied.
- Treat `Code/config committed` and `Docs updated if needed` as hygiene checks, not the core completion criteria.
- Do not commit derived artifacts like `workmesh/.index/`.
