---
name: workmesh-cli
description: CLI-first WorkMesh workflow. Use when agents should run shell commands instead of MCP tool calls.
---

# WorkMesh CLI Skill

Use this skill when interacting with WorkMesh through shell commands only.

## When to use
- MCP is not configured.
- You want lower token overhead from concise shell commands.
- You want explicit command history in terminal output.

## Setup
```bash
# install CLI-focused skill in the current project
workmesh --root . install --skills --profile cli --scope project
```

## Baseline checks
```bash
workmesh --root . doctor --json
workmesh --root . focus show --json
```

## High-signal command loop
- Next candidates: `workmesh --root . next --json`
- Candidate set: `workmesh --root . ready --json`
- Start work: `workmesh --root . claim <task-id> <owner> --minutes 60`
- Mark active: `workmesh --root . set-status <task-id> "In Progress"`
- Capture context: `workmesh --root . note <task-id> "<note>"`
- Finish: `workmesh --root . set-status <task-id> Done`
- Release: `workmesh --root . release <task-id>`

## Grammar-style workflows
Notation:
- `[]` optional
- `{}` repeatable
- `->` then

Bootstrap:
```text
quickstart -> focus set -> list --status "To Do" -> next
```

Daily loop:
```text
focus show -> next -> claim -> set-status(In Progress) -> work -> note -> set-status(Done) -> release
```

Continuity:
```text
session save -> stop -> session resume -> focus show -> next -> claim
```

Hygiene:
```text
doctor -> blockers -> board --focus -> validate -> index-refresh
```

## Useful views
- Board: `workmesh --root . board --by status --focus`
- Blockers: `workmesh --root . blockers`
- Graph export: `workmesh --root . graph-export --pretty`

## Rules
- Prefer `--json` for agent parsing.
- Keep dependencies current when status changes.
- Do not commit derived `workmesh/.index/` files.
