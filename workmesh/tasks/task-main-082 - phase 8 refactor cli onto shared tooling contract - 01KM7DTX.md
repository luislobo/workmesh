---
id: task-main-082
uid: 01KM7DTXBBEPSDJ34TATSZCQ8S
title: Phase 8: refactor CLI onto shared tooling contract
kind: task
status: To Do
priority: P1
phase: Phase8
dependencies: [task-main-080, task-main-081]
labels: [phase8, cli, parity, tooling, solid, tdd]
assignee: [luis]
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-03-21 00:29
---
Description:
--------------------------------------------------
- Refactor the CLI so it consumes the shared tooling contract directly rather than importing metadata/helpers from `workmesh-mcp-server`.
- Replace the current `tool_info_payload` dependency path in `crates/workmesh-cli/src/main.rs` with the shared crate API.
- Review the CLI command/help paths that depend on shared tool metadata or response contracts, including `tool-info`, `readme`, render command help, alias handling, and any parity-oriented output that currently mirrors MCP metadata.
- Preserve current user-facing CLI behavior: command names, aliases, render fallback behavior, and output contract expectations must remain stable unless task-main-079 explicitly authorizes a change.
- Apply SOLID by making the CLI depend only on shared contract layers and lower-level libraries, never on another transport adapter.
- Use TDD: add or update CLI-focused tests before removing the old dependency, then prove the CLI still behaves the same after the migration.

Acceptance Criteria:
--------------------------------------------------
- `crates/workmesh-cli/Cargo.toml` no longer depends on `workmesh-mcp-server`.
- CLI metadata/help behavior that previously depended on MCP-server internals now works through `workmesh-tools`.
- CLI render and alias behavior remain intact after the refactor.
- Regression tests cover the migrated CLI integration points.
- Workspace tests and CLI smoke checks pass after the dependency removal.

Definition of Done:
--------------------------------------------------
- The CLI is cleanly layered on top of `workmesh-core`, `workmesh-render`, and the new shared tooling crate only.
- No CLI path requires `workmesh-mcp-server` to compile or run.
- Tests demonstrate that user-visible CLI behavior remains stable across the extraction.
- Description goals are achieved and all Acceptance Criteria are satisfied.

Notes:
- Verify with `cargo metadata` or equivalent that the CLI dependency edge to `workmesh-mcp-server` is gone.
- Keep the CLI render fallback story intact; this refactor is architectural, not user-facing feature churn.