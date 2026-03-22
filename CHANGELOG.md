# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.6] - 2026-03-22

### Added
- Explicit split-root configuration with `tasks_root` and `state_root` across config, CLI, and MCP bootstrap/quickstart flows.
- Repo-root metadata persistence for external custom state roots so WorkMesh can recover the owning repository reliably.

### Changed
- New repositories now default to a split layout:
  - `tasks/` for task files
  - `.workmesh/` for repo-local state
- Replaced the legacy single-root/backlog-centric storage model with explicit task-root and state-root resolution while keeping legacy layouts readable.
- Updated migration behavior to normalize legacy single-root layouts into the split default instead of steering new setups toward `workmesh/tasks/`.

### Fixed
- Bootstrap and quickstart now honor configured custom roots even when they are not passed explicitly as command parameters.
- `migrate --to split` now handles legacy `.workmesh/tasks` repositories correctly instead of rejecting them because `.workmesh/` already exists.
- Repo-root recovery now remains correct when `state_root` is configured outside the repository tree.

### Documentation
- Updated the human and agent docs to describe `tasks/` plus `.workmesh/` as the default layout and to document the new root configuration options.

## [0.3.5] - 2026-03-22

### Changed
- Aligned WorkMesh project-local skill installation with the shared Agent Skills layout:
  - `.agents/skills/` for Codex and Cursor
  - `.claude/skills/` for Claude
- Updated embedded skill installation and removal so the full skill contents are materialized, including referenced doctrine files under each skill root.
- Kept legacy repo-local `.codex/skills` and `.cursor/skills` locations as read-only fallbacks when loading skills from older repositories.

### Documentation
- Promoted `docs/README.md` to the canonical human manual and consolidated the main usage guidance there in one sectioned document.
- Reduced `README.md` to a short landing page with install instructions and links to the canonical documentation.
- Simplified `README.json` so the agent-readable mirror points directly to the canonical documentation structure.
- Documented the WorkMesh skill model and compaction-safe agent operating procedure in the canonical manual.
- Made each canonical WorkMesh skill self-contained by moving the operating doctrine into per-skill `references/OPERATING_MODEL.md` files.
- Removed checked-in repo-local `.codex`/`.claude` skill copies so `skills/` is the single source of truth.
- Added a standard agent resume prompt and resume checklist to the canonical manual and agent-readable mirror.

## [0.3.4] - 2026-03-21

### Changed
- Added a new `workmesh-tools` crate to own shared tool metadata, response-policy helpers, and adapter-neutral tooling helpers.
- Removed the direct CLI dependency on `workmesh-mcp-server`; the CLI now reads shared tool metadata through `workmesh-tools`.
- `workmesh-mcp-server` now consumes shared metadata/root-resolution/response helpers from `workmesh-tools` instead of owning them all locally.

### Fixed
- Corrected the core coverage gate so CI measures `workmesh-core` as intended and applies per-file baseline exceptions consistently.
- Normalized workstream restore session matching across platforms so macOS and Windows recover sessions from worktree paths reliably.

### Documentation
- Reworked the top-level README into a clearer developer-first entrypoint with:
  - a shorter Codex-first path
  - clearer CLI fallback guidance
  - a simpler core concepts section
  - an explicit maintainer section for changelog and docs sync discipline
- Simplified `README.json` to mirror the new human-facing structure without duplicating low-signal detail.
- Documented the current crate architecture and clarified that MCP remains the canonical source for full input schemas while shared tool metadata/examples live in `workmesh-tools`.
- Added a dedicated `docs/architecture.md` page with rendered PlantUML diagrams for the application overview, runtime execution paths, and state topology.
- Added committed PlantUML source files plus rendered PNG artifacts under `docs/diagrams/` for architecture documentation generation.

## [0.3.3] - 2026-03-20

### Changed
- MCP mutation tools now follow an explicit token-saving response policy:
  - default responses are compact acknowledgements
  - `verbose=true` returns richer post-write state when needed
- `session_save` now defaults to a compact acknowledgement instead of returning the full saved session object.
- Bulk mutation tools now return compact failure identification by default:
  - `updated_count`
  - `failed_count`
  - `failed_ids`

