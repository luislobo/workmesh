---
name: workmesh
description: Router skill for WorkMesh. Selects CLI-first or MCP-first workflow based on available capabilities and user preference.
---

# WorkMesh Router Skill

Use this skill to provide a Codex-first WorkMesh experience.

Read `../../../skills/workmesh-shared/OPERATING_MODEL.md` before executing feature work. It is the canonical shared doctrine for router, CLI, and MCP operation.

## Mode selection
- If WorkMesh MCP tools are available, use MCP mode.
- Otherwise use CLI mode.
- If the user explicitly requests one mode, honor it.
- When using the CLI, always pass `--root <repo>` unless the workflow already establishes it.

## Bootstrap contract
When the user says `bootstrap workmesh` or equivalent:
1. detect mode
2. detect repository state:
   - no WorkMesh data
   - modern WorkMesh layout
   - legacy/deprecated layout
   - clone-based parallel stream workflow
3. apply the correct setup path:
   - initialize if missing
   - migrate if legacy
   - validate and continue if modern
   - keep work unblocked and recommend worktree adoption if clone-based
4. return a short summary:
   - detected state
   - actions taken
   - current context
   - next actionable tasks

## Runtime expectations
- Enforce the operating procedure from `OPERATING_MODEL.md`.
- Keep WorkMesh continuously updated during feature work.
- Prefer `truth propose --current` or equivalent current-scope truth capture when a decision should persist.
- Use `workstream restore` when recovering from reboot/lost terminals.
- Use `config show|set|unset` rather than asking users to edit config files manually.
- Use native render tools when the user wants structured human-friendly output.
