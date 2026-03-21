---
id: task-main-081
uid: 01KM7DTXAPVMYJ8FGJB2855B09
title: Phase 8: extract transport-neutral resolution and execution helpers
kind: task
status: Done
priority: P1
phase: Phase8
dependencies: [task-main-080]
labels: [phase8, architecture, resolution, contracts, solid, tdd]
assignee: [luis]
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-03-21 01:15
---
Description:
--------------------------------------------------
- Extract the transport-neutral helper logic that is currently trapped inside `crates/workmesh-mcp-server/src/tools.rs` but conceptually shared across adapters.
- This includes shared root/backlog/repo resolution policy, adapter-neutral input normalization helpers, and any execution helpers that do not need MCP SDK types.
- Keep MCP-specific request/response conversion inside `workmesh-mcp-server`; the extracted helpers should accept/return normal Rust data types and project-level errors.
- Use this task to resolve the existing root-resolution inconsistency between repo-root and backlog-root usage where appropriate, but only if the behavior can be made explicit and regression-tested.
- Apply SOLID by separating environment resolution and shared orchestration helpers from the transport layer.
- Use TDD: write focused tests that cover repo-root resolution, backlog-root resolution, missing-root errors, and any shared helper semantics before or during extraction.

Acceptance Criteria:
--------------------------------------------------
- Shared resolution/helper logic is extracted to the shared tooling layer (or another adapter-neutral layer justified by task-main-079).
- `workmesh-tools` remains free of MCP SDK dependencies after the extraction.
- Root resolution semantics are documented and covered by regression tests.
- Any behavior change to root resolution is intentional, documented, and tested.
- Workspace tests remain green after this migration step.

Definition of Done:
--------------------------------------------------
- The extracted helpers are reusable from both CLI and MCP without adapter-specific glue.
- Root and repo resolution behavior is deterministic and protected by tests.
- The MCP adapter retains only transport-specific concerns for these flows.
- Description goals are achieved and all Acceptance Criteria are satisfied.

Notes:
- Be careful not to move generic domain logic that belongs in `workmesh-core`; this task is about shared adapter-neutral tooling behavior above core, not below it.