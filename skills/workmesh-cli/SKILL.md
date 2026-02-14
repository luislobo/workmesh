---
name: workmesh-cli
description: CLI-first WorkMesh workflow. Use when agents should run shell commands instead of MCP tool calls.
---

# WorkMesh CLI Skill

Use this skill for shell-first WorkMesh workflows.

## Baseline checks
```bash
workmesh --root . doctor --json
workmesh --root . context show --json
workmesh --root . worktree list --json
workmesh --root . truth list --state accepted --limit 20 --json
```

## Progressive loops

Stage 1: Start
```text
quickstart -> context set -> next -> claim -> set-status(In Progress) -> note -> set-status(Done) -> release
```

Stage 2: Parallelize
```text
worktree create -> cd <worktree> -> session save -> worktree attach -> context show -> next -> claim
```

Stage 3: Recover
```text
worktree list -> cd <worktree> -> session resume -> context show -> truth list(accepted) -> next
```

Stage 4: Consolidate clones
```text
audit sibling clones -> ensure clean -> create canonical worktree per stream -> session save/attach -> archive old clone later
```

## High-signal commands
- Next candidates: `workmesh --root . next --json`
- Candidate set: `workmesh --root . ready --json`
- Start work: `workmesh --root . claim <task-id> <owner> --minutes 60`
- Mark active: `workmesh --root . set-status <task-id> "In Progress"`
- Capture context: `workmesh --root . note <task-id> "<note>"`
- Finish: `workmesh --root . set-status <task-id> Done`
- Release: `workmesh --root . release <task-id>`

## Defaults and overrides
- Worktree guidance defaults to ON (`worktrees_default`).
- Auto session updates should run in interactive local workflows by default (`auto_session_default`).
- One-off overrides:
  - force ON: `--auto-session-save`
  - force OFF: `--no-auto-session-save`

## Rules
- Prefer `--json` for parsing.
- Keep dependencies current when status changes.
- Persist durable decisions as truths scoped by project/epic/worktree/session when possible.
- Keep task metadata complete: `Description`, `Acceptance Criteria`, `Definition of Done`.
- Move to `Done` only when description goals + acceptance criteria are satisfied.
- Treat `Code/config committed` and `Docs updated if needed` as hygiene checks.
- Do not commit derived `workmesh/.index/` files.
