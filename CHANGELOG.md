# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Focus automation: keep repo focus state updated as tasks move through statuses and leases.
- Epic Done gating: prevent marking an epic Done until its dependencies/children are Done.
- `next_tasks`: MCP tool to return deterministically ordered next-task candidates (focus-aware).
- `readme`: MCP tool to return the agent-friendly `README.json`.
- Coverage docs: added `docs/test-coverage.md` describing CI gates and local measurement commands.

### Changed
- Rekey behavior defaults to "perfect" (non-strict): rewrites structured references and free-text task id mentions in bodies; `--strict` limits to structured fields only.

### Fixed
- Build metadata rerun triggers for git changes so `--version` stays in sync with HEAD without `cargo clean`.
- Core skill tests hardened (env isolation) and expanded to raise coverage.

## [0.2.12] - 2026-02-08

### Added
- Agent-assisted task id rekey: prompt + apply tooling that rewrites ids and structured references.

### Changed
- Documented the rekey workflow and froze the initiative key strategy for stable ids.

## [0.2.11] - 2026-02-08

### Added
- 4-letter initiative keys with deterministic dedup for namespaced task ids.

## [0.2.10] - 2026-02-08

### Added
- Initiative-slug task ids and `fix-ids` repair tooling for duplicate ids after merges.

## [0.2.9] - 2026-02-08

### Added
- UID suffix support in task filenames to reduce branch-collision risk.

## [0.2.8] - 2026-02-07

### Fixed
- CI binary discovery and Windows path assertions in tests.

## [0.2.7] - 2026-02-07

### Added
- Embedded WorkMesh skill and CLI/MCP exposure.
- Global skill installation (auto-detect Codex/Claude/Cursor skill dirs).

### Changed
- PlantUML env vars renamed to `WORKMESH_PLANTUML_*`.
- Docs updated: DX workflow phases/actors and skill install paths.

## [0.2.6] - 2026-02-07

### Added
- DX workflow diagram documentation.

## [0.2.5] - 2026-02-07

### Added
- Coverage measurement + CI gating for `workmesh-core` (per-file and total thresholds).
- Repo-local focus state (CLI + MCP), plus integration with global sessions.
- Agent-oriented `README.json`.

### Fixed
- Task index stores repo-relative paths (avoid absolute-path churn across machines).

## [0.2.4] - 2026-02-06

### Added
- Global agent sessions (WORKMESH_HOME storage) with CLI + MCP parity and optional auto-updates.
- Release install documentation for macOS/Linux/Windows.

### Fixed
- Version metadata kept in sync with git state.

## [0.2.3] - 2026-02-06

### Changed
- Release CI updates: macOS runner moved to `macos-14`.

## [0.2.2] - 2026-02-06

### Added
- MCP `version` tool.
- Linux arm64 release builds.
- `WORKMESH_NO_PROMPT` to disable interactive migration prompts (for CI/tests).

### Fixed
- Windows: PlantUML command parsing/execution and stack-size issues.
- MCP parity tests improved error output.

### Changed
- `Done` implies `touch`: setting status to Done updates timestamps by default.

## [0.2.1] - 2026-02-06

### Added
- Bulk operations (CLI + MCP) with parity tests.
- MCP tool catalog (`tool_info`) and kind guidance (Jira-friendly task kinds).
- Workmesh layout support: migration and archive.
- Git build metadata in `--version`.

### Changed
- README expanded for onboarding and IDE/agent CLI setups.

## [0.2.0] - 2026-02-03

### Added
- Initial WorkMesh CLI + core model:
  - Task parsing, root detection, read/write operations, stats/export.
  - MCP server toolset.
  - Gantt support and best-practices command.
  - Docs-first project model and initial PRDs.

[Unreleased]: https://github.com/luislobo/workmesh/compare/v0.2.12...HEAD
[0.2.12]: https://github.com/luislobo/workmesh/compare/v0.2.11...v0.2.12
[0.2.11]: https://github.com/luislobo/workmesh/compare/v0.2.10...v0.2.11
[0.2.10]: https://github.com/luislobo/workmesh/compare/v0.2.9...v0.2.10
[0.2.9]: https://github.com/luislobo/workmesh/compare/v0.2.8...v0.2.9
[0.2.8]: https://github.com/luislobo/workmesh/compare/v0.2.7...v0.2.8
[0.2.7]: https://github.com/luislobo/workmesh/compare/v0.2.6...v0.2.7
[0.2.6]: https://github.com/luislobo/workmesh/compare/v0.2.5...v0.2.6
[0.2.5]: https://github.com/luislobo/workmesh/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/luislobo/workmesh/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/luislobo/workmesh/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/luislobo/workmesh/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/luislobo/workmesh/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/luislobo/workmesh/releases/tag/v0.2.0

