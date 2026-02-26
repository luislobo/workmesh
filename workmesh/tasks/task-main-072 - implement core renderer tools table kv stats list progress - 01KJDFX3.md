---
id: task-main-072
uid: 01KJDFX3EJCKV3GHY7T72CE6KA
title: Implement core renderer tools (table, kv, stats, list, progress)
kind: task
status: Done
priority: P1
phase: Phase7
dependencies: [task-main-071]
labels: [phase7, render, core5]
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
- Implement the first production renderer set in Rust: `render_table`, `render_kv`, `render_stats`, `render_list`, `render_progress`.
- Ensure output is deterministic and CLI-friendly, with configuration support aligned to documented tool contracts.
- Provide robust handling for malformed or incomplete inputs.
Acceptance Criteria:
--------------------------------------------------
- All 5 core tools are available as callable functions in `workmesh-render`.
- Each tool has unit coverage for nominal and error scenarios.
- Outputs are deterministic across repeated runs with the same input.
Definition of Done:
--------------------------------------------------
- Core 5 rendering tools are implemented and test-covered.
- Error behavior is consistent with provider error-mapping expectations.
- Description goals are met and all Acceptance Criteria are satisfied.
- Code/config committed.
- Docs updated if needed.
