# WorkMesh

WorkMesh is a docs-first, MCP-ready project/task system that keeps planning state in plain text next to your code.

This repository contains:
- `workmesh` (CLI)
- `workmesh-core` (shared logic)
- `workmesh-mcp` (MCP server)
- `workmesh-service` (HTTP service runtime)

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

Task quality policy:
- Required sections in every task body: `Description`, `Acceptance Criteria`, `Definition of Done`.
- `Definition of Done` must include outcome-based completion criteria, not only hygiene checks.
- Setting status to `Done` is quality-gated (CLI and MCP).
- Legacy tasks can be normalized with migration tooling (`task_section_normalization` action).

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
workmesh-service --version
```

Install from release artifacts (`workmesh`, `workmesh-mcp`, `workmesh-service`) and verify versions.

### Build from source
```bash
git clone git@github.com:luislobo/workmesh.git
cd workmesh
cargo build -p workmesh
cargo build -p workmesh-mcp
cargo build -p workmesh-service
```

## MCP Setup
Configure your MCP client to run `workmesh-mcp` over stdio.

Codex example:
```toml
[mcp_servers.workmesh]
command = "/usr/local/bin/workmesh-mcp"
args = []
```

Full run/install/agent setup:
- [`docs/setup/run-modes-and-agent-mcp.md`](docs/setup/run-modes-and-agent-mcp.md)

## HTTP Service Mode
WorkMesh can also run as a local/LAN HTTP service runtime.

CLI management:
- Verify binary: `workmesh --root . service verify`
- Start service: `workmesh --root . service start --config ./service.toml`
- Install user systemd unit: `workmesh --root . service install-systemd --scope user --enable --start`
- Install system unit: `sudo workmesh --root . service install-systemd --scope system --enable --start`

Direct binary run:
```bash
workmesh-service --config ./service.toml
```

Systemd notes:
- `service install-systemd` writes or updates a unit file and runs `systemctl ... daemon-reload`.
- default unit name: `workmesh-service.service` (override with `--unit-name`).
- use `--dry-run --print-unit` to preview without writing files.

Key endpoints:
- `GET /v1/healthz`
- `GET /v1/readyz`
- `GET /v1/status`
- `GET /v1/metrics`
- `GET /v1/providers`
- `POST /v1/mcp/invoke`
- `POST /v1/admin/reload`

Provider namespaces:
- `workmesh`: task/context/truth/workstream/worktree/session tools
- `system`: service diagnostics (`ping`, `version`, `status`)
- `render`: native Rust terminal render tools (`render_table`, `render_kv`, `render_stats`, `render_progress`, `render_tree`, `render_diff`, `render_logs`, `render_alerts`, `render_list`, `render_chart_bar`, `render_sparkline`, `render_timeline`)

Render invocation example:
```bash
curl -s \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  http://127.0.0.1:4747/v1/mcp/invoke \
  -d '{"namespace":"render","tool":"render_list","arguments":{"data":[{"text":"Plan"},{"text":"Build"},{"text":"Validate"}],"configuration":{"ordered":true}}}'
```

Deprecation note:
- External Node `mcp-gui` is retired as the primary renderer path.
- Use `workmesh-service` `render` namespace for renderer workflows.

LAN safety baseline:
- default bind should remain localhost (`127.0.0.1`)
- if binding non-localhost, configure an auth token
- authenticated routes require `Authorization: Bearer <token>`

Docker sample:
- Files: [`docker/workmesh-service/`](docker/workmesh-service/)
- Build: `docker build -f docker/workmesh-service/Dockerfile -t workmesh-service:local .`
- Compose: `cd docker/workmesh-service && WORKMESH_REPO_ROOT=/abs/path WORKMESH_AUTH_TOKEN=<token> docker compose up --build -d`

## Defaults
Global config:
- `~/.workmesh/config.toml` (or `$WORKMESH_HOME/config.toml`)

Project config:
- `.workmesh.toml` (preferred)

Keys:
- `worktrees_default = true|false`
- `worktrees_dir = "<path>"`
- `auto_session_default = true|false`

Config helpers:
- Show: `workmesh --root . config show --json`
- Set (project): `workmesh --root . config set --scope project --key worktrees_dir --value "../myrepo.worktrees" --json`
- Set (global): `workmesh --root . config set --scope global --key auto_session_default --value true --json`

Archive default status filter (when `--status` is omitted):
- `Done`, `Cancelled`, `Canceled`, `Won't Do`, `Wont Do`
- override (explicit): pass `--status` one or more times to archive any specific state, including non-terminal states.

