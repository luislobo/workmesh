---
id: task-ws3-002
title: "Phase WS3: config command surface (CLI + MCP parity)"
kind: task
status: Done
priority: P1
phase: PhaseWS3
dependencies: [task-ws3-001]
labels: [ws3, config, parity]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-16 18:28
---
Description:
--------------------------------------------------
- Add first-class config helpers so chat-driven workflows can update WorkMesh defaults without
  editing files by hand.
- Expose the same behavior in CLI and MCP.

Acceptance Criteria:
--------------------------------------------------
- CLI:
  - `config show`
  - `config set --scope project|global --key <key> --value <value>`
  - `config unset --scope project|global --key <key>`
- MCP:
  - `config_show`
  - `config_set`
  - `config_unset`
- Supports keys:
  - `worktrees_default`
  - `worktrees_dir`
  - `auto_session_default`
  - `root_dir`
  - `do_not_migrate`
- `config show` reports:
  - project/global config paths (when present)
  - effective values
  - sources (project/global/default)

Definition of Done:
--------------------------------------------------
- Implemented for both CLI and MCP.
- Docs updated for new command/tool surface.

