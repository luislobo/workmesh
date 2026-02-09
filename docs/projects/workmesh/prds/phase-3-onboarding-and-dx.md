# PRD: Onboarding + Core DX Commands

## Summary
Make WorkMesh easy to adopt in a new repo and easy to operate day-to-day with a small set of
high-leverage commands that are safe for agents and deterministic for humans.

This PRD covers:
- A guided "Getting Started" doc path
- A docs index that routes to concepts/reference
- Three DX commands (CLI + MCP): `doctor`, `board`, `blockers`

## Goals
- A new developer can install + quickstart + pick next work in under 5 minutes.
- Agents can discover "what to do next" and "what is blocking us" without reading many files.
- All new commands are available in both CLI and MCP with stable JSON output.

## Non-goals
- No UI/TUI dashboard in this phase.
- No external sync (Jira/Trello/GitHub) in this phase.
- No new storage backend (keep plain text + git + derived JSONL index).

## Users and workflows
### Bootstrap (run once per repo)
```text
quickstart -> focus_set -> add -> next_tasks
```

### Daily loop (repeat)
```text
focus_show -> next_tasks -> claim -> set-status(In Progress) -> work -> note -> set-status(Done) -> release
```

### Continuity (restart/reboot/compaction)
```text
session save -> later session resume -> focus_show -> next_tasks -> claim -> continue
```

### Hygiene (occasional)
```text
doctor -> blockers -> board -> validate -> index-refresh -> graph-export -> archive
```

## Features
### 1) Guided docs path
- `docs/getting-started.md` exists and is the canonical "Start Here".
- Root `README.md` links to `docs/getting-started.md` near the top.
- `docs/README.md` is a navigable docs index (start here, concepts, reference).
- `README.md` (humans) and `README.json` (agents) remain in sync (enforced by `AGENTS.md`).

### 2) `doctor` (diagnostics)
Purpose: one-shot sanity check for layout + focus + index + skills + versions.

CLI:
- `workmesh --root . doctor [--json]`

MCP tool:
- `doctor` (format: `json` or `text`)

Output includes:
- repo root, backlog dir, layout
- focus status (project/epic/objective/working set)
- index presence + line count
- versions (self + best-effort sibling binary)
- skill presence for detected agents (codex/claude/cursor)

### 3) `board` (swimlanes)
Purpose: quick snapshot of work state without opening many files.

CLI:
- `workmesh --root . board [--by status|phase|priority] [--focus] [--all] [--json]`

MCP tool:
- `board` (args: `by`, `focus`, `all`, `format`)

Determinism:
- Lanes are stable (status has canonical ordering).
- Tasks inside a lane are stably sorted (id-based).

### 4) `blockers` (blocked work + top blockers)
Purpose: help prioritize unblocking work.

CLI:
- `workmesh --root . blockers [--epic-id <id>] [--all] [--json]`

MCP tool:
- `blockers` (args: `epic_id`, `all`, `format`)

Scoping:
- By default scopes to `focus.epic_id` subtree when focus exists.
- Otherwise repo-wide.

## Acceptance criteria
- Docs:
  - `docs/getting-started.md` exists and includes install + quickstart + workflows.
  - `README.md` links to `docs/getting-started.md` near the top.
  - `README.json` includes the same install/quickstart/command set.
- Tools:
  - `doctor`, `board`, `blockers` exist in CLI and MCP.
  - Each supports stable JSON output suitable for agents.
- Tests:
  - Unit tests cover core behavior (lane ordering, focus scoping, blockers counts).
  - MCP tests validate the new tool outputs.

## Implementation notes
- Keep the implementation local-first and deterministic.
- Avoid committing derived artifacts (`workmesh/.index/`); they are rebuildable.

