---
id: task-main-065
uid: 01KJDADV30JRZ3HA7DT5JGP7QG
title: Implement MCP-over-HTTP transport with parity mapping
kind: task
status: Done
priority: P1
phase: Phase6
dependencies: [task-main-064]
labels: [phase6, mcp, http, transport]
assignee: [luis]
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-26 10:04
---
Description:
--------------------------------------------------
- Implement MCP-over-HTTP transport adapter in service mode.
- Ensure request/response and error semantics match current MCP behavior for existing tools.
Acceptance Criteria:
--------------------------------------------------
- Service endpoint executes core WorkMesh tools via HTTP transport.
- Error mapping is deterministic and parity-tested against MCP behavior.
- Integration tests validate representative tool calls through HTTP path.
Definition of Done:
--------------------------------------------------
- Existing tool operations are reachable via HTTP transport with parity behavior.
- Transport layer is stable enough for multi-tool hosting extensions.
- Code/config committed for transport implementation.
- Docs updated if needed.
