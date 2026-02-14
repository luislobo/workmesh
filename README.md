# WorkMesh

WorkMesh is a docs-first, MCP-ready project and task system that keeps work in plain text,
versioned alongside your code. It is designed for human+agent workflows: deterministic
commands, dependency-aware planning, and restartable sessions.

This repository contains the Rust implementation (CLI + core + MCP server).

AI-friendly format: see `README.json` (keep it in sync with this file).
Agents can also fetch it via MCP: tool `readme`.

Start here: `docs/getting-started.md`
Command reference (CLI + MCP): `docs/reference/commands.md`

## Why WorkMesh
- Keep planning artifacts close to code and in git.
- Make dependencies explicit so "ready work" is queryable.
- Provide reliable handoff between sessions via checkpoints.
- Enable agent-safe coordination (leases/claims, stable ordering).

## DX workflow (diagram)
This diagram shows the "moments" where WorkMesh commands happen, and who/what they touch:
- Phase 1: bootstrap a repo (run once)
- Phase 2: daily loop (many times)
- Phase 3: continuity (restart/resume, compaction, reboots)
- Phase 4: hygiene (history, exports, archive)

![WorkMesh DX workflow](docs/diagrams/dx-workflow.png)

Diagram source: `docs/diagrams/dx-workflow.puml`

Related diagrams (kept close to the sections they document):
- Install + MCP wiring: `docs/diagrams/install-and-mcp.png`
- Task lifecycle: `docs/diagrams/task-lifecycle.png`
- Session continuity: `docs/diagrams/continuity.png`

## Install
Prerequisites:
- None if you use prebuilt releases
- Rust toolchain (stable) if you build from source

Full guided install + quickstart: `docs/getting-started.md`

### Prebuilt binaries (recommended)
Each release publishes archives named like:
- `workmesh-vX.Y.Z-x86_64-apple-darwin.tar.gz` (macOS Intel)
- `workmesh-vX.Y.Z-aarch64-apple-darwin.tar.gz` (macOS Apple Silicon)
- `workmesh-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` (Linux x86_64, glibc)
- `workmesh-vX.Y.Z-aarch64-unknown-linux-gnu.tar.gz` (Linux arm64, glibc)
- `workmesh-vX.Y.Z-x86_64-pc-windows-msvc.zip` (Windows x86_64)

Note: in Rust target triples the `unknown` segment is the historical "vendor" field on Linux (`x86_64-unknown-linux-gnu`).

Pick a release version:
```bash
workmesh_version="v0.2.9"
```

#### macOS / Linux (tar.gz)
Option A: download with GitHub CLI (`gh`) (no raw URLs):
```bash
# Example: Linux x86_64
gh release download "$workmesh_version" -R luislobo/workmesh \
  -p "workmesh-$workmesh_version-x86_64-unknown-linux-gnu.tar.gz"

tar -xzf "workmesh-$workmesh_version-x86_64-unknown-linux-gnu.tar.gz"
sudo install -m 0755 "workmesh-$workmesh_version-x86_64-unknown-linux-gnu/workmesh" /usr/local/bin/workmesh
sudo install -m 0755 "workmesh-$workmesh_version-x86_64-unknown-linux-gnu/workmesh-mcp" /usr/local/bin/workmesh-mcp
```

Option B: download from the GitHub release page in a browser, then:
```bash
tar -xzf workmesh-vX.Y.Z-<target>.tar.gz
sudo install -m 0755 workmesh-vX.Y.Z-<target>/workmesh /usr/local/bin/workmesh
sudo install -m 0755 workmesh-vX.Y.Z-<target>/workmesh-mcp /usr/local/bin/workmesh-mcp
```

Verify:
```bash
workmesh --version
workmesh-mcp --version
```

Optional checksum verification:
- Linux: `sha256sum <archive>`
- macOS: `shasum -a 256 <archive>`

#### Windows (zip)
PowerShell (with `gh`):
```powershell
$workmesh_version = "v0.2.9"
gh release download $workmesh_version -R luislobo/workmesh `
  -p "workmesh-$workmesh_version-x86_64-pc-windows-msvc.zip"

