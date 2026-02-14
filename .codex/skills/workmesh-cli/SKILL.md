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

## Feature work contract
When user says to use WorkMesh for feature development:
- maintain PRD/task documentation continuously
- keep context current
- maintain acceptance criteria and definition of done quality
- capture stable decisions as truths

## High-signal loop
- `workmesh --root . next --json`
- `workmesh --root . claim <task-id> <owner> --minutes 60`
- `workmesh --root . set-status <task-id> "In Progress"`
- `workmesh --root . note <task-id> "<note>"`
- `workmesh --root . set-status <task-id> Done`
- `workmesh --root . release <task-id>`

## Defaults and overrides
- Worktree guidance default: `worktrees_default`.
- Auto session update default: `auto_session_default`.
- One-off overrides:
  - `--auto-session-save`
  - `--no-auto-session-save`

## Rules
- Prefer `--json` when parsing output.
- Keep dependencies and blockers current.
- Keep task metadata complete: `Description`, `Acceptance Criteria`, `Definition of Done`.
- Move to `Done` only when goals and criteria are fully met.
