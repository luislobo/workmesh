---
id: task-cif-010
title: 'Phase 0: documentation and skills invariants update'
status: Done
priority: P1
phase: Phase0
dependencies: [task-cif-007, task-cif-008, task-cif-009]
labels: [phase0, docs, skills]
assignee: []
kind: task
relationships:
  blocked_by: [task-cif-007, task-cif-008, task-cif-009]
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-15 23:51
---
Description:
--------------------------------------------------
- Update required docs in same phase:
  - `README.md` and `README.json`: add Concurrency Integrity Foundation guarantees
  - `docs/reference/commands.md`: conflict and recovery behavior
  - `docs/README.md`: storage/integrity policy section
  - skills docs: invariant "Do not bypass storage primitives for tracking files."
- Ensure docs reflect final implemented contract (not aspirational only).

Acceptance Criteria:
--------------------------------------------------
- All required documentation files are updated and consistent.
- README human and agent docs remain in sync.
- Commands reference clearly documents conflict and recovery semantics.
- Skills guidance explicitly encodes storage-primitive invariant.

Definition of Done:
--------------------------------------------------
- Documentation requirements from Phase 0 are completed and verified.
- Acceptance criteria are met with cross-file consistency checks.
- Docs are actionable for operators and agents without ambiguity.

Notes:
- Updated README.md and README.json with Concurrency Integrity Foundation guarantees; updated docs/reference/commands.md with doctor fix-storage, conflict semantics, and recovery behavior; updated docs/README.md with storage integrity policy; updated workmesh skills to enforce storage-primitive invariant.
- This task fulfills Section 7 of the approved plan.
