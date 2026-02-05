# WorkMesh

WorkMesh is a docs-first, MCP-ready project and task system that keeps work in plain text,
versioned alongside your code. It is designed for human+agent workflows: deterministic
commands, dependency-aware planning, and restartable sessions.

This repository contains the Rust implementation (CLI + core + MCP server).

## Why WorkMesh
- Keep planning artifacts close to code and in git.
- Make dependencies explicit so "ready work" is queryable.
- Provide reliable handoff between sessions via checkpoints.
- Enable agent-safe coordination (leases/claims, stable ordering).

## Install
Prerequisites: Rust toolchain (stable).

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
# create docs + backlog + seed task
workmesh --root . quickstart workmesh --agents-snippet

# list tasks
workmesh --root . list --status "To Do"

# pick next task
workmesh --root . next

# start work
workmesh --root . set-status task-001 "In Progress"

# add a note
workmesh --root . note task-001 "Found missing edge case"

# mark done
workmesh --root . set-status task-001 Done
```

What gets created:
```
docs/
  projects/
    workmesh/
      README.md
      prds/
      updates/
backlog/
  tasks/
    task-001 - seed task.md
```

## Task file format (plain text)
Each task is a Markdown file with front matter and sections:
```markdown
---
id: task-001
uid: 01...
title: Seed task
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

## Session continuity
Use checkpoints to resume work after compaction or a new session.
```bash
# write checkpoint
workmesh --root . checkpoint --project workmesh

# resume from latest checkpoint
workmesh --root . resume --project workmesh

# diff since last checkpoint
workmesh --root . checkpoint-diff --project workmesh
```

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

## Codex setup (recommended)
Add WorkMesh MCP to your Codex config (rootless):
```toml
[mcp_servers.workmesh]
command = "/path/to/workmesh/target/debug/workmesh-mcp"
args = []
```

Then start Codex inside your repo and run:
```json
{"tool": "ready_tasks", "format": "json"}
```

## Agent CLI setup (popular)
Use these if you drive WorkMesh via a terminal agent rather than an IDE.

Codex CLI (OpenAI):
- Configure MCP via `codex mcp add` or by editing `~/.codex/config.toml` directly.
- Example:
  ```bash
  codex mcp add workmesh -- /path/to/workmesh-mcp
  ```

Claude Code:
- Add local MCP servers with `claude mcp add <name> -- <command> [args...]`.
- Example:
  ```bash
  claude mcp add workmesh -- /path/to/workmesh-mcp
  ```

Gemini CLI:
- Gemini CLI supports MCP servers and exposes `/mcp` to manage them.
- Use `/mcp` inside the CLI to add the WorkMesh stdio server.

GitHub Copilot CLI:
- Use `/mcp add` inside Copilot CLI and save the WorkMesh server in `~/.copilot/mcp-config.json`.

Cursor CLI:
- Cursor CLI supports MCP via `cursor-agent mcp` (same configuration as the editor).

## IDE/editor setup
VS Code (Copilot Agent mode):
- Add `.vscode/mcp.json` in your repo:
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

Cursor (editor):
- Uses the same MCP configuration as Cursor CLI. Add WorkMesh once and both use it.

IntelliJ / JetBrains IDEs:
- If your IDE doesn’t support MCP yet, use WorkMesh via the CLI or an agent CLI.
- If you have a plugin that supports MCP, configure WorkMesh as a stdio server.

Antigravity IDE:
- Use WorkMesh via the CLI unless the IDE exposes MCP configuration; if it does, point it to `workmesh-mcp`.

## Skills (Codex/Claude)
WorkMesh can serve its own skill content to agents.

- Skill file: `.codex/skills/workmesh/SKILL.md`
- MCP tool: `skill_content` or `project_management_skill`

This lets the MCP server return the exact workflow instructions for agents.

## Command reference (CLI)
Read:
- `list`, `show`, `next`, `ready`, `stats`, `export`, `issues-export`, `graph-export`

Write:
- `add`, `add-discovered`, `set-status`, `set-field`, `label-add`, `label-remove`
- `dep-add`, `dep-remove`, `note`, `set-body`, `set-section`, `claim`, `release`

Bulk:
- `bulk-set-status`, `bulk-set-field`, `bulk-label-add`, `bulk-label-remove`
- `bulk-dep-add`, `bulk-dep-remove`, `bulk-note`
- Alias group: `bulk set-status|set-field|label-add|label-remove|dep-add|dep-remove|note`

Docs/Scaffold:
- `project-init`, `quickstart`, `validate`

Index:
- `index-rebuild`, `index-refresh`, `index-verify`

Gantt:
- `gantt`, `gantt-file`, `gantt-svg`

Session continuity:
- `checkpoint`, `resume`, `working-set`, `session-journal`, `checkpoint-diff`

Auto-checkpointing:
- CLI flag: `--auto-checkpoint`
- Env: `WORKMESH_AUTO_CHECKPOINT=1`

## Features
- CLI for list/next/show/stats/export, plus task mutation (status, fields, labels, deps, notes).
- MCP server with parity tools and rootless resolution (infer backlog from CWD).
- Markdown task format with tolerant front-matter parsing.
- Backlog discovery supports `tasks/`, `backlog/tasks/`, or `project/tasks/`.
- Gantt output (PlantUML text/file/svg) with dependency links.
- Graph export command (property-graph JSON for nodes + edges).
- JSONL task index with rebuild/refresh/verify for fast queries.
- Docs-first project model under `docs/projects/<project-id>/`.
- Project scaffolding via `project-init` (CLI) / `project_init` (MCP).
- Validation for required fields, missing dependencies, and missing project docs.
- Checkpoints + resume + diff for session continuity.
- Bulk update operations for common task mutations (CLI + MCP).

## Repo layout
- `docs/` - project documentation, PRDs, decisions, updates.
- `backlog/tasks/` - Markdown tasks managed by the CLI/MCP tools.
- `crates/` - Rust crates (CLI, core, MCP server).
- `.codex/skills/` - WorkMesh agent skills.

## Troubleshooting
- **No tasks found**: ensure `backlog/tasks/` exists or run `quickstart`.
- **PlantUML SVG fails**: install `plantuml` or set `PLANTUML_CMD`/`PLANTUML_JAR`.
- **MCP tool can’t find root**: start MCP in repo or pass `root` explicitly.

## Roadmap
Near-term:
- Harden session continuity (more metadata, richer resume output).
- Expand validation and reporting (task health, stale deps).

Later:
- External sync (Jira/Trello/GitHub) — deferred.
- UI/visualization layer.

Status: Phase 1–3 complete. See PRDs under `docs/projects/workmesh/prds/`.
