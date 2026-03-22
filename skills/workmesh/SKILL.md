---
name: workmesh
description: Router skill for WorkMesh. Selects CLI-first or MCP-first workflow based on available capabilities and user preference.
---

# WorkMesh Router Skill

Read `OPERATING_MODEL.md` in this folder first. It is the shared doctrine for router, CLI, and MCP operation.

## Mode selection
- If WorkMesh MCP tools are available, use MCP mode.
- Otherwise use CLI mode.
- If the user explicitly requests one mode, honor it.
- When using the CLI, always pass `--root <repo>` unless the workflow already establishes it.

## Bootstrap contract
When the user says `bootstrap workmesh` or equivalent:
1. detect mode
2. detect repository state
3. apply the correct setup path
4. return detected state, actions taken, current context, and next actionable tasks

## Runtime expectations
- Enforce the operating procedure from `OPERATING_MODEL.md`.
- Keep WorkMesh continuously updated during feature work.
- Use workstream restore for deterministic reboot recovery.
- Use render tools for structured output.