Expand-Archive "workmesh-$workmesh_version-x86_64-pc-windows-msvc.zip" -DestinationPath . -Force
# Add the extracted folder to PATH, or move binaries somewhere already on PATH.
```

Verify:
```powershell
.\workmesh-$workmesh_version-x86_64-pc-windows-msvc\workmesh.exe --version
.\workmesh-$workmesh_version-x86_64-pc-windows-msvc\workmesh-mcp.exe --version
```

Optional checksum verification:
```powershell
CertUtil -hashfile "workmesh-$workmesh_version-x86_64-pc-windows-msvc.zip" SHA256
```

### Agent configuration (MCP)
Point your agent to the `workmesh-mcp` binary you installed (either from releases or built locally).

<details>
<summary>Diagram: Install + MCP wiring</summary>

![Install + MCP wiring](docs/diagrams/install-and-mcp.png)

</details>

### Agent-first quickstart (via MCP)
If you interact via an agent, the first verification loop is:
1. Call MCP tool `version`
2. Call MCP tool `readme`
3. Call MCP tool `doctor`
4. Quickstart a repo with MCP tool `quickstart`, then set context via `context_set`

The guided, agent-first prompts are in: `docs/getting-started.md`

From source:
```bash
git clone git@github.com:luislobo/workmesh.git
cd workmesh
cargo build
```

Optional install (CLI):
```bash
cargo install --path crates/workmesh-cli
```

MCP server binary (for Codex/Claude):
```bash
cargo build -p workmesh-mcp
# binary at target/debug/workmesh-mcp
```

## Quickstart (60 seconds)
```bash
# create docs + workmesh + seed task
workmesh --root . quickstart workmesh --feature "WorkMesh Core" --agents-snippet

# optionally set context (recommended for agents)
workmesh --root . context set --project workmesh --epic task-<init>-001 --objective "Ship v0.3"

# list tasks
workmesh --root . list --status "To Do"

# pick next task
workmesh --root . next

# start work
workmesh --root . set-status task-<init>-001 "In Progress"

# add a note
workmesh --root . note task-<init>-001 "Found missing edge case"

# mark done
workmesh --root . set-status task-<init>-001 Done
```

What gets created:
```
docs/
  projects/
    workmesh/
      README.md
      prds/
      updates/
workmesh/
  tasks/
    task-<init>-001 - seed task.md
```

Seed task id behavior:
- Quickstart now creates a namespaced seed id (`task-<init>-001`) instead of `task-001`.
- `<init>` is deduplicated across known initiatives and prefers an acronym-style key from:
  1. `--feature` (if provided)
  2. `--name` (project display name)
  3. `<project-id>` fallback

## Context (primary orchestration state)
`context` is the repo-local "what we are doing now" pointer for humans and agents.
It keeps project/objective/scope explicit, reduces session thrash, and powers scoped views/recommendations.

It lives at: `workmesh/context.json`.

Common workflow:
```bash
# set context explicitly (best for agents)
workmesh --root . context set --project workmesh --epic task-<init>-001 --objective "Ship v0.3"

# inspect current context
workmesh --root . context show

# clear context
workmesh --root . context clear
```

Migration note:
- `focus.json` is legacy and should be migrated to `context.json` via `workmesh --root . migrate audit|plan|apply`.

Integration points:
- `session save` captures `epic_id` from context epic scope (or best-effort from git branch like `task-123`).
- `session resume` prints a resume script that includes `context show` as the first step.
- `next` (CLI) and `next_task` / `next_tasks` (MCP) are context-aware:
  - epic scope: prioritizes the epic subtree
  - task scope: prioritizes listed task ids
  - active work (`In Progress` / leased) stays first within scope

## Worktrees (parallel execution layer)
Use git worktrees to run multiple agents in parallel while keeping context and sessions explicit.

CLI workflow:
```bash
# list known worktrees (git + registry)
workmesh --root . worktree list --json

# create a worktree + branch (optionally seed context)
workmesh --root . worktree create \
  --path ../repo-feature-a \
  --branch feature/a \
  --project workmesh \
  --objective "Implement feature A" \
  --json

# bind current/specified session to a worktree
workmesh --root . worktree attach --path ../repo-feature-a --json

# diagnose drift (missing paths, stale registry entries)
workmesh --root . worktree doctor --json
```

MCP tools:
- `worktree_list`
- `worktree_create`
- `worktree_attach`
- `worktree_detach`
- `worktree_doctor`

## Truth Ledger (feature-level source of truth)
Use the Truth Ledger to persist validated decisions, constraints, and contracts across sessions and worktrees.

Storage:
- Events (append-only): `workmesh/truth/events.jsonl`
- Current projection (derived from events): `workmesh/truth/current.jsonl`

Lifecycle:
- `proposed -> accepted | rejected`
- `accepted -> superseded` (only by another accepted truth)

Boundary between orchestration layers:
- `context`: current intent and scope pointer
- `truth`: validated feature decisions and invariants
- `sessions`: continuity pointer and resume hints
- `worktrees`: parallel execution isolation

CLI workflow:
```bash
# propose a truth in current project/epic scope
workmesh --root . truth propose \
  --title "Use append-only truth events" \
  --statement "Truth records are append-only and immutable." \
  --project workmesh \
  --epic task-<init>-001 \
  --feature truth-ledger \
  --tags architecture,truth \
  --json

