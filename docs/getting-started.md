# Getting Started

This guide is Codex-first. It assumes your normal workflow is chat-driven, not command-driven.

## One-Time Setup
1. Install `workmesh`, `workmesh-mcp`, and `workmesh-service`.
2. Configure Codex MCP for `workmesh-mcp` if you want MCP mode.
3. Install WorkMesh skills (router + CLI + MCP profiles).

After this, your day-to-day entry flow is one prompt.

## Daily Entry Flow (Any Repo State)
You can start from any directory state:
- brand new repo with no WorkMesh
- modern WorkMesh repo
- old legacy WorkMesh/backlog layout
- long-lived branch clone workflow

Run:
1. `cd <repo-or-clone-dir>`
2. `codex` or `codex resume`
3. Prompt:

`Bootstrap WorkMesh in this repo. Use MCP if available, otherwise CLI. Detect repo state and set me up to start feature work.`

## Bootstrap Contract (What Codex Should Do)
When given that prompt, Codex should do this automatically:

1. Detect tool mode:
- if WorkMesh MCP is available, use MCP tools.
- otherwise use WorkMesh CLI commands.

2. Detect repository state:
- no WorkMesh data
- modern WorkMesh layout
- legacy layout (`backlog/`, `focus.json`, deprecated structures)
- clone/branch stream not yet migrated to worktree model

3. Apply the correct path:
- New repo: initialize WorkMesh, seed project/task context.
- Modern repo: validate health and show current scope + next work.
- Legacy repo: migrate to modern layout, then continue.
- Clone-based stream: continue work now and recommend worktree consolidation path.

4. Confirm bootstrap result in chat:
- detected state
- actions taken
- current context
- next recommended task(s)

## Start Feature Work (Single Prompt)
After bootstrap, use one explicit feature prompt:

`Use WorkMesh for this feature end to end. Create/update PRD, create and maintain tasks with acceptance criteria and definition of done, keep context current, and track stable decisions in Truth Ledger.`

Quality expectations:
- Every task should have meaningful `Description`, `Acceptance Criteria`, and `Definition of Done`.
- `Definition of Done` should include outcome-based completion criteria, not only hygiene checks.
- WorkMesh gates `Done` transitions when quality requirements are not met.

From here, stay in normal chat. You should not need to switch into command memorization mode.

## Optional HTTP Service Mode
If you want a long-lived local/LAN service runtime:

1. Verify binary:
   - `workmesh --root . service verify`
2. Start service in foreground:
   - `workmesh --root . service start --config ./service.toml`
3. Use HTTP endpoints:
   - `GET /v1/healthz`, `GET /v1/readyz`, `GET /v1/status`, `GET /v1/metrics`
   - `GET /v1/providers`
   - `POST /v1/mcp/invoke`
   - `POST /v1/admin/reload`

For LAN exposure, use bearer auth token and keep localhost as the default bind unless explicitly required.

## Reboot / Resume Flow
When you come back later:
1. `cd <repo-or-worktree>`
2. `codex resume`
3. Prompt:

`Rehydrate this session with WorkMesh: restore context, accepted truths, and next actionable tasks.`

## Restore Multiple Workstreams (After Reboot)
If you run multiple parallel workstreams (often one git worktree per stream), you can restore them deterministically from any checkout of the repo:

1. `cd <repo-or-any-worktree>`
2. Run:

```bash
workmesh --root . workstream restore --json
```

This returns a per-workstream restore plan. Each entry includes:
- `worktree_path`: where to open a terminal for that stream
- `session_id`: the last-known session for that stream (best effort)
- `context`: objective/scope (from `context.json`, legacy `focus.json`, or the workstream snapshot)
- `next_task`: the next recommended task under that stream's context
- `resume_script`: the exact commands to run in that worktree to resume

Typical usage:
1. Open a terminal in `worktree_path`.
2. Run the `resume_script` commands (they include `session resume`, `context show`, and `next`).
3. Continue feature work in Codex from that worktree.

Tip: to get resume commands for a single workstream:
```bash
workmesh --root . workstream show <id-or-key> --restore --json
```

## Clone-to-Worktree Transition (When You Are Ready)
If you currently keep multiple full clones for parallel streams, migrate progressively:

1. Pick one canonical repo clone.
2. Keep current stream moving.
3. For each old clone, adopt it into a git worktree (backup + worktree add):

```bash
workmesh --root . worktree adopt-clone --from <path-to-clone> --json
workmesh --root . worktree adopt-clone --from <path-to-clone> --apply --json
```

4. Bind a workstream to the created worktree with `workstream create --existing` (the adoption plan includes the exact command).
5. Attach/save session metadata to each worktree (`session save`, `worktree attach`).
6. Retire old clone directories after validation.

This migration is operationally helpful, but it should not block you from feature work.

## Manual Commands (Appendix)
Only use this if you explicitly want direct CLI execution.

- Bootstrap: `workmesh --root . bootstrap --project-id <project-id> --feature "<feature-name>" --json`
- Context: `workmesh --root . context set --project <project-id> --epic <epic-id> --objective "<objective>"`
- Next task: `workmesh --root . next --json`
- Session resume: `workmesh --root . session resume --json`
- Workstream restore (all active streams): `workmesh --root . workstream restore --json`
- Worktrees: `workmesh --root . worktree list --json`
- Adopt clone into worktree: `workmesh --root . worktree adopt-clone --from <path-to-clone> --apply --json`
- Archive (default terminal statuses): `workmesh --root . archive --before 30d --json`
- Archive (explicit override): `workmesh --root . archive --status "To Do" --before 2026-12-31 --json`
- Migrate legacy: `workmesh --root . migrate audit|plan|apply --apply`

## Related Docs
- Command reference: [`docs/reference/commands.md`](reference/commands.md)
- Documentation index: [`docs/README.md`](README.md)