### Added
- `tool_info` notes now call out the `verbose=true` mutation response contract.
- `tool_info` examples for representative mutation and bulk tools now include `verbose=true` variants.

### Documentation
- Documented the mutation response contract across README, agent README, command reference, MCP setup guidance, and embedded skills.

## [0.3.2] - 2026-03-12

### Added
- CLI render fallback via `workmesh render ...` for all native renderer tools, using inline data, files, or stdin.
- CLI parity commands for MCP-only metadata/read views:
  - `readme`
  - `tool-info`
  - `skill-content`
  - `project-management-skill`
  - `next-tasks`
- MCP-style CLI aliases for command parity, including:
  - `list_tasks`, `show_task`, `next_task`, `next_tasks`
  - `config_show`, `truth_list`, `workstream_list`, `worktree_list`
  - `render_table` and the full `render_*` tool family
- CLI `help` alias support so `workmesh help` matches MCP `help` intent.

### Changed
- Skills and docs now direct agents to prefer MCP render tools first, then use the CLI render fallback before hand-formatting output.
- Human and agent docs now document the CLI parity layer and alias behavior.

### Fixed
- Repo-root resolution for CLI metadata commands now reads from the actual repository root when `--root` points at the repo instead of a backlog directory.

## [0.3.1] - 2026-03-07

### Changed
- WorkMesh skills now document the full renderer catalog and explicit output-format guidance.
- Router skill now tells agents when to use `render_table`, `render_kv`, `render_stats`, `render_tree`, `render_timeline`, `render_diff`, `render_progress`, `render_alerts`, `render_logs`, `render_chart_bar`, and `render_sparkline`.
- CLI skill now makes the JSON-vs-rendering split explicit and directs agents to switch to MCP mode for rich rendered output.

## [0.3.0] - 2026-03-07

### Added
- Phase 1 Workstreams (parallel streams of work per repo):
  - global workstream registry (`$WORKMESH_HOME/workstreams/registry.json`) with versioned/CAS updates.
  - CLI workstream runtime: `workstream list|create|show|switch|doctor|restore`.
  - MCP parity tools: `workstream_list|create|show|switch|doctor|restore`.
  - deterministic multi-stream restore plan (`workstream restore`) with per-stream resume commands.
- Phase WS2 Workstream lifecycle + adoption:
  - CLI workstream lifecycle: `workstream pause|close|reopen|rename|set` (+ `workstream show --truth`).
  - MCP parity tools: `workstream_pause|close|reopen|rename|set` (+ `workstream_show` with `truths=true`).
  - `workstream create --existing` to bind a new workstream to an existing worktree checkout.
  - Clone-to-worktree adoption helper:
    - CLI: `worktree adopt-clone` (plan + apply)
    - MCP: `worktree_adopt_clone`
  - Truth Ledger can be linked to workstreams via `workstream_id` context field and filters.
- Phase WS3 Workflow polish:
  - `workstream create` auto-provisions a new git worktree by default when invoked from the canonical checkout (requires a real HEAD commit; respects `worktrees_default`).
  - New config key: `worktrees_dir` to control the default directory used for auto-provisioned worktrees.
  - Workstream restore view on demand:
    - CLI: `workstream show --restore`
    - MCP: `workstream_show` with `restore=true`
  - Config helpers (CLI + MCP):
    - CLI: `config show|set|unset`
    - MCP: `config_show|config_set|config_unset`
- Expanded test gates for workstreams:
  - CLI/MCP parity coverage for workstream restore.
  - Concurrency tests proving safe concurrent read-modify-write updates.
- Native render tooling over MCP stdio:
  - `render_table`, `render_kv`, `render_stats`, `render_list`, `render_progress`
  - `render_tree`, `render_diff`, `render_logs`, `render_alerts`
  - `render_chart_bar`, `render_sparkline`, `render_timeline`
- Sample project demonstrating WorkMesh capabilities with tasks, context, truth records, PRD docs, and a minimal Inventory Sync MVP implementation.

