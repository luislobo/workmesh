# Getting Started

WorkMesh is a docs-first, MCP-ready task system that keeps work in plain text, versioned alongside
your code. It is designed for human+agent workflows: deterministic commands, dependency-aware
planning, and restartable sessions.

If you are landing here from GitHub, start with:
1. Install WorkMesh (prebuilt binaries recommended)
2. Quickstart a repo (creates docs + a seed task)
3. Set focus (keeps agents scoped)
4. Use `next_tasks` (MCP) or `workmesh next` (CLI) to pick the next thing to do

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

## Quickstart (60 seconds)
From your repo root:
```bash
# create docs + workmesh + a seed task
workmesh --root . quickstart <project-id> --agents-snippet

# set focus explicitly (recommended for agents)
workmesh --root . focus set --project-id <project-id> --epic-id task-<init>-001 --objective "Ship v0.3"

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

## Recommended workflows (phases)
WorkMesh commands are easiest to use when you treat them as a small number of repeatable loops.

Phase A: bootstrap (once per repo)
```text
quickstart -> focus_set -> add -> ready/next
```

Phase B: daily loop (repeat)
```text
focus_show -> next_tasks -> claim -> set-status(In Progress) -> work -> note/set-section -> set-status(Done) -> release
```

Phase C: continuity (restart / reboot / compaction)
```text
session save -> stop working -> session resume -> focus_show -> next_tasks -> claim -> continue
```

Phase D: hygiene (occasional)
```text
validate -> blockers -> board -> index-refresh -> graph-export -> archive
```

## Agent setup (MCP)
Point your agent to the `workmesh-mcp` binary you installed (from releases or built locally).

If your agent supports skills directories, WorkMesh can install its embedded skill to common
locations:
```bash
workmesh --root . skill install --scope user
workmesh --root . skill install-global-auto
```

For agent-friendly docs:
- `README.json` (kept in sync with `README.md`)
- MCP tool: `readme` (returns the JSON version)

## Parallel work (branches, worktrees, multiple terminals)
If you work on multiple initiatives in parallel (multiple terminals/agents, multiple branches, or
git worktrees), the minimal loop that keeps you sane is:

1. Set focus in each workspace (repo-local): `workmesh --root . focus set ...`
2. Claim before changing tasks: `workmesh --root . claim <task-id> <owner>`
3. When context switching (reboot, OS switch, or "come back later"): `workmesh --root . session save --objective "..."`
4. When returning: `workmesh --root . session resume` and then `focus show`

This keeps "what we were doing" recorded on disk, so a fresh agent session can pick up reliably.

## Troubleshooting
Start with:
```bash
workmesh --root . doctor --json
```

Common issues:
- `root is required`: pass `--root .` in CLI; for MCP, either start the server inside the repo or provide `root`.
- Task not found: run `workmesh --root . list --all` (if you want archive included) and confirm task IDs.
- "next task" feels wrong: check `focus show --json`, dependencies/leases, and run `workmesh --root . blockers`.
- Want a quick visual snapshot: run `workmesh --root . board` or `workmesh --root . board --focus`.

Next:
- Full command reference (CLI + MCP): `docs/reference/commands.md`
