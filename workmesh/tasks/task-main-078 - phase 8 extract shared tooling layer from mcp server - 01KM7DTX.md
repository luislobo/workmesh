---
id: task-main-078
uid: 01KM7DTX52EDG8M3DJ4GDH8RNH
title: Phase 8: extract shared tooling layer from MCP server
kind: task
status: Done
priority: P1
phase: Phase8
dependencies: [task-main-079, task-main-080, task-main-081, task-main-082, task-main-083, task-main-084, task-main-085]
labels: [phase8, architecture, tooling, parity, solid, tdd]
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
- Execute the Phase 8 architectural refactor that removes `workmesh-mcp-server` as the accidental shared tooling layer for the whole project.
- Introduce a dedicated shared tooling contract layer, expected to land as `crates/workmesh-tools`, that owns tool metadata, shared response-shaping helpers, and transport-neutral helper logic that currently lives in `crates/workmesh-mcp-server/src/tools.rs`.
- Keep `workmesh-core` focused on domain/storage logic, keep `workmesh-render` focused on generic rendering, keep `workmesh-mcp` as a thin stdio wrapper, and make both the CLI and MCP adapter depend on the same shared tooling contract.
- Apply SOLID explicitly: each crate should have one primary reason to change, transport adapters should depend on an abstract shared contract instead of each other, and the CLI must no longer depend on the MCP server crate for normal operation.
- Execute the refactor with TDD discipline: add or update tests before/with each extraction step so parity regressions are caught while the architecture changes, not after.

Acceptance Criteria:
--------------------------------------------------
- The target architecture is implemented with a new shared tooling crate and a reduced responsibility set for `workmesh-mcp-server`.
- `crates/workmesh-cli/Cargo.toml` no longer depends on `workmesh-mcp-server`.
- Tool metadata, mutation response contract helpers, and any shared adapter-neutral helpers are owned by the new shared tooling layer instead of the MCP adapter crate.
- `cargo test --workspace` passes after the refactor.
- CLI and MCP parity remain intact for documented command/tool behavior.

Definition of Done:
--------------------------------------------------
- All dependent Phase 8 implementation tasks are complete and merged.
- The crate boundary is measurably cleaner than before: adapters depend on the shared tooling layer, and the CLI no longer reaches into MCP-server internals.
- Test coverage exists for the extracted shared contract and protects the migration from parity regressions.
- Description goals are achieved and all Acceptance Criteria are satisfied.
- Code/config committed.
- Docs updated if needed.

Notes:
- This task is the umbrella for task-main-079 through task-main-085.
- Release/versioning work belongs at the end of the phase after the architecture and tests are green.