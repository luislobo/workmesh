# WorkMesh Documentation

This is the canonical human guide for WorkMesh.

If you read only one document, read this one. The rest of the docs are supporting references.

## 1. What WorkMesh Is

WorkMesh is a docs-first project and task system for developers and coding agents.

It is built for:
- repo-local task tracking
- chat-driven development with durable context
- stable decision tracking through Truth Ledger
- parallel feature work across worktrees
- the same workflow through CLI and MCP

It is not built to be:
- a hosted ticketing product
- a remote SaaS project manager

## 2. How State Is Stored

WorkMesh keeps operational state close to the repo:
- `workmesh/tasks/`: Markdown task files
- `workmesh/context.json`: repo-local working scope
- `workmesh/truth/`: durable decisions
- `docs/projects/<project-id>/`: PRDs, decisions, updates

Global continuity lives under `~/.workmesh/`:
- sessions
- worktree/workstream registries
- indexes and integrity metadata

## 3. Install

Verify installed binaries:

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

## 4. Agent Setup

Codex MCP example:

```toml
[mcp_servers.workmesh]
command = "/usr/local/bin/workmesh-mcp"
args = []
```

Supported run modes:
- CLI: `workmesh`
- MCP stdio: `workmesh-mcp`

For lower-level setup details, see:
- [`docs/setup/run-modes-and-agent-mcp.md`](setup/run-modes-and-agent-mcp.md)

## 5. Recommended Workflow

This project is Codex-first. The intended usage is prompt-driven, not command-matrix-driven.

Day-to-day entry flow:
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

That is the main path. You should not need to memorize commands to get started.

## 6. Bootstrap Behavior

`bootstrap` is designed to handle mixed repo states:
- brand new repo with no WorkMesh data
- modern WorkMesh repo
- legacy backlog/focus layout
- long-lived clone workflow that should eventually move to worktrees

Expected behavior:
- initialize when no WorkMesh data exists
- validate and continue when the modern layout already exists
- migrate deprecated structures when needed
- keep work moving and recommend worktree adoption when the repo uses parallel clones

## 7. Core Concepts

### Tasks

Task files live under `workmesh/tasks/` or `.workmesh/tasks/`.

Required task sections:
- `Description`
- `Acceptance Criteria`
- `Definition of Done`

`Definition of Done` must be outcome-based. It cannot be only hygiene lines like “code committed” or “docs updated”.

### Context

`workmesh/context.json` stores the current repo-local scope:
- project
- epic
- objective
- task working set
- active workstream

### Truth Ledger

Truth records live under `workmesh/truth/`.

Use them for decisions that must survive:
- restarts
- agent changes
- worktree changes
- feature handoffs

### Workstreams and Worktrees

Workstreams are the parallel-work model.

Typical pattern:
- one canonical repo
- multiple git worktrees
- one active workstream per worktree

Useful commands:

```bash
workmesh --root . workstream create --name "OCA integration" --project <project-id> --objective "..." --json
workmesh --root . workstream restore --json
workmesh --root . worktree adopt-clone --from <path-to-clone> --apply --json
```

## 8. Daily Work Pattern

A good WorkMesh-driven feature flow looks like this:
1. create or update the PRD
2. create tasks that support the real work
3. keep `Description`, `Acceptance Criteria`, and `Definition of Done` complete
4. keep repo-local context current
5. capture durable feature truths when a decision should persist
6. make atomic commits per task or coherent task slice
7. archive completed tasks after the work is actually done

## 9. Restore After Reboot

Single repo/worktree:
1. `cd <repo-or-worktree>`
2. `codex resume`
3. ask Codex to restore context, truths, and next tasks

Multiple active workstreams:

```bash
workmesh --root . workstream restore --json
```

The restore output gives you:
- `worktree_path`
- `session_id`
- `context`
- `next_task`
- `resume_script`

That is the deterministic recovery path after losing terminals or rebooting.

## 10. CLI Fallback

If you are not using MCP, these are the main direct commands:

```bash
workmesh --root . bootstrap --project-id <project-id> --feature "<feature-name>" --json
workmesh --root . context show --json
workmesh --root . next --json
workmesh --root . list --json
workmesh --root . workstream restore --json
```

CLI render fallback:

```bash
workmesh --root . render table --data '[{"task":"task-001","status":"Done"}]'
```

## 11. Important Command Families

### Read and navigation
- `list`
- `show`
- `next`
- `next-tasks`
- `ready`
- `board`
- `blockers`
- `stats`

### Task mutation
- `add`
- `add-discovered`
- `set-status`
- `set-field`
- `label-add` / `label-remove`
- `dep-add` / `dep-remove`
- `note`
- `set-body`
- `set-section`
- `claim`
- `release`

### Context and truth
- `context show|set|clear`
- `truth propose|accept|reject|supersede|show|list|validate`

### Workstream runtime
- `workstream create|show|list|switch|pause|close|reopen|rename|restore`
- `worktree list|attach|detach|adopt-clone`

### Diagnostics and migration
- `doctor`
- `validate`
- `migrate audit|plan|apply`
- `truth migrate audit|plan|apply`

For the exhaustive surface, see:
- [`docs/reference/commands.md`](reference/commands.md)

## 12. Renderers

WorkMesh has native renderers for human-friendly output:
- `table`
- `kv`
- `stats`
- `list`
- `progress`
- `tree`
- `diff`
- `logs`
- `alerts`
- `chart-bar`
- `sparkline`
- `timeline`

Use MCP render tools first when available.
Use CLI `render ...` as the local fallback.

## 13. Mutation Response Contract

MCP mutation tools default to compact acknowledgements to save tokens.

Typical response:

```json
{ "ok": true, "id": "task-001", "status": "Done" }
```

Bulk default:

```json
{ "ok": false, "updated_count": 3, "failed_count": 1, "failed_ids": ["task-009"] }
```

If you need richer post-write state:
- pass `verbose=true`
- or call the matching read tool afterwards

## 14. Architecture

Current crate boundaries:
- `workmesh-core`: domain logic, storage, state
- `workmesh-render`: renderers
- `workmesh-tools`: shared tool contract
- `workmesh`: CLI adapter
- `workmesh-mcp-server`: MCP adapter
- `workmesh-mcp`: stdio wrapper

Contributor rule:
- shared tool semantics go into `workmesh-tools`
- CLI-only behavior goes into `workmesh`
- MCP transport-only behavior goes into `workmesh-mcp-server`
- domain/state/storage logic goes into `workmesh-core`

Architecture diagrams:
- [`docs/architecture.md`](architecture.md)

## 15. Safety and Storage

Critical tracking files use lock-safe and atomic storage primitives.

Important guarantees:
- versioned mutable snapshots
- CAS-based writes
- append-safe JSONL event streams
- integrity recovery through doctor

Useful repair path:

```bash
workmesh --root . doctor --fix-storage --json
```

Archive defaults are intentionally conservative:
- `Done`
- `Cancelled`
- `Canceled`
- `Won't Do`
- `Wont Do`

Non-terminal statuses are archived only when explicitly requested.

## 16. Supporting Documents

These remain useful, but they are no longer the primary entrypoint:
- [`docs/architecture.md`](architecture.md)
- [`docs/getting-started.md`](getting-started.md)
- [`docs/reference/commands.md`](reference/commands.md)
- [`docs/setup/run-modes-and-agent-mcp.md`](setup/run-modes-and-agent-mcp.md)
- [`docs/test-coverage.md`](test-coverage.md)
- [`docs/samples/workmesh-demo/README.md`](samples/workmesh-demo/README.md)
