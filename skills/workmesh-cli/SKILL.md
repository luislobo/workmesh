---
name: workmesh-cli
description: CLI-first WorkMesh workflow. Use when agents should run shell commands instead of MCP tool calls.
---

# WorkMesh CLI Skill

Read `references/OPERATING_MODEL.md` first. It is the canonical shared doctrine for router, CLI, and MCP operation.

## CLI mode rules
- Always pass `--root <repo>`.
- Prefer `--json` when parsing output.
- Read `config show --json` when task-quality policy or storage roots might matter.
- Keep one active implementation task at a time and update the task notes before/during coding.
- Use `workmesh --root <repo> render ...` before hand-formatting structured output.

## Bootstrap contract
Use `doctor`, `quickstart`, `migrate`, `context show`, `truth list`, and `next` according to repo state.

## CLI helpers
- `workmesh --root . workstream restore --json`
- `workmesh --root . workstream show <id-or-key> --restore --json`