# accept or reject after review
workmesh --root . truth accept truth-01... --note "Reviewed by team" --json
workmesh --root . truth reject truth-01... --note "Superseded by newer direction" --json

# supersede an accepted truth with another accepted truth
workmesh --root . truth supersede truth-01old --by truth-01new --reason "Refined contract" --json

# query accepted truths for current scope
workmesh --root . truth list --state accepted --project workmesh --epic task-<init>-001 --limit 20 --json

# validate events/current projection
workmesh --root . truth validate --json
```

MCP tools:
- `truth_propose`
- `truth_accept`
- `truth_reject`
- `truth_supersede`
- `truth_show`
- `truth_list`
- `truth_validate`
- `truth_migrate_audit`
- `truth_migrate_plan`
- `truth_migrate_apply`

## Views and diagnostics
These commands are meant to be "high leverage" in human+agent workflows.

Diagnostics:
```bash
workmesh --root . doctor --json
```

Board view (swimlanes):
```bash
workmesh --root . board
workmesh --root . board --by phase
workmesh --root . board --focus
```

Blocked work and top blockers (scoped to context epic by default):
```bash
workmesh --root . blockers
workmesh --root . blockers --epic-id task-<init>-001
```

## Task file format (plain text)
Each task is a Markdown file with front matter and sections:
```markdown
---
id: task-001
uid: 01...
title: Seed task
kind: task
status: To Do
priority: P2
phase: Phase1
dependencies: []
labels: []
assignee: []
---

## Notes
- Start here
```

## Initiative-slug task IDs (avoid collisions across branches)
By default, WorkMesh generates task IDs in a namespaced form:
- `task-<init>-NNN` (example: `task-logi-001`)

The `<init>` key is a 4-letter code derived from your current git branch name and then frozen in `.workmesh.toml`
so multiple agents/terminals in the same repo avoid reusing the same initiative key.

Details:
- Branch name to code: `feature/login-ui` becomes `logi` (best-effort, uses last path segment).
- Freeze mapping: `.workmesh.toml` stores `branch_initiatives.{branch} = "<initiative>"`.
- Collision avoidance: if a desired 4-letter code is already used by another branch, WorkMesh picks another
  deterministic 4-letter code (still length 4).
- Override (non-git / tests): set `WORKMESH_BRANCH=feature/login` to make the behavior deterministic.
- Manual override: pass `--id` to `add`/`add-discovered` if you want an explicit id.

If you merge branches and end up with duplicate task IDs, use:
```bash
# dry-run (default)
workmesh --root . fix ids

# apply changes (renames the duplicate tasks)
workmesh --root . fix ids --apply
```

If you want to migrate an existing backlog from legacy ids like `task-001` to the initiative-key scheme,
use the agent-assisted rekey flow:
```bash
# 1) generate a prompt that includes tasks + dependencies + graph
workmesh --root . rekey-prompt > rekey-prompt.txt

# 2) ask your agent to return JSON in the required schema, save it as mapping.json

# 3) dry-run (default: rewrites structured fields + body mentions)
workmesh --root . rekey-apply --mapping mapping.json

# 4) apply
workmesh --root . rekey-apply --mapping mapping.json --apply

# Optional: structured-only mode (no body edits)
workmesh --root . rekey-apply --mapping mapping.json --strict --apply
```

MCP tools:
- `rekey_prompt`
- `rekey_apply`

Note on collisions:
- `uid` is the true unique identity (ULID) and is required on new tasks.
- Filenames include a short UID suffix so merges stay clean even if two branches create the same `task-###`.

<details>
<summary>Diagram: Task lifecycle</summary>

![Task lifecycle](docs/diagrams/task-lifecycle.png)

</details>

### Kind (Jira-friendly)
WorkMesh supports a `kind` field to help Jira users map familiar issue types into plain-text tasks.

WorkMesh does not enforce a fixed taxonomy; Jira issue types vary by instance. Use any string you
want, but these are good defaults: `epic`, `story`, `task`, `bug`, `subtask`, `incident`, `spike`.

