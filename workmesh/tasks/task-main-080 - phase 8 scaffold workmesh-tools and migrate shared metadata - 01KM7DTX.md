---
id: task-main-080
uid: 01KM7DTXA3JT8J6QYX1K4PSH4H
title: Phase 8: scaffold workmesh-tools and migrate shared metadata
kind: task
status: To Do
priority: P1
phase: Phase8
dependencies: [task-main-079]
labels: [phase8, architecture, tooling, metadata, solid, tdd]
assignee: [luis]
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-03-21 00:28
---
Description:
--------------------------------------------------
- Create the new shared tooling crate, expected as `crates/workmesh-tools`, and wire it into the workspace.
- Move canonical tool metadata and tool-introspection support out of `workmesh-mcp-server` into the new crate, starting with `tool_info_payload` and any closely related catalog/spec helpers.
- Move the mutation response contract helpers that shape compact acknowledgements versus verbose responses into the new crate so CLI and MCP can share one source of truth.
- Keep the public API intentionally small and stable: the new crate should expose only shared tool-contract concepts, not transport wrappers or CLI parsing details.
- Apply SOLID by making this crate responsible for the shared tool contract, not for domain state or transport concerns.
- Use TDD: add tests for tool metadata lookup, response-shaping helpers, and error behavior as the code is extracted.

Acceptance Criteria:
--------------------------------------------------
- `crates/workmesh-tools` exists and is part of the workspace build.
- Shared tool metadata/introspection logic is owned by `workmesh-tools` rather than `workmesh-mcp-server`.
- Response-shaping helpers for compact mutation acknowledgements are owned by `workmesh-tools` and covered by tests.
- The new crate does not depend on `rust-mcp-sdk` or `clap`.
- Workspace builds and tests remain green after the extraction step.

Definition of Done:
--------------------------------------------------
- The shared tooling crate compiles, is test-covered, and has a clearly bounded API.
- Existing behavior of tool metadata and response shaping is preserved or intentionally improved with tests proving the result.
- No adapter-specific concerns have leaked into the new crate.
- Description goals are achieved and all Acceptance Criteria are satisfied.

Notes:
- Prefer moving code with targeted unit tests rather than rewriting the logic from scratch.
- Keep naming consistent with the broader architecture contract defined in task-main-079.