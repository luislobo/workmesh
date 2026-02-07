---
id: task-054
uid: 01KGWR2METJMF8WNCZRWPYZAV5
title: Focus: integrate with session save/resume + docs/skill + AGENTS guidance
kind: task
status: Done
priority: P1
phase: Phase4
dependencies: []
labels: [docs, agents, focus, sessions]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-02-07 13:26
---
Description:
--------------------------------------------------
- Integrate repo-local `focus` into global sessions:
  - Persist `epic_id` alongside `project_id` in session save events.
  - Prefer `focus.json` when inferring epic/project; fallback to best-effort branch parsing.
- Make session resume guidance include focus discovery (`focus show`) so agents stay scoped.
- Document `focus` in `README.md` and ship a repo-local skill file under `skills/`.

Acceptance Criteria:
--------------------------------------------------
- `workmesh session save` records `epic_id` (from focus or git branch).
- `workmesh session resume` resume-script includes `focus show`.
- MCP `session_save` records `epic_id`.
- `README.md` contains a `Focus` section with examples.
- `skills/workmesh/SKILL.md` exists and includes focus-first workflow guidance.

Definition of Done:
--------------------------------------------------
- Code/config committed.
- Docs updated if needed.

Notes:
- Implemented focus->sessions integration (epic_id), updated resume script to include focus show, documented focus in README, added repo skill file.
