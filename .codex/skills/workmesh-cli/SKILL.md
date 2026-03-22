---
name: workmesh-cli
description: CLI-first WorkMesh workflow. Use when agents should run shell commands instead of MCP tool calls.
---

# WorkMesh CLI Skill

Use this skill when WorkMesh MCP is not available.

Read `OPERATING_MODEL.md` in this folder before executing feature work. It is the shared doctrine for router, CLI, and MCP operation.

## CLI mode rules
- Always pass `--root <repo>`.
- Prefer `--json` when parsing output.
- Treat JSON as the canonical data contract.
- Use `workmesh --root <repo> render ...` before hand-formatting tables, trees, stats, or timelines.

## Bootstrap intent handling
1. Discover state:
```bash
workmesh --root . doctor --json
```

2. If no WorkMesh structure:
```bash
workmesh --root . quickstart <project-id> --feature "<feature-name>" --agents-snippet
workmesh --root . context set --project <project-id> --objective "<objective>"
```

3. If legacy structure exists:
```bash
workmesh --root . migrate audit
workmesh --root . migrate plan
workmesh --root . migrate apply --apply
```

4. If modern structure exists:
```bash
workmesh --root . context show --json
workmesh --root . truth list --state accepted --limit 20 --json
workmesh --root . next --json
```

5. If clone-based stream workflow is detected:
- do not block feature work
- recommend canonical repo + worktree migration path
- helper commands:
```bash
workmesh --root . worktree adopt-clone --from <path-to-clone> --json
workmesh --root . worktree adopt-clone --from <path-to-clone> --apply --json
```

## CLI-specific helpers
### Multi-stream restore
```bash
workmesh --root . workstream restore --json
workmesh --root . workstream show <id-or-key> --restore --json
```

### High-signal loop
```bash
workmesh --root . next --json
workmesh --root . claim <task-id> <owner> --minutes 60
workmesh --root . set-status <task-id> "In Progress"
workmesh --root . note <task-id> "<note>"
workmesh --root . set-status <task-id> Done
workmesh --root . release <task-id>
```

### Config helpers
```bash
workmesh --root . config show --json
workmesh --root . config set --scope global --key auto_session_default --value true --json
workmesh --root . config set --scope project --key worktrees_dir --value "../myrepo.worktrees" --json
```
