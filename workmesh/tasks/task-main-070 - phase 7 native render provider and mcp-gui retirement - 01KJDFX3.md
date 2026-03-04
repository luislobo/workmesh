---
id: task-main-070
uid: 01KJDFX3YNZ7V9YDN5MKZA3A4C
title: Phase 7: native render provider and mcp-gui retirement
kind: task
status: Done
priority: P1
phase: Phase7
dependencies: [task-main-071, task-main-072, task-main-074, task-main-075, task-main-076, task-main-077]
labels: [phase7, render, platform]
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
- Deliver native Rust renderer tooling under namespace `render`.
- Migrate capabilities from external Node `mcp-gui` into first-class WorkMesh tooling.
- Complete migration docs and deprecation path so WorkMesh runs renderer workflows end-to-end without Node dependency.
Acceptance Criteria:
--------------------------------------------------
- `render` tooling exists with documented and tested renderer tools.
- CLI and MCP stdio guidance includes renderer usage and configuration examples.
- Legacy `mcp-gui` workflow is formally deprecated/retired after parity and regression gates pass.
Definition of Done:
--------------------------------------------------
- Phase 7 scope is fully implemented and validated by tests/docs.
- Migration outcome is stable, documented, and operationally clear.
- Description goals are met and all Acceptance Criteria are satisfied.
- Code/config committed.
- Docs updated if needed.
