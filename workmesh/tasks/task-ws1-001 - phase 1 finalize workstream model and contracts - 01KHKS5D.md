---
id: task-ws1-001
uid: 01KHKS5D7C3ZZWVS2ZMTT57J0Q
title: Phase 1: finalize workstream model and contracts
kind: task
status: In Progress
priority: P1
phase: PhaseWS1
dependencies: []
labels: [phase1, workstreams, orchestration, contracts]
assignee: [luis]
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-16 11:50
lease_owner: luis
lease_acquired_at: 2026-02-16 11:50
lease_expires_at: 2026-02-16 13:20
---
Description:
--------------------------------------------------
- Finalize Phase 1 workstream domain contract before implementation.
- Define source-of-truth ownership between context, session, worktree, truth, and the new workstream registry.
- Freeze CLI and MCP command behavior contracts for create/list/show/switch/doctor style operations.

Acceptance Criteria:
--------------------------------------------------
- Workstream data model is documented with field-level semantics and ownership.
- Command contract is explicit for CLI and MCP with consistent behavior definitions.
- Compatibility and migration expectations are documented for existing repos.

Definition of Done:
--------------------------------------------------
- Contract is complete enough to implement without redesign churn.
- Acceptance criteria are fully met and reviewed.
- Downstream implementation tasks can proceed with no open blocking ambiguities.

Notes:
- Phase 1 PRD drafted at docs/projects/workmesh/prds/phase-1-workstream-orchestration.md. Backlog seeded with task-ws1-001..008 and dependencies.
