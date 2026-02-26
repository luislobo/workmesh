---
id: task-main-076
uid: 01KJDFX3SJZTJHA1TMWE93RF1T
title: Update run-mode and agent setup docs for render namespace
kind: task
status: Done
priority: P1
phase: Phase7
dependencies: [task-main-073]
labels: [phase7, docs, setup]
assignee: [luis]
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-26 11:40
---
Description:
--------------------------------------------------
- Update docs for CLI, MCP stdio, and MCP HTTP usage to include `render` namespace tooling.
- Add clear setup examples for CLI and GUI agents consuming render tools.
- Keep `README.md` and `README.json` synchronized for all setup/contract changes.
Acceptance Criteria:
--------------------------------------------------
- Documentation includes end-to-end examples invoking render tools via service/provider contracts.
- Agent setup guidance includes both command-based MCP and HTTP-capable client flows.
- README sync rule is respected in the same commit for relevant changes.
Definition of Done:
--------------------------------------------------
- Documentation accurately reflects implemented renderer provider behavior.
- Setup guidance is coherent across README, docs index, and setup reference pages.
- Description goals are met and all Acceptance Criteria are satisfied.
- Code/config committed.
- Docs updated if needed.
