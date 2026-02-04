# PRD: Phase 2 - Docs-first project model

Date: 2026-02-01
Owner: Luis Lobo
Status: Draft

## Problem
WorkMesh needs a docs-first project structure so each project has durable documentation that
travels with the codebase.

## Goals
- Define project-level docs under `docs/projects/<project-id>/`.
- Link tasks to PRDs and projects.
- Introduce project and initiative metadata in Markdown (no database).

## Non-goals
- Visualization UI.

## Requirements
- Project template (PRD, decisions, updates).
- Task front matter supports `project` and `initiative` fields.
- Validation checks missing project docs for referenced projects.

## Acceptance criteria
- A new project can be created via CLI/MCP.
- Tasks referencing a project validate against docs existence.
