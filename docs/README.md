# WorkMesh Docs

All project documentation lives under `docs/`.

## Structure
- `docs/projects/<project-id>/` - project-level docs.
  - `prds/` - product requirement documents.
  - `decisions/` - ADRs and decision logs.
  - `updates/` - status updates (date-stamped).
  - `comments/` - comment history (append-only).
  - `events/` - normalized change events (append-only).
- `docs/test-coverage.md` - how we measure and enforce test coverage.

Tasks live in `workmesh/tasks/` (or `.workmesh/tasks/`) and should reference the relevant PRD.
