---
id: task-main-076
uid: 01KJDFX3SJZTJHA1TMWE93RF1T
title: Update run-mode and agent setup docs for render namespace
kind: task
status: Done
priority: P1
phase: Phase7
dependencies: []
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
- Update docs for CLI and MCP stdio usage to include `render` namespace tooling.
- Add clear setup examples for CLI and GUI agents consuming render tools via stdio MCP.
- Keep `README.md` and `README.json` synchronized for all setup/contract changes.
Acceptance Criteria:
--------------------------------------------------
- Documentation includes end-to-end examples invoking render tools via MCP stdio.
- Agent setup guidance includes command-based MCP usage for CLI and GUI clients.
- README sync rule is respected in the same commit for relevant changes.
Definition of Done:
--------------------------------------------------
- Documentation accurately reflects implemented renderer tool behavior.
- Setup guidance is coherent across README, docs index, and setup reference pages.
- Description goals are met and all Acceptance Criteria are satisfied.
- Code/config committed.
- Docs updated if needed.
