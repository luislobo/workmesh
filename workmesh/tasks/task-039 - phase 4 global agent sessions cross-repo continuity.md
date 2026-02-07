---
id: task-039
uid: 01KGV3B9NAM01W7RRWQQE9BZPS
title: Phase 4: Global agent sessions (cross-repo continuity)
kind: epic
status: To Do
priority: P1
phase: Phase4
dependencies: []
labels: [sessions, dx, agents]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
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
