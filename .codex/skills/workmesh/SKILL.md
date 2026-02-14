---
name: workmesh
description: Router skill for WorkMesh. Selects CLI-first or MCP-first workflow based on available capabilities and user preference.
---

# WorkMesh Router Skill

Use this skill to provide a Codex-first WorkMesh experience.

## Mode selection
- If WorkMesh MCP tools are available, use `workmesh-mcp`.
- Otherwise use `workmesh-cli`.
- If the user explicitly requests one mode, honor it.

## Primary user intent
When the user says `bootstrap workmesh` (or equivalent), this skill must route into bootstrap behavior, not explain commands.

## Bootstrap behavior contract
On bootstrap intent, do the following in order:
1. Detect mode (MCP or CLI).
2. Detect repository state:
   - no WorkMesh data
   - modern WorkMesh layout
   - legacy layout/deprecated structure
   - clone-based stream workflow
3. Apply state-appropriate setup:
   - initialize if missing
   - migrate if legacy
   - validate and set context if modern
   - keep work unblocked and suggest worktree consolidation if clone-based
4. Return a short summary:
   - detected state
   - actions taken
   - current context
   - next actionable tasks

## Working mode contract
After bootstrap, if user asks to work on a feature, maintain WorkMesh continuously:
- create/update PRD docs
- create/maintain tasks
- keep context current
- capture durable decisions in Truth Ledger

## Rules
- Keep task metadata complete: `Description`, `Acceptance Criteria`, `Definition of Done`.
- Move task to `Done` only when description goals and acceptance criteria are fully satisfied.
- Treat `Code/config committed` and `Docs updated if needed` as hygiene checks.
- Do not commit derived artifacts like `workmesh/.index/`.