Filtering (CLI):
```bash
workmesh --root . list --kind bug
workmesh --root . list --kind epic --sort kind
```

Epic completion rule:
- When `kind: epic`, WorkMesh refuses to set status to `Done` until:
  - `dependencies` are `Done`
  - `relationships.blocked_by` are `Done`
  - all inferred children (tasks with `relationships.parent` pointing to the epic) are `Done`
  - plus any explicit `relationships.child` links are `Done`

## Session continuity
WorkMesh provides two complementary continuity mechanisms:

1. Repo-local checkpoints: store a snapshot inside the repo (good for "continue this repo")
2. Global agent sessions: store a cross-repo session pointer under `WORKMESH_HOME` (good for "I rebooted / switched OS / changed machines")

<details>
<summary>Diagram: Session continuity</summary>

![Session continuity](docs/diagrams/continuity.png)

</details>

### Repo-local checkpoints
Use checkpoints to resume work after compaction or a new session inside the same repo.
```bash
# write checkpoint
workmesh --root . checkpoint --project workmesh

# resume from latest checkpoint
workmesh --root . resume --project workmesh

# diff since last checkpoint
workmesh --root . checkpoint-diff --project workmesh
```

### Global agent sessions (cross-repo continuity)
Global sessions are stored outside the repo by default:
- `WORKMESH_HOME` (default: `~/.workmesh`)
- Events: `WORKMESH_HOME/sessions/events.jsonl` (append-only)
- Current pointer: `WORKMESH_HOME/sessions/current.json`
- Index: `WORKMESH_HOME/.index/sessions.jsonl` (derived, rebuildable)

Typical "I need to reboot/switch OS" workflow:
```bash
# before reboot: save a session (sets the current session pointer)
workmesh --root . session save --objective "Finish Phase 4 sessions docs"

# later: list recent sessions
workmesh --root . session list --limit 20

# resume from current session pointer (or provide a session id)
workmesh --root . session resume
workmesh --root . session resume 01K...
```

`session resume` prints a short summary plus a "resume script" (suggested next commands).
When scoped truths exist, the resume output includes accepted `truth_refs` plus a suggested
`truth list --state accepted ...` command for quick rehydration.

Auto session updates (opt-in):
- CLI flag: `--auto-session-save`
- Env: `WORKMESH_AUTO_SESSION=1`

When enabled, mutating commands update the current global session with best-effort context:
repo root, inferred project id, working set (in progress tasks / active leases), git snapshot,
and worktree binding metadata when available.
## Migration from legacy backlog/
WorkMesh prefers `workmesh/` (or `.workmesh/`). If it detects a legacy `backlog/` layout, the CLI will prompt to migrate.

If you choose **No**, it writes an optional config file so you won’t be prompted again:
```toml
# .workmesh.toml (preferred) or .workmeshrc
do_not_migrate = true
# Optional: use a different root (e.g., .workmesh)
# root_dir = ".workmesh"
```

You can migrate later at any time:
```bash
workmesh --root . migrate audit
workmesh --root . migrate plan

# dry-run by default (reports what would change)
workmesh --root . migrate apply

# apply changes
workmesh --root . migrate apply --apply

# optional backups for migrated files
workmesh --root . migrate apply --apply --backup

# include/exclude specific migration actions
workmesh --root . migrate plan --include truth_backfill
workmesh --root . migrate apply --include truth_backfill --apply
```

Migration audit/plan/apply detects deprecated structures and proposes deterministic actions:
- `layout_backlog_to_workmesh`
- `focus_to_context`
- `truth_backfill` (legacy decisions -> proposed truth records)
- `session_handoff_enrichment`
- `config_cleanup`

If you only need truth backfill from legacy notes:
```bash
workmesh --root . truth migrate audit
workmesh --root . truth migrate plan
workmesh --root . truth migrate apply        # dry-run
workmesh --root . truth migrate apply --apply
```

## Archive (date-based)
Archive moves tasks into `workmesh/archive/YYYY-MM/` when:
- `status` matches `--status` (default: `Done`)
- `task_date <= --before` (default: `30d`, meaning 30 days ago)

`task_date` is resolved as:
1. `updated_date` (if present)
2. `created_date` (if present)
3. today (fallback)

This means `workmesh --root . archive` is intentionally conservative by default. If everything was completed recently, `Archived 0 tasks` is expected.
```bash
# archive Done tasks older than 30 days (default)
workmesh --root . archive

# archive Done tasks before a specific date
workmesh --root . archive --before 2024-12-31

# archive all Done tasks dated today or earlier
workmesh --root . archive --before 0d
```

