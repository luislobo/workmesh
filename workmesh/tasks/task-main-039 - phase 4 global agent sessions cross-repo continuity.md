---
phase: Phase4
child: []
priority: P1
updated_date: 2026-02-06 22:27
kind: epic
uid: 01KGV3B9NAM01W7RRWQQE9BZPS
parent: []
id: task-main-039
relationships: []
labels:
- sessions
- dx
- agents
assignee: []
status: Done
dependencies: []
discovered_from: []
blocked_by: []
title: 'Phase 4: Global agent sessions (cross-repo continuity)'
---
Description:
--------------------------------------------------
- Implement a global, developer-local "agent sessions" store so you can resume work across
  repos after reboots/OS switches.
- PRD: `docs/projects/workmesh/prds/phase-4-global-agent-sessions.md`
- Deliverables:
  - Core model + storage: task-040
  - CLI commands: task-041
  - MCP parity: task-042
  - Optional sessions index: task-043
  - Opt-in auto session save: task-044
  - Docs: task-045

Acceptance Criteria:
--------------------------------------------------
- A developer can `session save` in repo A, reboot/switch, then `session resume` and reliably
  recover PWD/objective/working set in repo A.
- CLI and MCP parity exists for all session commands.
- Storage is local-first and deterministic.

Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.

Notes:
- Phase 4 global agent sessions delivered: global store under WORKMESH_HOME, session save/list/show/resume (CLI + MCP), sessions index rebuild/verify, opt-in auto updates, docs + skill updated.