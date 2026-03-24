# WorkMesh Shared Operating Doctrine

This file is the canonical WorkMesh operating procedure shared by router, CLI, and MCP skills.

## Core intent
WorkMesh exists to make agent work durable and recoverable.

The agent must assume chat context is lossy.
Do not rely on chat memory alone for anything important.

The durable layers are:
- tasks: the contract for the work
- context: the current scope pointer
- truths: durable decisions and constraints
- notes: implementation breadcrumbs and progress journal
- sessions/workstreams: continuity across terminals, worktrees, and restarts

## Non-negotiable rules
- Respect the repo's configured task-quality policy. The strict default requires `Description`, `Acceptance Criteria`, and `Definition of Done`.
- If configured, `Definition of Done` must be outcome-based, not hygiene-only.
- Mark `Done` only when description goals and acceptance criteria are actually satisfied.
- Treat all status mutation paths as equivalent for `Done` gating, including field writes and bulk updates.
- Do not commit derived artifacts like `workmesh/.index/`.
- Do not bypass WorkMesh storage primitives for tracking files.
- Use SOLID when decomposing work or making architectural decisions.
- Prefer TDD for behavior changes, bug fixes, and contract-sensitive code.
- Make atomic commits per task or coherent task slice.
- Archive tasks only after they are truly terminal.

## Agent operating procedure

### 1. Start or resume
On entering a repo or resuming a session, the agent should establish durable state before new work:
- determine whether this is bootstrap or resume
- read current context
- read accepted truths relevant to current scope
- if parallel streams exist or may exist, inspect workstream restore state
- determine the active task or smallest valid working set

Minimum restore objective:
- what repo/worktree am I in?
- what objective am I pursuing?
- what task am I working now?
- what durable truths constrain the work?
- what is the next concrete step?

### 2. Before doing substantive work
Before coding or making structural changes:
- claim the task if coordination matters
- set task status to `In Progress` if appropriate
- inspect the effective task-quality policy with config before assuming which fields are required
- validate that the active task satisfies the repo's configured task-quality requirements
- if the task is missing required sections or they are weak, fix the task first
- if the current work does not fit an existing task, create or split tasks before continuing

### 3. During work
When the work changes, WorkMesh must change with it.
Update WorkMesh immediately when any of these happens:
- scope changes
- a new subproblem is discovered
- a blocker appears
- a dependency changes
- a design choice becomes durable
- implementation intent materially changes
- the active objective changes

Use the right persistence layer:
- task updates: when the work contract itself changes
- notes: when recording progress, breadcrumbs, or temporary reasoning
- truths: when a decision or invariant should survive compaction, handoff, restart, worktree changes, or agent changes
- context: when current repo-local objective/scope/working set changes

### 4. Compaction-safe discipline
Assume context compaction or session loss is normal.
Before any of the following:
- long reasoning bursts
- large refactors
- topic switches
- ending the session
- after completing a meaningful implementation slice
- when conversation size is clearly growing

the agent should persist state:
- append a concise implementation note describing current status and immediate next step
- ensure task status is correct
- ensure context still matches the active work
- save or refresh session continuity when appropriate
- capture any newly durable decision as a truth

This is the key DX discipline.
The agent should behave as if chat history can disappear at any moment.

### 5. Completion procedure
Before setting a task to `Done`:
- verify that delivered work satisfies the task description goals
- verify that acceptance criteria are satisfied
- verify that definition of done is satisfied
- if needed, update task notes or sections to reflect the final state
- only then mark `Done`
- archive terminal tasks when appropriate

## Truth vs note vs task update
Use the right mechanism.

### Use a truth when
- the decision should survive restarts, handoffs, or agent changes
- it changes how future work should be done
- it is an invariant, design rule, constraint, or accepted direction
- future agents should read it before proceeding

### Use a note when
- recording progress
- recording temporary reasoning
- leaving breadcrumbs for likely resume
- documenting implementation observations that are useful but not durable policy

### Update the task when
- the task is wrong, incomplete, or too broad
- acceptance criteria changed
- definition of done changed
- task-quality policy changed and the task no longer matches the repo contract
- dependencies/blockers changed
- the work needs to be split or newly discovered follow-up work should be created

## Structured output guidance
Treat JSON as the canonical machine-readable output.
Use renderers for human presentation.
Use Markdown only for reusable narrative content such as docs, PR comments, or decision records.

Renderer selection guidance:
- MCP render tools take `data` as a JSON-encoded string. Use typed `configuration` objects when the tool exposes them.
- `render_table`: multi-row lists, boards, session lists, worktree lists
- `render_kv`: one record with many fields
- `render_stats`: counts and summaries
- `render_tree`: hierarchy or topology
- `render_timeline`: chronology and milestones
- `render_diff`: before/after comparisons
- `render_progress`: completion state
- `render_alerts`: warnings and blockers
- `render_logs`: event/journal streams
- `render_chart_bar` and `render_sparkline`: compact visual summaries only

## Mutation response contract
- Treat mutation tools as acknowledgement-first operations.
- Do not assume writes return full refreshed objects.
- Use `verbose=true` only when the richer payload is worth the token cost.
- For bulk mutations, expect summary plus compact failure identification by default.
- When current authoritative state matters after a write, use the matching read tool.

## Architecture routing
- `workmesh-core`: domain logic and storage
- `workmesh-render`: rendering
- `workmesh-tools`: shared tool metadata, response policy, and adapter-neutral helpers
- `workmesh`: CLI adapter
- `workmesh-mcp-server`: MCP adapter
- `workmesh-mcp`: stdio wrapper

Routing rule:
- shared tool semantics start in `workmesh-tools`
- CLI-only behavior goes in `workmesh`
- MCP transport-only behavior goes in `workmesh-mcp-server`
- domain/storage/state behavior goes in `workmesh-core`
- do not make the CLI depend on `workmesh-mcp-server`