## Derived files (git-friendly)
WorkMesh generates derived artifacts for speed and continuity:
- Task index: `workmesh/.index/tasks.jsonl` (derived, rebuildable; ignored by git)
- Audit log: `workmesh/.audit.log` (append-only semantic history; ignored by default; optionally commit for full in-repo PM history)
- Global sessions index: `WORKMESH_HOME/.index/sessions.jsonl` (derived, rebuildable)

The index files are intentionally safe to delete and should not be committed. The audit log is also
safe to delete, but you may choose to commit it if you want a full, versioned history of PM actions.

## MCP usage
If the MCP server is started inside a repo, `root` can be omitted. Otherwise pass `root`.

Example (MCP call shape):
```json
{"tool": "list_tasks", "root": "/path/to/repo", "status": ["To Do"]}
```

Bulk MCP examples:
```json
{"tool": "bulk_set_status", "root": "/path/to/repo", "tasks": ["task-001","task-002"], "status": "In Progress"}
{"tool": "bulk_add_label", "root": "/path/to/repo", "tasks": ["task-001","task-002"], "label": "docs"}
{"tool": "bulk_add_dependency", "root": "/path/to/repo", "tasks": ["task-001","task-002"], "dependency": "task-010"}
{"tool": "bulk_add_note", "root": "/path/to/repo", "tasks": ["task-001","task-002"], "note": "checkpointed", "section": "notes"}
```

Choosing work (agent-friendly):
```json
{"tool": "next_tasks", "format": "json", "limit": 10}
```

## MCP client setup (examples)
WorkMesh provides a stdio MCP server binary: `workmesh-mcp`.

You configure your MCP-capable tool/editor to run it as a local stdio server, typically:
- `command`: path to `workmesh-mcp`
- `args`: usually `[]`
- start the tool in the repo so WorkMesh can infer `root` from CWD (or pass `root` explicitly in calls)

Codex example (TOML):
```toml
[mcp_servers.workmesh]
command = "/path/to/workmesh/target/debug/workmesh-mcp"
args = []
```

Then start Codex inside your repo and run:
```json
{"tool": "ready_tasks", "format": "json"}
```

VS Code example (`.vscode/mcp.json`):
  ```json
  {
    "servers": {
      "workmesh": {
        "type": "stdio",
        "command": "/path/to/workmesh/target/debug/workmesh-mcp",
        "args": []
      }
    }
  }
  ```

## Skills (Codex/Claude)
WorkMesh ships with three embedded skill profiles:
- `workmesh-mcp` (MCP-first workflows)
- `workmesh-cli` (CLI-first workflows)
- `workmesh` (router profile that selects mode)

Convenience install:
```bash
# install all profiles (router + cli + mcp) for this project
workmesh --root . install --skills --profile all --scope project

# install only MCP profile
workmesh --root . install --skills --profile mcp --scope project

# install only CLI profile
workmesh --root . install --skills --profile cli --scope project
```

Convenience uninstall:
```bash
# uninstall all profiles (router + cli + mcp) for this project
workmesh --root . uninstall --skills --profile all --scope project

# uninstall only MCP profile
workmesh --root . uninstall --skills --profile mcp --scope project

# uninstall only CLI profile
workmesh --root . uninstall --skills --profile cli --scope project
```

You can still use the explicit skill subcommands:
```bash
workmesh --root . skill install --name workmesh-mcp --scope project --agent all --force
workmesh --root . skill install --name workmesh-cli --scope project --agent all --force
workmesh --root . skill install-global --name workmesh --force
workmesh --root . skill uninstall --name workmesh-mcp --scope project --agent all
workmesh --root . skill uninstall --name workmesh-cli --scope project --agent all
workmesh --root . skill uninstall-global --name workmesh
```

Where agents discover skills:
- Project-level: `.codex/skills/<name>/SKILL.md`, `.claude/skills/<name>/SKILL.md`, `.cursor/skills/<name>/SKILL.md`
- User-level: `~/.codex/skills/<name>/SKILL.md`, `~/.claude/skills/<name>/SKILL.md`, `~/.cursor/skills/<name>/SKILL.md`
- Canonical source in this repo: `skills/workmesh/SKILL.md`, `skills/workmesh-cli/SKILL.md`, `skills/workmesh-mcp/SKILL.md`

