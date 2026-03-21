# WorkMesh

WorkMesh is a docs-first project/task system for developers and coding agents.

It keeps planning state in plain text next to your code:
- tasks in Markdown
- repo-local context in `workmesh/context.json`
- durable decisions in `workmesh/truth/`
- global continuity in `~/.workmesh/`

This repository contains:
- `workmesh`: CLI
- `workmesh-core`: shared logic
- `workmesh-mcp`: MCP stdio server

Agent-readable mirror: [`README.json`](README.json)

## What It Is For
Use WorkMesh when you want:
- task tracking that lives with the repo
- chat-driven development without losing context
- durable decisions and task history
- parallel work across worktrees
- CLI and MCP access to the same workflow

Use it for software delivery orchestration, not as a generic ticketing SaaS.

## Recommended Developer Workflow
If you work inside Codex, this is the intended path:

1. `cd` into a repo
2. run `codex` or `codex resume`
3. prompt:

```text
Bootstrap WorkMesh in this repo. Use MCP if available, otherwise CLI.
```

4. then prompt:

```text
Use WorkMesh for this feature end to end. Create/update PRD, create and maintain tasks with acceptance criteria and definition of done, keep context current, and track stable decisions in Truth Ledger.
```

That is the primary experience. You should not need to memorize a command matrix to get started.

## What Bootstrap Does
`bootstrap` detects repo state and applies the correct path:
- no WorkMesh data: initialize docs/tasks/context
- modern WorkMesh repo: validate, show context, suggest next work
- legacy layout: migrate to modern layout
- clone-based workflow: keep work moving and recommend worktree consolidation

## Quick Start

### Install
Verify binaries:

```bash
workmesh --version
workmesh-mcp --version
```

Build from source:

```bash
git clone git@github.com:luislobo/workmesh.git
cd workmesh
cargo build -p workmesh
cargo build -p workmesh-mcp
```

### Configure MCP
Codex MCP example:

```toml
[mcp_servers.workmesh]
command = "/usr/local/bin/workmesh-mcp"
args = []
```

Full setup details:
- [`docs/setup/run-modes-and-agent-mcp.md`](docs/setup/run-modes-and-agent-mcp.md)

## If You Prefer CLI
Direct bootstrap:

```bash
workmesh --root . bootstrap --project-id <project-id> --feature "<feature-name>" --json
```

Useful direct commands:

```bash
workmesh --root . context show --json
workmesh --root . next --json
workmesh --root . list --json
workmesh --root . workstream restore --json
```

Renderer fallback when MCP render tools are unavailable:

```bash
workmesh --root . render table --data '[{"task":"task-001","status":"Done"}]'
```

CLI parity helpers:
- `workmesh --root . readme --json`
- `workmesh --root . tool-info render_table --json`
- `workmesh --root . skill-content --json`
- `workmesh --root . project-management-skill --json`
- `workmesh --root . next-tasks --json`

The CLI also accepts MCP-style aliases such as:
- `list_tasks`
- `show_task`
- `truth_list`
- `workstream_list`
- `render_table`

## Core Concepts

### Tasks
Task files live under `workmesh/tasks/` (or `.workmesh/tasks/`).

Required body sections:
- `Description`
- `Acceptance Criteria`
- `Definition of Done`

`Definition of Done` must be outcome-based, not just hygiene items like “code committed” or “docs updated”.

### Context
`workmesh/context.json` is the repo-local pointer for current scope:
- project
- epic
- objective
- task working set
- active workstream

### Truth Ledger
Truth records in `workmesh/truth/` capture durable decisions you want agents and humans to keep respecting.

### Workstreams and Worktrees
Workstreams are the parallel-work model.

Typical pattern:
- one repo
- multiple git worktrees
- one active workstream per worktree

Start one:

```bash
workmesh --root . workstream create --name "OCA integration" --project <project-id> --objective "..." --json
```

Restore after reboot:

```bash
workmesh --root . workstream restore --json
```

Adopt old full clones into worktrees:

```bash
workmesh --root . worktree adopt-clone --from <path-to-clone> --apply --json
```

## Mutation Response Contract
To save tokens, MCP mutation tools default to compact acknowledgements instead of returning full objects.

Default pattern:

```json
{ "ok": true, "id": "task-001", "status": "Done" }
```

Bulk default pattern:

```json
{ "ok": false, "updated_count": 7, "failed_count": 2, "failed_ids": ["task-003", "task-009"] }
```

When you need richer post-write state, pass `verbose=true`.

Use read tools for full state:
- `show_task`
- `context_show`
- `truth_show`
- `session_show`
- `workstream_show`

## Safety and Storage
Critical tracking files use lock-safe and atomic storage primitives.

Important guarantees:
- versioned mutable snapshots
- CAS-based updates
- append-safe JSONL event streams
- recovery through `doctor --fix-storage`

Examples:

```bash
workmesh --root . doctor --fix-storage --json
```

Archive defaults are safety-first:
- default archive statuses: `Done`, `Cancelled`, `Canceled`, `Won't Do`, `Wont Do`
- non-terminal states are archived only with explicit `--status`

## Docs Map
- Getting started: [`docs/getting-started.md`](docs/getting-started.md)
- Run modes and MCP setup: [`docs/setup/run-modes-and-agent-mcp.md`](docs/setup/run-modes-and-agent-mcp.md)
- Command reference: [`docs/reference/commands.md`](docs/reference/commands.md)
- Docs index: [`docs/README.md`](docs/README.md)
- Sample project: [`docs/samples/workmesh-demo/README.md`](docs/samples/workmesh-demo/README.md)

## Maintainers

### Changelog Discipline
When cutting a release:
- move relevant items from `CHANGELOG.md` `[Unreleased]` into a versioned section
- update compare links at the bottom of `CHANGELOG.md`
- bump `Cargo.toml`
- tag the release

### Docs Sync Rule
If you change install, quickstart, MCP setup, commands, layout, or roadmap:
- update `README.md`
- update `README.json`
- keep them in the same commit

### Release Notes
Binary versions include build metadata automatically:
- `X.Y.Z+git.<commit_count>.<sha>[.dirty]`

That gives traceability without editing versions on every build.
