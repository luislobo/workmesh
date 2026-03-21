---
name: workmesh
description: Router skill for WorkMesh. Selects CLI-first or MCP-first workflow based on available capabilities and user preference.
---

# WorkMesh Router Skill

Use this skill to provide a Codex-first WorkMesh experience.

## Mode selection
- If WorkMesh MCP tools are available, use `workmesh-mcp`.
- Otherwise use `workmesh-cli`.
- If the user explicitly requests one mode, honor it.
- When using the CLI, always pass `--root <repo>` unless you are already in a workflow that sets it.

## Primary user intent
When the user says `bootstrap workmesh` (or equivalent), this skill must route into bootstrap behavior, not explain commands.

## Bootstrap behavior contract
On bootstrap intent, do the following in order:
1. Detect mode (MCP or CLI).
2. Detect repository state:
   - no WorkMesh data
   - modern WorkMesh layout
   - legacy layout/deprecated structure
   - clone-based stream workflow
3. Apply state-appropriate setup:
   - initialize if missing
   - migrate if legacy
   - validate and set context if modern
   - keep work unblocked and suggest worktree consolidation if clone-based (use `worktree adopt-clone` + `workstream create --existing`)
4. Return a short summary:
   - detected state
   - actions taken
   - current context
   - next actionable tasks

## Working mode contract
After bootstrap, if user asks to work on a feature, maintain WorkMesh continuously:
- create/update PRD docs
- create/maintain tasks
- keep context current
- capture durable decisions in Truth Ledger (prefer `truth propose --current` when a workstream is active)
- if the user is restoring after reboot / lost terminals, use `workstream restore` to enumerate active streams and provide deterministic resume commands per stream
- if the user wants to change defaults (worktrees/session behavior), use `config show|set|unset` instead of asking them to edit files by hand
- when MCP is available and the user wants structured terminal-friendly output, use the `render_*` tools instead of hand-formatting large tables or trees

## Mutation response contract
- Treat MCP mutation tools as minimal-acknowledgement APIs by default.
- Do not assume a write tool returns the full refreshed object.
- Pass `verbose=true` only when the richer post-write payload is worth the token cost.
- For bulk mutations, expect compact failure identification by default (`failed_ids`) rather than full per-item result objects.
- If you need the authoritative current state after a write, prefer the matching read tool (`show_task`, `context_show`, `truth_show`, `session_show`, `workstream_show`) instead of overusing verbose writes.

## Recommended workflows

### Initialization (first time in a repo)
- Run bootstrap detection and setup.
- Ensure context is set (project/objective/epic/task scope) for the current work.
- If the repo is clone-based, propose consolidation via worktrees (adopt clone + workstream create --existing).

### Feature setup (new feature)
- Create or select a workstream for the feature.
- Seed context (objective, tasks) and create/refresh PRD docs.
- Create tasks before coding. Use SOLID methodology when decomposing tasks.
- Define `Description`, `Acceptance Criteria`, and outcome-focused `Definition of Done` for every task.

### Normal work procedure
- Always keep tasks in sync with the work (create/update tasks as scope changes).
- Use SOLID methodology when making design decisions or decomposing work.
- Make atomic commits per task.
- Mark tasks `Done` only when goals and acceptance criteria are satisfied.
- Archive completed tasks after they are `Done`.

### Structured output
- Prefer native `render_*` MCP tools for pretty tables, stats, trees, diffs, lists, progress bars, and timelines when MCP is available.
- If MCP render tools are unavailable but the local CLI is available, use `workmesh --root <repo> render ...` as the fallback before hand-formatting output.
- Fall back to plain text only when neither MCP render tools nor the CLI render command is available, or the user explicitly wants raw output.

### Renderer catalog
- `render_table`: tabular rows/columns.
- `render_kv`: compact key/value blocks.
- `render_stats`: summary metrics and counters.
- `render_list`: simple ordered/unordered item views.
- `render_progress`: progress bars and completion summaries.
- `render_tree`: hierarchical/tree structures.
- `render_diff`: before/after or unified diff-style views.
- `render_logs`: structured log/event streams.
- `render_alerts`: warnings/errors/attention states.
- `render_chart_bar`: bar chart summaries.
- `render_sparkline`: tiny trend visualizations.
- `render_timeline`: chronological milestone/event views.

### Output contract guidance
- Treat JSON as the canonical machine-readable output for WorkMesh data tools.
- Use render tools to present JSON-derived data for humans instead of overloading every data tool with custom table formatting.
- Use Markdown output only when the user wants content intended to be pasted into docs, PRs, comments, or long-form notes.
- Prefer render tools over ad hoc hand-built ASCII layouts when MCP is available.

### Renderer selection guidance
- Use `render_table` for multi-row task lists, session lists, worktree lists, and board-like summaries.
- Use `render_kv` for one task, one truth record, one session, one config object, or any single record with many fields.
- Use `render_stats` for counts by status, validation summaries, doctor summaries, and other aggregate metrics.
- Use `render_tree` for dependencies, workstream/worktree topology, or any hierarchical structure.
- Use `render_timeline` for checkpoints, session history, truth history, or ordered milestone/event views.
- Use `render_diff` for before/after comparisons such as task body changes, plan changes, or config drift.
- Use `render_progress` for completion state, rollout state, archive progress, or phase progress.
- Use `render_alerts` for blockers, warnings, integrity issues, and notable exceptions that need attention.
- Use `render_logs` for event streams, audit trails, or session/journal entries.
- Use `render_chart_bar` and `render_sparkline` only for compact visual summaries of trends or distributions, not as the primary detailed task view.
- Prefer raw JSON when another tool or agent will consume the result next.
- Prefer Markdown when generating reusable narrative content for docs, PRs, comments, or decision records.

## Rules
- Keep task metadata complete: `Description`, `Acceptance Criteria`, `Definition of Done`.
- Ensure `Definition of Done` includes outcome-based completion criteria, not only hygiene checks.
- Move task to `Done` only when description goals and acceptance criteria are fully satisfied.
- Treat all status mutation paths as equivalent for `Done` gating (including `set-field status Done` and bulk variants).
- Treat `Code/config committed` and `Docs updated if needed` as hygiene checks.
- Do not commit derived artifacts like `workmesh/.index/`.
- Do not bypass WorkMesh storage primitives for tracking files.