Auto session behavior:
- default ON for interactive non-CI terminals
- default OFF for CI/non-interactive contexts
- explicit override:
  - on: `--auto-session-save` or `WORKMESH_AUTO_SESSION=1`
  - off: `--no-auto-session-save` or `WORKMESH_AUTO_SESSION=0`

## Concurrency Integrity Foundation
Phase 0 storage guarantees are now active for tracking files.

- Critical tracking writes use lock-safe + atomic write primitives.
- Mutable snapshots are versioned and updated with CAS semantics:
  - `workmesh/context.json`
  - `$WORKMESH_HOME/sessions/current.json`
  - `$WORKMESH_HOME/worktrees/registry.json`
- Trailing malformed JSONL is tolerated for event readers and can be repaired safely.
- Recovery command path:
  - CLI: `workmesh --root . doctor --fix-storage --json`
  - MCP: `doctor` with `fix_storage=true`
- CLI and MCP share the same recovery behavior contract.

## Task Quality Guardrails
- `set-status <task> Done` is gated by task quality checks.
- Equivalent status mutations are also gated:
  - `set-field <task> status Done`
  - `bulk set-status --status Done`
  - `bulk set-field --field status --value Done`
- Validation behavior:
  - non-`Done` tasks with missing/incomplete required sections emit warnings
  - `Done` tasks with missing/incomplete sections (or hygiene-only DoD) emit errors
- Migration helper:
  - `workmesh --root . migrate audit|plan|apply --apply`
  - includes `task_section_normalization` for legacy task bodies missing required sections

## Workstreams
Workstreams let you manage multiple parallel streams of work in the same repo (often one git worktree per stream), with durable pointers to context, sessions, and per-stream scope/objective.

Start a new workstream (recommended):
- `workmesh --root . workstream create --name "OCA integration" --project <project-id> --objective "..." --json`

Behavior:
- When run from the canonical checkout and the repo has a real `HEAD` commit, `workstream create` auto-provisions a new git worktree by default (unless you pass `--existing` or explicit `--path/--branch`).
- Auto-provision uses `worktrees_dir` when set; otherwise it defaults to `<repo_parent>/<repo_name>.worktrees/`.
- When run from a non-canonical git worktree checkout, `workstream create` binds the workstream to the current worktree by default (no new worktree).
- If the target worktree path is already bound to a workstream, `workstream create` returns the existing workstream (`already_exists=true`) instead of creating a duplicate.

When a workstream is active in a worktree, `session save` and `worktree attach/detach` keep the stream's session/worktree pointers updated automatically.

After reboot (or losing terminals), run `workmesh --root . workstream restore --json` to get per-stream resume commands (path, session id, objective/scope, next task).

To get resume commands for a single stream:
- `workmesh --root . workstream show <id-or-key> --restore --json`

Lifecycle (pause/close/reopen/rename/set):
- `workmesh --root . workstream pause [<id-or-key>] --json`
- `workmesh --root . workstream close [<id-or-key>] --json`
- `workmesh --root . workstream reopen [<id-or-key>] --json`
- `workmesh --root . workstream rename [<id-or-key>] --name "..." --json`
- `workmesh --root . workstream set [<id-or-key>] --key ... --notes "..." --objective "..." --json`

Adopting multiple full clones into worktrees:
- `workmesh --root . worktree adopt-clone --from <path-to-clone> --apply --json`
- then bind a workstream to the created worktree with `workstream create --existing` (see the emitted plan).

Truth Ledger for durable decisions per stream:
- `workmesh --root . truth propose --title "..." --statement "..." --current --json`
- `workmesh --root . workstream show [<id-or-key>] --truth --json`

See [`docs/reference/commands.md`](docs/reference/commands.md) for `workstream ...` (CLI) and `workstream_*` (MCP tools).

## Documentation
- Codex-first onboarding: [`docs/getting-started.md`](docs/getting-started.md)
- Run modes and agent MCP setup: [`docs/setup/run-modes-and-agent-mcp.md`](docs/setup/run-modes-and-agent-mcp.md)
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

Common migration actions surfaced by `migrate plan`:
- `layout_backlog_to_workmesh`
- `focus_to_context`
- `task_section_normalization`
- `truth_backfill`
- `session_handoff_enrichment`
- `config_cleanup`
