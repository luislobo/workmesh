# WorkMesh PRD: Phase 1 Workstream Orchestration

Date: 2026-02-16
Owner: WorkMesh
Status: Draft

## Summary

Phase 0 delivered storage safety and recovery guarantees.  
Phase 1 builds the orchestration layer for real parallel development streams, centered on a first-class `workstream` model that ties together:

- objective and scope
- worktree and branch
- active session pointers
- task set
- truth references

Goal: make parallel work deterministic to start, track, pause, and resume across reboots and multiple agents.

## Problem

Current capabilities are strong but fragmented across context, sessions, worktrees, and truth. Users can still lose mental state across many concurrent terminals/agents.

## Goals

1. Introduce first-class `workstream` runtime model for feature streams.
2. Provide predictable resume flow after reboot (all active streams).
3. Keep CLI/MCP command behavior aligned.
4. Preserve Phase 0 storage guarantees for all new tracking paths.

## Non-goals

1. No remote database.
2. No external integration dependency.
3. No redesign of task lifecycle semantics in this phase.

## Proposed Scope

1. `workstream` registry (versioned/CAS, lock-safe).
2. CLI commands:
   - `workstream list`
   - `workstream create`
   - `workstream show`
   - `workstream switch`
   - `workstream doctor`
3. MCP parity for the same command set.
4. Session/worktree auto-linking into active workstream.
5. Resume workflow:
   - enumerate active streams
   - per-stream restore hints (path, session, context, next task)
6. Documentation + skills guidance for stream-based operation.

## Storage And Safety Invariants

1. New workstream tracking files must use `workmesh-core::storage` primitives only.
2. Mutable snapshots must be versioned and CAS-updated.
3. Event/rebuild flows must tolerate trailing malformed JSONL where applicable.
4. `doctor` must surface integrity signals and safe remediation scope.

## Acceptance Criteria

1. User can manage N parallel feature streams from one canonical repo/worktree setup.
2. User can restore active stream state after reboot using deterministic commands.
3. CLI/MCP parity tests pass for workstream commands.
4. Concurrency tests show no lost updates/corruption in workstream tracking.
5. Docs and skills clearly recommend and explain workstream-based workflow.

## Risks

1. Command-surface sprawl.
2. Ambiguity with existing context/session commands.
3. Potential duplication of registry/state unless contracts are explicit.

## Mitigations

1. Keep command set small and role-specific.
2. Define source-of-truth ownership for each state file.
3. Enforce storage invariants in code review, tests, and docs.

