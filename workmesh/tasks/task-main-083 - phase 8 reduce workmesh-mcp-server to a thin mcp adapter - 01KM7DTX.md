---
id: task-main-083
uid: 01KM7DTXBY8AZKR7C8H0BBZM4P
title: Phase 8: reduce workmesh-mcp-server to a thin MCP adapter
kind: task
status: Done
priority: P1
phase: Phase8
dependencies: [task-main-080, task-main-081]
labels: [phase8, mcp, adapter, tooling, solid, tdd]
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
- Reduce `workmesh-mcp-server` to a thin MCP adapter that owns only MCP-specific concerns.
- After shared metadata and helper extraction, simplify `crates/workmesh-mcp-server` so it focuses on MCP schemas, handler wiring, request-to-tool invocation, result conversion, and server-specific error mapping.
- Remove or shrink duplicated/shared logic that should now live in `workmesh-tools`, while keeping the MCP surface fully compatible with current documented behavior.
- Ensure `crates/workmesh-mcp/src/main.rs` remains a minimal stdio bootstrap wrapper over the server crate.
- Apply SOLID by restoring a single primary reason to change for `workmesh-mcp-server`: MCP transport adaptation.
- Use TDD: add or update focused MCP adapter tests so the refactor proves that initialization, tool listing, tool info, and call behavior stay correct as internals move around.

Acceptance Criteria:
--------------------------------------------------
- `workmesh-mcp-server` no longer owns shared tool metadata or transport-neutral helper logic that is now provided by `workmesh-tools`.
- MCP adapter tests continue to pass for initialize, tools/list, tool-info, and representative tool calls.
- The `workmesh-mcp` binary remains a thin wrapper around the server crate.
- No user-facing MCP tool names or documented behavior regress as a result of the refactor.
- Workspace tests remain green after the MCP adapter cleanup.

Definition of Done:
--------------------------------------------------
- The MCP server crate has a clearly reduced responsibility set centered on MCP adaptation.
- Shared logic has been extracted without breaking documented MCP behavior.
- Tests prove the adapter still exposes the expected server/tool contract.
- Description goals are achieved and all Acceptance Criteria are satisfied.

Notes:
- This task is not an invitation to introduce HTTP/service work. Keep scope strictly to stdio MCP adapter cleanup and boundary correction.