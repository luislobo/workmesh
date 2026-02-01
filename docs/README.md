# WorkMesh Docs

All project documentation lives under `docs/`.

## Structure
- `docs/projects/<project-id>/` - project-level docs.
  - `prds/` - product requirement documents.
  - `decisions/` - ADRs and decision logs.
  - `updates/` - status updates (date-stamped).
  - `comments/` - synced comment history (append-only).
  - `events/` - normalized change events (append-only).
  - `conflicts/` - conflict resolution records.

Tasks live in `backlog/tasks/` and should reference the relevant PRD.
