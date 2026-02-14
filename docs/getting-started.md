# Getting Started

WorkMesh is easiest to adopt with a progressive workflow. Use these stages in order.

## Stage 0: Install And Verify

Install binaries (recommended) or build from source.

Quick verify:
```bash
workmesh --version
workmesh-mcp --version
```

MCP verify (from your agent chat):
1. Call `version`
2. Call `readme`
3. Call `doctor`

## Stage 1: Start (Single Repo)
Run this once in your repository root.

```bash
# bootstrap docs + workmesh + seed task
workmesh --root . quickstart <project-id> --feature "<feature-name>" --agents-snippet

# set explicit context
workmesh --root . context set \
  --project <project-id> \
  --epic task-<init>-001 \
  --objective "Ship <feature-name>"

# inspect and start
workmesh --root . context show --json
workmesh --root . next --json
workmesh --root . claim <task-id> <owner> --minutes 60
workmesh --root . set-status <task-id> "In Progress"
```

Daily loop:
```bash
workmesh --root . next --json
workmesh --root . note <task-id> "<what changed>"
workmesh --root . set-status <task-id> Done
workmesh --root . release <task-id>
```

## Stage 2: Parallelize (Worktrees)
Use one worktree per stream (feature/integration/upgrade). Avoid sibling full clones.

From canonical repo root:
```bash
# inspect existing worktrees
workmesh --root . worktree list --json

# create a stream worktree
workmesh --root . worktree create \
  --path ../.worktrees/<repo>/<stream-slug> \
  --branch <stream-branch> \
  --project <project-id> \
  --epic <epic-task-id> \
  --objective "<stream objective>" \
  --json
```

Then in the new worktree:
```bash
cd ../.worktrees/<repo>/<stream-slug>

# save/attach a session for this stream
workmesh --root . session save --objective "<stream objective>" --project <project-id>
workmesh --root . worktree attach --path . --json

# re-check scope before coding
workmesh --root . context show --json
workmesh --root . truth list --state accepted --limit 20 --json
workmesh --root . next --json
```

## Stage 3: Recover (Reboot / Lost Terminals)
This is the deterministic recovery loop.

From canonical repo root:
```bash
cd <canonical-repo>
workmesh --root . worktree list --json
```

How to use that output:
- Each `worktrees[].path` is one active stream directory.
- Open one terminal per path you want to continue.
- Ignore entries with `issues` until fixed (`worktree doctor --json`).

In each stream terminal:
```bash
cd <worktree-path>
codex resume
workmesh --root . session resume --json
workmesh --root . context show --json
workmesh --root . truth list --state accepted --limit 20 --json
workmesh --root . next --json
```

If no session exists yet for that stream:
```bash
workmesh --root . session save --objective "<stream objective>" --project <project-id>
workmesh --root . worktree attach --path . --json
```

## Stage 4: Consolidate Existing Sibling Clones
If you currently have multiple full clones of the same repo, convert to one canonical clone + worktrees.

### 4.1 Pick canonical repo
Choose one directory as canonical, for example:
```bash
cd ~/dev/nubing/repos/platform
```

### 4.2 Audit sibling clones
From the parent directory:
```bash
cd ~/dev/nubing/repos
find . -maxdepth 2 -type d -name ".git" | sed 's#/.git##' | sort
```

For each candidate clone:
```bash
git -C <clone-path> remote get-url origin
git -C <clone-path> branch --show-current
git -C <clone-path> status --porcelain
```

Rules:
- Same `origin` as canonical: candidate for conversion.
- Dirty clone (`status --porcelain` not empty): clean/commit/stash before conversion.

### 4.3 Convert one clone
Assume:
- canonical repo: `~/dev/nubing/repos/platform`
- old clone: `~/dev/nubing/repos/platform-oca`
- stream branch in old clone: `feature/oca-integration`

1. Ensure branch exists in canonical:
```bash
git -C ~/dev/nubing/repos/platform fetch --all --prune
git -C ~/dev/nubing/repos/platform show-ref --verify --quiet refs/heads/feature/oca-integration \
  || git -C ~/dev/nubing/repos/platform branch feature/oca-integration "$(git -C ~/dev/nubing/repos/platform-oca rev-parse HEAD)"
```

2. Create worktree in canonical:
```bash
cd ~/dev/nubing/repos/platform
workmesh --root . worktree create \
  --path ../.worktrees/platform/oca-integration \
  --branch feature/oca-integration \
  --project platform \
  --objective "OCA integration" \
  --json
```

3. Move session tracking to new worktree:
```bash
cd ../.worktrees/platform/oca-integration
workmesh --root . session save --objective "OCA integration" --project platform
workmesh --root . worktree attach --path . --json
workmesh --root . session resume --json
```

4. Keep old clone for safety first (default), then archive/remove later.

Repeat for each stream clone (NAVE, Tapestry Upgrade, Barcode, LINE, Reports, etc.).

## Auto Session DX (No Daily Friction)
Default behavior is designed for local interactive usage:
- Auto session updates: ON by default in interactive non-CI terminals.
- Auto session updates: OFF by default in CI/non-interactive contexts.

Overrides:
```bash
# force on for this invocation
workmesh --root . --auto-session-save set-status <task-id> "In Progress"

# force off for this invocation
workmesh --root . --no-auto-session-save set-status <task-id> "In Progress"

# persistent defaults
# ~/.workmesh/config.toml or .workmesh.toml
# auto_session_default = true
# worktrees_default = true
```

## Troubleshooting
Start here:
```bash
workmesh --root . doctor --json
workmesh --root . worktree doctor --json
```

Common issues:
- `root is required`: always run CLI with `--root .` from repo/worktree root.
- `Session not found`: create one with `session save` then `worktree attach`.
- Wrong next task: run `context show --json` and confirm scope.
- Lost feature decisions: run `truth list --state accepted --limit 20 --json`.

## What Is Next
- Detailed command syntax: [`docs/reference/commands.md`](reference/commands.md)
- Concepts and docs map: [`docs/README.md`](README.md)
