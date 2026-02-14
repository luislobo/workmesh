# Getting Started

WorkMesh is a docs-first, MCP-ready task system that keeps work in plain text, versioned alongside
your code. It is designed for human+agent workflows: deterministic commands, dependency-aware
planning, and restartable sessions.

If you are landing here from GitHub, start with:
1. Install WorkMesh (prebuilt binaries recommended)
2. Configure your agent to run WorkMesh as MCP (`workmesh-mcp`)
3. Quickstart a repo (creates docs + a seed task)
4. Set context (keeps agents scoped)
5. Use `next_tasks` to get candidates and let the agent decide

## Install
You have two install options:
- Use prebuilt binaries from GitHub Releases (recommended)
- Build from source (Rust stable)

### Prebuilt binaries (recommended)
Releases include `workmesh` (CLI) and `workmesh-mcp` (MCP server).

Archives are named like:
- `workmesh-vX.Y.Z-x86_64-apple-darwin.tar.gz` (macOS Intel)
- `workmesh-vX.Y.Z-aarch64-apple-darwin.tar.gz` (macOS Apple Silicon)
- `workmesh-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` (Linux x86_64, glibc)
- `workmesh-vX.Y.Z-aarch64-unknown-linux-gnu.tar.gz` (Linux arm64, glibc)
- `workmesh-vX.Y.Z-x86_64-pc-windows-msvc.zip` (Windows x86_64)

Note: in Rust target triples the `unknown` segment is the historical "vendor" field on Linux
(`x86_64-unknown-linux-gnu`).

Pick a release version:
```bash
workmesh_version="v0.2.13"
```

macOS / Linux (tar.gz), using GitHub CLI:
```bash
# Example: Linux x86_64
gh release download "$workmesh_version" -R luislobo/workmesh \
  -p "workmesh-$workmesh_version-x86_64-unknown-linux-gnu.tar.gz"

tar -xzf "workmesh-$workmesh_version-x86_64-unknown-linux-gnu.tar.gz"
sudo install -m 0755 "workmesh-$workmesh_version-x86_64-unknown-linux-gnu/workmesh" /usr/local/bin/workmesh
sudo install -m 0755 "workmesh-$workmesh_version-x86_64-unknown-linux-gnu/workmesh-mcp" /usr/local/bin/workmesh-mcp
```

macOS / Linux (tar.gz), if you downloaded from a browser:
```bash
tar -xzf workmesh-vX.Y.Z-<target>.tar.gz
sudo install -m 0755 workmesh-vX.Y.Z-<target>/workmesh /usr/local/bin/workmesh
sudo install -m 0755 workmesh-vX.Y.Z-<target>/workmesh-mcp /usr/local/bin/workmesh-mcp
```

Windows (zip), PowerShell:
```powershell
$workmesh_version = "v0.2.13"
gh release download $workmesh_version -R luislobo/workmesh `
  -p "workmesh-$workmesh_version-x86_64-pc-windows-msvc.zip"

