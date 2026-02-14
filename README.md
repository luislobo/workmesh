# WorkMesh

WorkMesh is a docs-first, MCP-ready project/task system that keeps planning state in plain text next to your code.

This repository contains:
- `workmesh` (CLI)
- `workmesh-core` (shared logic)
- `workmesh-mcp` (MCP server)

Agent-friendly format: [`README.json`](README.json) (kept in sync with this file).

## Codex-First Workflow (Recommended)
If you work inside Codex, the happy path is intentionally short.

1. `cd` into any repo directory.
2. Start Codex:
   - `codex` for a new chat
   - `codex resume` for an existing chat
3. Tell Codex:
   - `Bootstrap WorkMesh in this repo. Use MCP if available, otherwise CLI.`
4. Start feature work:
   - `Use WorkMesh to document this feature end to end: tasks, PRD updates, and decisions.`

That is the primary workflow. You should not need to memorize command lists.

## What "Bootstrap WorkMesh" Means
When you ask Codex to bootstrap, it should detect repository state and do the right thing:

- No WorkMesh data yet:
  - initialize WorkMesh docs/tasks and seed context.
- Existing modern WorkMesh layout:
  - validate health, show current context, and pick next work.
- Legacy WorkMesh/backlog layout:
  - run migration audit/plan/apply, then continue on modern layout.
- Long-lived clone-based branch workflow:
  - keep you unblocked now, and recommend migration to canonical repo + worktrees.

## Feature Workflow Prompt
After bootstrap, you can stay in normal conversation and be explicit once:

`Use WorkMesh for this feature. Create/update PRD, break down tasks with acceptance criteria and definition of done, keep context current, and track decisions in Truth Ledger.`

Codex should then operate with WorkMesh continuously while you discuss implementation.

## CLI Fallback (Single Command)
If you want direct CLI execution instead of chat orchestration:

```bash
workmesh --root . bootstrap --project-id <project-id> --feature "<feature-name>" --json
```

`bootstrap` detects repo state (new/modern/legacy), initializes or migrates as needed, seeds missing context, and returns next-task recommendations.

## Install

### Prebuilt binaries (recommended)
```bash
workmesh --version
workmesh-mcp --version
```

Install from release artifacts (`workmesh`, `workmesh-mcp`) and verify versions.

### Build from source
```bash
git clone git@github.com:luislobo/workmesh.git
cd workmesh
cargo build -p workmesh
cargo build -p workmesh-mcp
```

## MCP Setup
Configure your MCP client to run `workmesh-mcp` over stdio.

Codex example:
```toml
[mcp_servers.workmesh]
command = "/usr/local/bin/workmesh-mcp"
args = []
```

## Defaults
Global config:
- `~/.workmesh/config.toml` (or `$WORKMESH_HOME/config.toml`)

Project config:
- `.workmesh.toml` (preferred)

Keys:
- `worktrees_default = true|false`
- `auto_session_default = true|false`

Auto session behavior:
- default ON for interactive non-CI terminals
- default OFF for CI/non-interactive contexts
- explicit override:
  - on: `--auto-session-save` or `WORKMESH_AUTO_SESSION=1`
  - off: `--no-auto-session-save` or `WORKMESH_AUTO_SESSION=0`

## Documentation
- Codex-first onboarding: [`docs/getting-started.md`](docs/getting-started.md)
- Command catalog: [`docs/reference/commands.md`](docs/reference/commands.md)
- Documentation index: [`docs/README.md`](docs/README.md)

## Legacy Note
`bootstrap` already handles legacy `backlog/` / `focus.json` structures automatically.

If you need explicit migration controls:
```bash
workmesh --root . migrate audit
workmesh --root . migrate plan
workmesh --root . migrate apply --apply
```
