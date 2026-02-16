---
id: task-cif-008
title: 'Phase 0: MCP parity for storage fix pathway'
status: To Do
priority: P1
phase: Phase0
dependencies: [task-cif-007]
labels: [phase0, mcp, parity]
assignee: []
kind: task
relationships:
  blocked_by: [task-cif-007]
  parent: []
  child: []
  discovered_from: []
---

Description:
--------------------------------------------------
- Add MCP equivalent for storage remediation via doctor invocation (`fix_storage=true`).
- Ensure MCP response contract mirrors CLI semantics for storage integrity findings and fixes.
- Add parity tests to prevent drift between CLI and MCP behavior.

Acceptance Criteria:
--------------------------------------------------
- MCP exposes the planned fix pathway with explicit flag/contract.
- CLI and MCP return consistent diagnostics and remediation outcomes.
- Parity tests cover success, no-op, and malformed-data recovery scenarios.

Definition of Done:
--------------------------------------------------
- MCP parity for storage fix behavior is implemented and test-backed.
- Acceptance criteria are met with deterministic outputs.
- No contract mismatch remains between CLI and MCP for this flow.

Notes:
- This task operationalizes the recovery command addition in the plan.
