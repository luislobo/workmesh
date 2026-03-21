---
id: task-main-079
uid: 01KM7DTX98BVMBVTS9NH2D38GR
title: Phase 8: finalize target crate boundaries and migration contract
kind: task
status: To Do
priority: P1
phase: Phase8
dependencies: []
labels: [phase8, architecture, contracts, solid, tdd]
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
- Freeze the target crate architecture and migration contract before code movement begins.
- Produce a responsibility matrix for `workmesh-core`, `workmesh-render`, `workmesh-cli`, `workmesh-mcp`, `workmesh-mcp-server`, and the planned `workmesh-tools` crate.
- Identify exactly which functions/types move out of `crates/workmesh-mcp-server/src/tools.rs`, including `tool_info_payload`, mutation response-shaping helpers, shared root/repo resolution helpers, and any transport-neutral execution helpers.
- Define what must remain transport-specific to MCP (`rust-mcp-sdk` types, handler glue, request/response conversion, server initialization details) so the extraction does not leak MCP concepts into shared layers.
- Use SOLID as the design bar: if a function exists only because MCP needs it, it stays in the adapter; if both CLI and MCP need it, it belongs in the shared tooling layer or lower.
- Establish the TDD plan for the extraction: what tests protect metadata parity, response contract parity, root-resolution semantics, and adapter behavior through the migration.

Acceptance Criteria:
--------------------------------------------------
- A written design note exists in the repo, under the workmesh project docs, describing the target crate boundaries and migration sequence.
- Every currently shared concern in `workmesh-mcp-server` is assigned a destination crate with reasoning.
- The design explicitly states what will not move into `workmesh-core` and why.
- The migration order is incremental and minimizes broken intermediate states.
- The required regression tests are enumerated before implementation starts.

Definition of Done:
--------------------------------------------------
- The architecture contract is specific enough that implementation tasks can proceed without redesign churn.
- There is no unresolved ambiguity about ownership of tool metadata, response policy, root resolution, or render-tool dispatch integration.
- Test-first checkpoints are defined for downstream tasks.
- Description goals are achieved and all Acceptance Criteria are satisfied.

Notes:
- Suggested doc path: `docs/projects/workmesh/prds/phase-8-shared-tooling-refactor.md`.
- This task intentionally precedes all code movement to reduce refactor thrash.