Expand-Archive "workmesh-$workmesh_version-x86_64-pc-windows-msvc.zip" -DestinationPath . -Force
# Add the extracted folder to PATH, or move binaries somewhere already on PATH.
```

Verify:
```bash
workmesh --version
workmesh-mcp --version
```

Optional checksum verification:
- Linux: `sha256sum <archive>`
- macOS: `shasum -a 256 <archive>`
- Windows: `CertUtil -hashfile <archive> SHA256`

### Build from source
```bash
git clone git@github.com:luislobo/workmesh.git
cd workmesh
cargo build
```

Optional install (CLI):
```bash
cargo install --path crates/workmesh-cli
```

MCP server binary (for agents):
```bash
cargo build -p workmesh-mcp
# binary at target/debug/workmesh-mcp
```

## Agent-first setup (MCP)
If you interact 100% via agents, the right order is:
1. Install `workmesh-mcp`
2. Wire your agent to run it via MCP (stdio)
3. Verify from chat by calling `version` and `readme`

Minimal verification (from agent chat):
- "Call MCP tool `version`"
- "Call MCP tool `readme`"
- "Call MCP tool `doctor` with `format=json`"

Optional: install the embedded skill so your agent can discover the workflow conventions:
```bash
workmesh --root . install --skills --profile mcp --scope project
```

For agent-friendly docs:
- `README.json` (kept in sync with `README.md`)
- MCP tool: `readme` (returns the JSON version)

Skill profiles:
- `workmesh-mcp`: MCP-first workflow guidance
- `workmesh-cli`: CLI-first workflow guidance
- `workmesh`: router skill (selects the right mode)

If you prefer CLI-only agent operation (no MCP tool calls), install:
```bash
workmesh --root . install --skills --profile cli --scope project
```

## Recommended workflows (phases, agent-first)
WorkMesh commands are easiest to use when you treat them as a small number of repeatable loops.

Each phase includes:
- Example prompt(s) you can paste into chat
- The MCP tools that should be invoked

### Phase A: bootstrap a repo (run once)
Example prompt:
- "Initialize WorkMesh in this repo for project `<project-id>`. Use quickstart, set context, then show me the next 10 candidate tasks."

MCP tools:
- `quickstart`
- `context_set`
- `next_tasks`
- `list_tasks` (optional verification)

### Phase B: daily loop (repeat)
Example prompt:
- "Show current context, then recommend next work items and claim the best one as `me` for 60 minutes. Mark it In Progress and add a short note about what you plan to do."

MCP tools:
- `context_show`
- `next_tasks`
- `claim_task`
- `set_status`
- `add_note`

### Phase C: continuity (restart / reboot / compaction)
Example prompt:
- "Save a global session with objective `<objective>`, then show me the resume script."
- "Resume the latest session and then show context + next tasks."

MCP tools:
- `session_save`
- `session_resume`
- `context_show`
- `next_tasks`

### Phase D: hygiene (occasional)
Example prompt:
- "Run doctor, then show blockers for the focused epic, then show a status board scoped to focus."

MCP tools:
- `doctor`
- `blockers`
- `board`
- `validate` (optional)
- `index_refresh` (optional)

## Quickstart (CLI appendix)
If you ever need to run the same workflow without an agent:
```bash
# create docs + workmesh + a seed task
workmesh --root . quickstart <project-id> --feature "<feature-name>" --agents-snippet

# set context explicitly (recommended for agents)
workmesh --root . context set --project <project-id> --epic task-<init>-001 --objective "Ship v0.3"

# list tasks
workmesh --root . list --status "To Do"

# pick next task (focus-aware)
workmesh --root . next

# start work
workmesh --root . set-status task-<init>-001 "In Progress"

# add a note
workmesh --root . note task-<init>-001 "Found missing edge case"

# mark done
workmesh --root . set-status task-<init>-001 Done
```

What gets created:
```text
docs/
  projects/
    <project-id>/
      README.md
      prds/
      updates/
workmesh/
  tasks/
    task-<init>-001 - seed task.md
```

## Parallel work (branches, worktrees, multiple terminals)
If you work on multiple initiatives in parallel (multiple terminals/agents, multiple branches, or
git worktrees), the minimal loop that keeps you sane is:

1. Set context in each workspace (repo-local): `workmesh --root . context set ...`
2. Claim before changing tasks: `workmesh --root . claim <task-id> <owner>`
3. When context switching (reboot, OS switch, or "come back later"): `workmesh --root . session save --objective "..."`
4. When returning: `workmesh --root . session resume` and then `context show`

This keeps "what we were doing" recorded on disk, so a fresh agent session can pick up reliably.

## Troubleshooting
Start with:
```bash
workmesh --root . doctor --json
```

Common issues:
- `root is required`: pass `--root .` in CLI; for MCP, either start the server inside the repo or provide `root`.
- Task not found: run `workmesh --root . list --all` (if you want archive included) and confirm task IDs.
- "next task" feels wrong: check `context show --json`, dependencies/leases, and run `workmesh --root . blockers`.
- Want a quick visual snapshot: run `workmesh --root . board` or `workmesh --root . board --focus`.

Next:
- Full command reference (CLI + MCP): `docs/reference/commands.md`