### Changed
- Workstream registry repo-root resolution is stable across git worktrees (uses git common dir when available).
- When a workstream is active in a worktree, `session save` and `worktree attach/detach` keep the stream's session/worktree pointers updated automatically.
- `context set` now persists the updated context snapshot into the active workstream record (best-effort).
- MCP server structure is split into a shared `workmesh-mcp-server` crate reused by the stdio binary.
- Docs and agent guidance now treat CLI + MCP stdio as the supported runtime path.

### Removed
- HTTP runtime and related container/service packaging from the main product surface.
- Stale service-specific docs, tasks, and architectural references that no longer match the supported workflow.

### Fixed
- Workstream read-modify-write updates now preserve concurrent field changes (no silent lost updates under contention).

## [0.2.15] - 2026-02-16

### Added
- Phase 0 Concurrency Integrity Foundation:
  - canonical storage safety primitives (`with_resource_lock`, atomic write helpers, JSONL append/recovery helpers, CAS updates with typed conflicts).
  - versioned snapshot + CAS migration for critical mutable state (`context.json`, global session pointer, worktree registry).
  - doctor storage integrity diagnostics and safe fix pathway (`--fix-storage` / `fix_storage=true`).

### Changed
- Global and repo-local tracking write paths now use centralized storage primitives (sessions, worktrees, context, truth, index, audit).
- Truth and global session event readers now tolerate trailing malformed partial lines; explicit recovery trims only trailing invalid JSONL.
- Doctor now reports lock accessibility, malformed JSONL counts, truth projection divergence, and versioned snapshot status.

### Fixed
- Removed silent lost-update risk on critical tracking snapshots by enforcing CAS conflict semantics.
- Added deterministic CLI/MCP parity coverage for doctor storage remediation (malformed recovery + no-op rerun).

### Notes
- Phase 0 gate is mandatory and complete before adding further multi-agent orchestration features.
- Phase 0 completion checks satisfied:
  - all critical tracking writes migrated to storage primitives
  - explicit conflict detection replaces silent overwrite on versioned snapshots
  - doctor detects storage integrity anomalies and supports safe remediation
  - expanded concurrency/recovery/parity tests are passing

## [0.2.14] - 2026-02-16

### Added
- Context orchestration as primary state (`context.json`) with migration audit/plan/apply support for legacy layouts and deprecated focus state.
- Truth ledger domain with CLI + MCP workflows (propose/accept/reject/supersede/list/show), validation, and legacy-decision backfill tooling.
- Worktree runtime tooling (list/create/attach/detach/doctor) and bootstrap flows for codex-first repo onboarding.
- Namespaced seed task-id generation from feature/project initials for clearer, collision-resistant bootstraps.
- Shared storage primitives for lock-protected and atomic file writes in core tracking state.

### Changed
- Command surface cleanup: removed deprecated aliases in favor of approved workflows.
- Worktree guidance defaults now use explicit precedence (project config > global config > default).
- Documentation clarified codex-first bootstrap and archive semantics across README, guides, and command reference.

### Fixed
- Archive default behavior now targets terminal statuses only, with explicit CLI/MCP parity.
- Hardened multi-agent safety by serializing read-modify-write task mutations and protecting append/projection writes against cross-process races.

## [0.2.13] - 2026-02-08

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

[Unreleased]: https://github.com/luislobo/workmesh/compare/v0.3.6...HEAD
[0.3.6]: https://github.com/luislobo/workmesh/compare/v0.3.5...v0.3.6
[0.3.5]: https://github.com/luislobo/workmesh/compare/v0.3.4...v0.3.5
[0.3.4]: https://github.com/luislobo/workmesh/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/luislobo/workmesh/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/luislobo/workmesh/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/luislobo/workmesh/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/luislobo/workmesh/compare/v0.2.15...v0.3.0
[0.2.15]: https://github.com/luislobo/workmesh/compare/v0.2.14...v0.2.15
[0.2.14]: https://github.com/luislobo/workmesh/compare/v0.2.13...v0.2.14
[0.2.13]: https://github.com/luislobo/workmesh/compare/v0.2.12...v0.2.13
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
