---
parent: []
discovered_from: []
kind: task
uid: 01KGWR2METJMF8WNCZRWPYZAV5
assignee: []
labels:
- docs
- agents
- focus
- sessions
updated_date: 2026-02-07 13:26
id: task-main-054
dependencies: []
phase: Phase4
relationships: []
priority: P1
blocked_by: []
title: 'Focus: integrate with session save/resume + docs/skill + AGENTS guidance'
status: Done
child: []
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