---
name: workmesh-cli
description: CLI-first WorkMesh workflow. Use when agents should run shell commands instead of MCP tool calls.
---

# WorkMesh CLI Skill

Use this skill when WorkMesh MCP is not available.

## Bootstrap intent handling
If user says `bootstrap workmesh`, execute this flow:

1. Discover state:
```bash
workmesh --root . doctor --json
```

2. If no WorkMesh structure:
```bash
workmesh --root . quickstart <project-id> --feature "<feature-name>" --agents-snippet
workmesh --root . context set --project <project-id> --objective "<objective>"
```

3. If legacy structure exists:
```bash
workmesh --root . migrate audit
workmesh --root . migrate plan
workmesh --root . migrate apply --apply
```

4. If modern structure exists:
```bash
workmesh --root . context show --json
workmesh --root . truth list --state accepted --limit 20 --json
workmesh --root . next --json
```

5. If clone-based stream workflow is detected:
- Do not block feature work.
- Recommend canonical repo + worktree migration path.
- Helper command (safe by default; dry-run plan unless `--apply` is passed):
```bash
workmesh --root . worktree adopt-clone --from <path-to-clone> --json
workmesh --root . worktree adopt-clone --from <path-to-clone> --apply --json
```

## Feature work contract
When user says to use WorkMesh for feature development:
- maintain PRD/task documentation continuously
- keep context current
- maintain acceptance criteria and definition of done quality
- capture stable decisions as truths (use `truth propose --current` for stream-aware defaults)
- use the CLI `render` subcommand for pretty tables/trees/stats/timelines when structured human-friendly output is needed

## Contributor architecture
- Shared tool metadata, response-policy helpers, and adapter-neutral tooling helpers belong in `workmesh-tools`.
- CLI-only parsing/presentation belongs in `workmesh`.
- MCP transport glue belongs in `workmesh-mcp-server`.
- Do not reintroduce a CLI dependency on `workmesh-mcp-server`.
- If exact MCP input-schema detail is required, prefer MCP `tool_info`; CLI `tool-info` mirrors shared metadata/examples.

## Mutation response contract
- Treat WorkMesh writes as acknowledgement-first operations.
- Do not assume a write should echo the full updated object.
- When using MCP-backed workflows, default to minimal responses and request `verbose=true` only when the richer payload is actually needed.
- For bulk mutations, expect compact failure identification by default (`failed_ids`) rather than full per-item result objects.
- Prefer a follow-up read command when you need current full state after a mutation.

## Multi-stream restore (after reboot)
If the user runs multiple workstreams in parallel (often one git worktree per stream), use:
```bash
workmesh --root . workstream restore --json
```
Each entry includes a `resume_script` with the exact commands to run in that worktree (session resume, context show, next).

For a single stream:
```bash
workmesh --root . workstream show <id-or-key> --restore --json
```

## Workstream lifecycle helpers
- Pause/close when parking a stream:
  - `workmesh --root . workstream pause [<id-or-key>] --json`
  - `workmesh --root . workstream close [<id-or-key>] --json`
- Reopen when resuming:
  - `workmesh --root . workstream reopen [<id-or-key>] --json`
- Update key/notes/snapshot:
  - `workmesh --root . workstream set [<id-or-key>] --key ... --notes "..." --objective "..." --json`

## High-signal loop
- `workmesh --root . next --json`
- `workmesh --root . claim <task-id> <owner> --minutes 60`
- `workmesh --root . set-status <task-id> "In Progress"`
- `workmesh --root . note <task-id> "<note>"`
- `workmesh --root . set-status <task-id> Done`
- `workmesh --root . release <task-id>`

## Defaults and overrides
- Worktree guidance default: `worktrees_default`.
- Default worktree directory (for auto-provision): `worktrees_dir`.
- Auto session update default: `auto_session_default`.
- One-off overrides:
  - `--auto-session-save`
  - `--no-auto-session-save`

Config helper (CLI):
```bash
workmesh --root . config show --json
workmesh --root . config set --scope global --key auto_session_default --value true --json
workmesh --root . config set --scope project --key worktrees_dir --value "../myrepo.worktrees" --json
```

## Rules
- Prefer `--json` when parsing output.
- If the user wants pretty rendered tables/trees/charts, prefer MCP render tools when available; otherwise use `workmesh --root . render ...` instead of manually simulating rich output.
- In CLI mode, treat JSON as the canonical data contract and plain text as a convenience view.
- Use Markdown output only for content that is meant to be reused in docs, PRs, or notes.
- Keep dependencies and blockers current.
- Keep task metadata complete: `Description`, `Acceptance Criteria`, `Definition of Done`.
- Ensure `Definition of Done` includes outcome-based completion criteria, not only hygiene checks.
- Move to `Done` only when goals and criteria are fully met.
- Treat all status mutation paths as equivalent for `Done` gating (including `set-field status Done` and bulk variants).
- Do not bypass WorkMesh storage primitives for tracking files.
