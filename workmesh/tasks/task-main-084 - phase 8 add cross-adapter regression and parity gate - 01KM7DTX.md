---
id: task-main-084
uid: 01KM7DTXCHJHKYCPKA03Q4N3NJ
title: Phase 8: add cross-adapter regression and parity gate
kind: task
status: Done
priority: P1
phase: Phase8
dependencies: [task-main-082, task-main-083]
labels: [phase8, tests, parity, regression, tdd]
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
- Build the regression and parity gate that makes the Phase 8 refactor safe.
- Add tests at the right levels: unit tests for `workmesh-tools`, CLI regression tests for metadata/help/alias behavior, and MCP regression tests for tool-list/tool-info/representative mutation/read paths.
- Ensure the mutation response contract remains protected: default minimal acknowledgements, verbose opt-in behavior, and bulk failure reporting semantics must stay stable unless intentionally changed.
- Add coverage for render-tool parity where the shared tooling contract influences metadata or invocation behavior.
- Use TDD throughout: each extracted concern should gain or retain tests before the old wiring is removed.
- Keep the gate practical: protect high-value behavior without creating a brittle test suite that locks the project into accidental implementation details.

Acceptance Criteria:
--------------------------------------------------
- The new shared tooling crate has direct unit coverage for metadata lookup and shared response-contract helpers.
- CLI and MCP regression tests cover the extracted shared contract surfaces.
- The test suite includes parity checks for representative commands/tools rather than relying only on manual smoke testing.
- `cargo test --workspace` passes reliably after the new gate is added.
- The regression suite is documented well enough that future contributors know where to add new parity assertions.

Definition of Done:
--------------------------------------------------
- The refactor is protected by automated tests at the shared-contract, CLI, and MCP layers.
- The regression gate would catch a future reintroduction of CLI->MCP-server coupling or response-contract drift.
- Description goals are achieved and all Acceptance Criteria are satisfied.

Notes:
- Prefer targeted, high-signal assertions over snapshot sprawl.
- Include at least one test that validates the CLI and MCP surfaces read the same canonical metadata for a representative tool.