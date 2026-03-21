---
id: task-main-086
uid: 01KM7J2P7GCMERRMYA2RNG94F8
title: Document application architecture with PlantUML sources and rendered PNGs
kind: task
status: Done
priority: P1
phase: Phase8
dependencies: []
labels: [docs, architecture, plantuml]
assignee: []
relationships:
  blocked_by: []
  parent: []
  child: []
  discovered_from: []
updated_date: 2026-03-21 01:45
---
Description:
--------------------------------------------------
- Produce application architecture documentation for WorkMesh using PlantUML source diagrams committed in the repo and rendered PNG artifacts embedded in the human-facing docs.
- Cover the current crate/application architecture after the Phase 8 shared-tooling refactor so contributors can understand the domain layer, shared tooling layer, CLI adapter, MCP adapter, and stdio wrapper at a glance.
- Follow the existing docs/diagrams convention: keep PlantUML sources as the generation source of truth and publish rendered PNGs in docs so GitHub readers see diagrams without needing local rendering.
- Update the relevant documents to reference the rendered PNGs only; do not link the `.puml` source files from the docs.
- Keep the docs developer-focused and precise: diagram names, captions, and surrounding prose should explain responsibilities and boundaries, not repeat implementation noise.

Acceptance Criteria:
--------------------------------------------------
- A coherent application architecture diagram set exists under `docs/diagrams/` as PlantUML source plus rendered PNG files.
- Human-facing docs embed the rendered PNG diagrams in the appropriate architecture/documentation pages.
- The documentation explains the current crate boundaries and runtime relationships introduced by the shared-tooling refactor.
- PlantUML source files remain in-repo for regeneration, but docs link only to PNG artifacts.
- The diagrams can be regenerated locally with the available PlantUML tooling in this environment.

Definition of Done:
--------------------------------------------------
- A contributor can open the docs and understand the application architecture without reading source files first.
- The committed PlantUML sources and rendered PNGs match the current implementation.
- Description goals are achieved and all Acceptance Criteria are satisfied.

Notes:
- Reuse the existing `docs/diagrams/` convention rather than creating a parallel diagram location.
- Keep `README.md` and `README.json` in sync if top-level architecture guidance changes.
