# WorkMesh

WorkMesh is a docs-first, MCP-ready project/task system that keeps planning state in plain text next to your code.

This repository contains the Rust implementation:
- `workmesh` (CLI)
- `workmesh-core` (shared logic)
- `workmesh-mcp` (MCP server)

Agent-friendly format: [`README.json`](README.json) (kept in sync with this file).

Start here:
- Progressive DX guide: [`docs/getting-started.md`](docs/getting-started.md)
- Command reference: [`docs/reference/commands.md`](docs/reference/commands.md)
- Docs index: [`docs/README.md`](docs/README.md)

## Why WorkMesh
- Work stays in git with your source.
- Dependency-aware task selection (`next`, `ready`, `blockers`).
- Parallel-safe execution through worktrees + sessions.
- Durable feature decisions through the Truth Ledger.

## Install

### Prebuilt binaries (recommended)
Release archives include both binaries (`workmesh`, `workmesh-mcp`).

Pick a release:
```bash
workmesh_version="vX.Y.Z"
```

macOS/Linux:
```bash
gh release download "$workmesh_version" -R luislobo/workmesh \
  -p "workmesh-$workmesh_version-<target>.tar.gz"

tar -xzf "workmesh-$workmesh_version-<target>.tar.gz"
sudo install -m 0755 "workmesh-$workmesh_version-<target>/workmesh" /usr/local/bin/workmesh
sudo install -m 0755 "workmesh-$workmesh_version-<target>/workmesh-mcp" /usr/local/bin/workmesh-mcp
```

Windows (PowerShell):
```powershell
$workmesh_version = "vX.Y.Z"
gh release download $workmesh_version -R luislobo/workmesh `
  -p "workmesh-$workmesh_version-x86_64-pc-windows-msvc.zip"
Expand-Archive "workmesh-$workmesh_version-x86_64-pc-windows-msvc.zip" -DestinationPath . -Force
```

Verify:
```bash
workmesh --version
workmesh-mcp --version
```

### Build from source
```bash
git clone git@github.com:luislobo/workmesh.git
cd workmesh
cargo build -p workmesh-cli
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

Quick verification from chat:
1. Call `version`
2. Call `readme`
3. Call `doctor`

## Progressive DX Workflow
WorkMesh usage is staged. Use one canonical guide:

1. Start (single repo): `quickstart` -> `context set` -> `next` -> claim/work/done.
2. Parallelize (worktrees): one stream per worktree, each with explicit context/session.
3. Recover (reboot/resume): `session resume` -> `context show` -> `truth list` -> `next`.
4. Consolidate clones: move sibling clones into one canonical repo + worktrees.

Full procedural commands: [`docs/getting-started.md`](docs/getting-started.md).

## Core Model
- `context`: repo-local intent/scope pointer (`workmesh/context.json`).
- `truth`: durable validated decisions (`workmesh/truth/`).
- `sessions`: cross-repo continuity in `WORKMESH_HOME` (default `~/.workmesh`).
- `worktrees`: runtime isolation for parallel streams.

## Defaults And Config
Global config path:
- `~/.workmesh/config.toml` (or `$WORKMESH_HOME/config.toml`)

Project config path:
- `.workmesh.toml` (preferred)
- `.workmeshrc` (legacy alias)

Supported DX defaults:
- `worktrees_default = true|false`
- `auto_session_default = true|false`

Precedence:
1. CLI flags
2. Environment variables
3. Project config
4. Global config
5. Built-in defaults

Auto session behavior:
- Built-in default: enabled for interactive non-CI terminals.
- Built-in default: disabled in CI/non-interactive contexts.
- Explicit override:
  - enable: `--auto-session-save` or `WORKMESH_AUTO_SESSION=1`
  - disable: `--no-auto-session-save` or `WORKMESH_AUTO_SESSION=0`

## Command Surface
Project lifecycle:
- `quickstart`, `project-init`, `doctor`, `validate`, `archive`

Task flow:
- `list`, `show`, `next`, `ready`, `claim`, `set-status`, `note`, `release`

Context/truth:
- `context set|show|clear`
- `truth propose|accept|reject|supersede|show|list|validate`

Worktrees/sessions:
- `worktree list|create|attach|detach|doctor`
- `session save|list|show|resume`

Index/reporting:
- `index-rebuild|index-refresh|index-verify`
- `board`, `blockers`, `stats`, `graph-export`, `issues-export`

Full CLI + MCP mapping: [`docs/reference/commands.md`](docs/reference/commands.md).

## Legacy Note (Minimal)
If a repo still uses legacy `backlog/` layout or `focus.json`, use migration tooling:
```bash
workmesh --root . migrate audit
workmesh --root . migrate plan
workmesh --root . migrate apply --apply
```

Legacy migration guidance is intentionally minimized in primary DX docs.

## Repository Layout
- `crates/` Rust crates (`workmesh-cli`, `workmesh-core`, `workmesh-mcp`)
- `docs/` product and usage docs
- `skills/` embedded WorkMesh skills
- `workmesh/` task and state files for this repo

## Roadmap (Near Term)
- Clone-to-worktree onboarding helpers (`worktree onboard ...`).
- Streamlined multi-worktree session recovery scripts.
- Additional diagnostics for stale stream/session bindings.
