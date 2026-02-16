---
id: task-ws2-004
title: "Phase WS2: docs and PRD status sync"
kind: task
status: Done
priority: P2
phase: PhaseWS2
dependencies: [task-ws2-001, task-ws2-002, task-ws2-003]
labels: [ws2, docs, prd]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-16 16:50
---
Description:
--------------------------------------------------
- Keep docs consistent with the new workstream/worktree/truth command surface.
- Ensure README.md and README.json remain synchronized on workflows and commands.
- Update PRD statuses that are now implemented to reduce confusion about "what phase are we on".

Acceptance Criteria:
--------------------------------------------------
- `docs/reference/commands.md` includes the new commands/tools and parameters.
- README.md and README.json include correct links and mention the new lifecycle/adoption workflow.
- Implemented PRDs under `docs/projects/workmesh/prds/` are updated from Draft to Implemented (or superseded) with clear notes.

Definition of Done:
--------------------------------------------------
- Docs are navigable in GitHub with correct relative links.
- No "DX" jargon-only phrasing (use "developer experience" language).
- Users can follow docs to manage workstreams and adopt clones into worktrees.