Serving skill content via MCP:
- `skill_content` (any named skill)
- `project_management_skill` (defaults to `workmesh-mcp`; accepts `name`)
- Resolution order: `.codex/skills` -> `.claude/skills` -> `.cursor/skills` -> `skills/` -> embedded fallback

## Command reference (CLI)
Read:
- `list`, `show`, `next`, `ready`, `stats`, `export`, `issues-export`, `graph-export`

List tips:
- `workmesh --root . list --all` includes archived tasks under `workmesh/archive/` (useful for historical Done work).

Write:
- `add`, `add-discovered`, `set-status`, `set-field`, `label-add`, `label-remove`
- `dep-add`, `dep-remove`, `note`, `set-body`, `set-section`, `claim`, `release`

Touch behavior:
- All mutating commands update `updated_date` by default.
- CLI: pass `--no-touch` to suppress `updated_date` updates.
- MCP: pass `"touch": false` to suppress `updated_date` updates.

Bulk:
- `bulk-set-status`, `bulk-set-field`, `bulk-label-add`, `bulk-label-remove`
- `bulk-dep-add`, `bulk-dep-remove`, `bulk-note`
- Alias group: `bulk set-status|set-field|label-add|label-remove|dep-add|dep-remove|note`

Docs/Scaffold:
- `project-init`, `quickstart`, `validate`, `migrate audit|plan|apply`, `archive`
- `fix list`, `fix uid|deps|ids`, `fix all`

Context:
- `context set|show|clear` (primary)

Truth ledger:
- `truth propose|accept|reject|supersede|show|list|validate`
- `truth migrate audit|plan|apply`

Worktrees:
- `worktree list|create|attach|detach|doctor`

Index:
- `index-rebuild`, `index-refresh`, `index-verify`

Gantt:
- `gantt`, `gantt-file`, `gantt-svg`

Session continuity:
- Repo-local: `checkpoint`, `resume`, `working-set`, `session-journal`, `checkpoint-diff`
- Global: `session save|list|show|resume|index-rebuild|index-refresh|index-verify`

Auto-checkpointing:
- CLI flag: `--auto-checkpoint`
- Env: `WORKMESH_AUTO_CHECKPOINT=1`

Auto session updates (opt-in):
- CLI flag: `--auto-session-save`
- Env: `WORKMESH_AUTO_SESSION=1`

## Features
- CLI for list/next/show/stats/export, plus task mutation (status, fields, labels, deps, notes).
- MCP server with parity tools and rootless resolution (infer workmesh from CWD).
- Markdown task format with tolerant front-matter parsing.
- Backlog discovery supports `workmesh/tasks/`, `.workmesh/tasks/`, `tasks/`, `backlog/tasks/`, or `project/tasks/`.
- Gantt output (PlantUML text/file/svg) with dependency links.
- Graph export command (property-graph JSON for nodes + edges).
- JSONL task index with rebuild/refresh/verify for fast queries.
- Docs-first project model under `docs/projects/<project-id>/`.
- Project scaffolding via `project-init` (CLI) / `project_init` (MCP).
- Validation for required fields, missing dependencies, and missing project docs.
- Checkpoints + resume + diff for session continuity.
- Worktree runtime tooling for parallel agent execution (`worktree list/create/attach/detach/doctor`).
- Truth Ledger for durable feature decisions with strict lifecycle + migration backfill (`truth ...`).
- Bulk update operations for common task mutations (CLI + MCP).

## Repo layout
- `docs/` - project documentation, PRDs, decisions, updates.
- `workmesh/tasks/` - Markdown tasks managed by the CLI/MCP tools.
- `crates/` - Rust crates (CLI, core, MCP server).
- `skills/` - WorkMesh agent skills (source of truth).

## Troubleshooting
- **No tasks found**: ensure `workmesh/tasks/` exists or run `quickstart`.
- **PlantUML SVG fails**: install `plantuml` or set `PLANTUML_CMD`/`PLANTUML_JAR`.
- If you want WorkMesh-specific overrides, use `WORKMESH_PLANTUML_CMD` / `WORKMESH_PLANTUML_JAR`.
- **MCP tool can’t find root**: start MCP in repo or pass `root` explicitly.

## Roadmap
Near-term:
- Harden session continuity (more metadata, richer resume output).
- Expand validation and reporting (task health, stale deps).

Later:
- External sync (Jira/Trello/GitHub) — deferred.
- UI/visualization layer.

Status: Phase 1–3 complete. See PRDs under `docs/projects/workmesh/prds/`.
