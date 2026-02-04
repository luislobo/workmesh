# PRD: Phase 3 - Session continuity + checkpoints

Date: 2026-02-04
Owner: Luis Lobo
Status: Draft

## Problem
Agents lose context between sessions. We need lightweight, deterministic artifacts that allow
fast resume and reliable handoff without requiring large prompts.

## Goals
- Provide a checkpoint command that captures current state in a structured format.
- Enable a resume command to continue from the latest checkpoint.
- Track a working set and a lightweight session journal.
- Include recent file changes and directories worked on.

## Non-goals
- Full external sync or conflict resolution.
- Real-time collaboration or UI dashboards.

## Requirements
- Checkpoint outputs both JSON and Markdown under `docs/projects/<project>/updates/`.
- Resume command reads latest checkpoint and prints a concise summary + next actions.
- Working set file is small and human-readable.
- Session journal is append-only and lightweight.
- Checkpoint includes: current task, ready list, leases, git status summary, changed files,
  top-level directories touched, and recent audit events.

## Acceptance criteria
- `checkpoint` produces deterministic output and does not fail when optional data is missing.
- `resume` works after a restart with no additional context.
