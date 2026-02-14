---
name: workmesh
description: Router skill for WorkMesh. Selects CLI-first or MCP-first workflow based on available capabilities and user preference.
---

# WorkMesh Router Skill

Use this skill to choose the operating mode and enforce the approved WorkMesh DX workflow.

## Mode selection
- Use `workmesh-mcp` when MCP tools are available and structured JSON tool calls are preferred.
- Use `workmesh-cli` when shell command execution is preferred.
- If user explicitly asks for one mode, do not mix modes unless asked.

## Install skills
```bash
workmesh --root . install --skills --profile all --scope project
workmesh --root . install --skills --profile cli --scope project
workmesh --root . install --skills --profile mcp --scope project
```

## Progressive DX policy
Always follow this stage order:
1. Start: `quickstart` + `context` + task loop.
2. Parallelize: one stream per git worktree.
3. Recover: `session resume` + context/truth rehydration.
4. Consolidate: migrate sibling clones into canonical repo + worktrees.

## Default behavior policy
- Prefer worktrees by default for parallel streams.
- Auto session updates are expected in interactive local workflows.
- Use explicit overrides only when needed:
  - enable: `--auto-session-save` / `WORKMESH_AUTO_SESSION=1`
  - disable: `--no-auto-session-save` / `WORKMESH_AUTO_SESSION=0`
- Config knobs:
  - `worktrees_default = true|false`
  - `auto_session_default = true|false`

## Ground rules
- Keep tasks small and outcome-based.
- Record dependencies and blockers so ready work is queryable.
- Use context to scope work and reduce session thrash.
- For parallel work, use separate worktrees and attach sessions to those worktrees.
- Claim before changes in multi-agent workflows.
- Capture stable feature decisions in Truth Ledger (`truth propose|accept|supersede`).
- On resume, rehydrate accepted truths before coding (`truth list --state accepted ...`).
- Keep task metadata complete: `Description`, `Acceptance Criteria`, `Definition of Done`.
- Move to `Done` only when description goals are met and acceptance criteria are satisfied.
- Treat `Code/config committed` and `Docs updated if needed` as hygiene, not completion criteria.
- Do not commit derived artifacts like `workmesh/.index/`.
