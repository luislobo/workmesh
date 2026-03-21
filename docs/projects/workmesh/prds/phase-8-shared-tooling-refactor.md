# WorkMesh PRD: Phase 8 Shared Tooling Refactor

Date: 2026-03-21
Owner: WorkMesh
Status: Draft

## Summary

Phase 8 corrects an architectural drift that accumulated while CLI/MCP parity, render tools, and metadata introspection were added quickly.

Today, `workmesh-mcp-server` is not just the MCP adapter. It has become the accidental shared tooling layer for the whole project:

- the MCP stdio binary depends on it for the server runtime
- the CLI depends on it for tool metadata
- transport-neutral helper logic lives inside it
- response-shaping policy is partially owned by it

That direction is wrong. The CLI should not depend on the MCP adapter crate. The shared contract should live below both adapters, not inside one of them.

Phase 8 introduces a dedicated shared tooling layer, expected as `crates/workmesh-tools`, and reduces `workmesh-mcp-server` to a thin MCP adapter over that shared contract.

## Problem

The current crate split is functional but not clean:

1. `workmesh-core` owns domain and storage logic.
2. `workmesh-render` owns generic renderers.
3. `workmesh-mcp-server` owns:
   - MCP server glue
   - tool metadata
   - some shared helper logic
   - response-shaping helpers
   - root/repo resolution helpers
4. `workmesh` CLI reaches into `workmesh-mcp-server` for `tool_info_payload`.

This creates three concrete problems:

1. Adapter inversion
   - The CLI depends on the MCP adapter for shared behavior.
   - Shared behavior is therefore defined "above" one adapter instead of "below" both.

2. Poor separation of responsibilities
   - `crates/workmesh-mcp-server/src/tools.rs` mixes transport logic, metadata, execution helpers, and formatting policy.
   - One file/crate has too many reasons to change.

3. Harder parity maintenance
   - Any new tool metadata or shared behavior change must thread through MCP-specific code paths.
   - Tests for shared behavior and tests for MCP transport behavior are not cleanly separated.

## Goals

1. Create a transport-neutral shared tooling layer for metadata and shared tool-contract behavior.
2. Remove the CLI dependency on `workmesh-mcp-server`.
3. Reduce `workmesh-mcp-server` to MCP-specific adaptation concerns only.
4. Preserve CLI/MCP parity for documented behavior.
5. Protect the refactor with TDD and regression tests while code is moved.

## Non-goals

1. No HTTP/service work.
2. No new remote orchestration runtime.
3. No redesign of the domain model in `workmesh-core`.
4. No movement of MCP SDK types into `workmesh-core`.
5. No broad user-facing command redesign unless a gap is discovered and explicitly documented.

## Architectural Principle

Adapters must depend on a shared contract layer, not on each other.

The target dependency shape is:

```text
workmesh-core      workmesh-render
        \            /
         \          /
          workmesh-tools
           /       \
          /         \
     workmesh   workmesh-mcp-server
                        |
                   workmesh-mcp
```

Implications:

- `workmesh-core` stays the home of domain logic, storage primitives, and repo-local/global state management.
- `workmesh-render` stays a generic rendering library.
- `workmesh-tools` becomes the home of shared tool-contract concerns.
- `workmesh-mcp-server` becomes a thin MCP adapter over shared tool definitions and helper flows.
- `workmesh` CLI becomes a thin CLI adapter over shared tool definitions and helper flows.

## Responsibility Matrix

### `workmesh-core`

Owns:

- task domain model and mutations
- context/session/truth/workstream/worktree persistence and orchestration
- storage safety primitives
- validation and business rules
- repo/global state models

Does not own:

- MCP schemas
- CLI argument parsing
- MCP tool metadata catalogs
- adapter-specific response formatting for tool introspection

Reason to change:

- domain behavior or storage behavior changes

### `workmesh-render`

Owns:

- generic rendering primitives:
  - table
  - kv
  - stats
  - list
  - progress
  - tree
  - diff
  - logs
  - alerts
  - chart-bar
  - sparkline
  - timeline

Does not own:

- tool metadata
- transport/runtime logic
- domain state mutation logic

Reason to change:

- renderer behavior changes

### `workmesh-tools` (new)

Owns:

- canonical tool metadata catalog shared by adapters
- shared tool-info payload generation
- shared mutation response-shaping helpers
- transport-neutral root/backlog/repo resolution helpers if shared by CLI and MCP
- shared helper functions that orchestrate common tool behavior without using transport types
- maybe shared tool input/output structs if they are adapter-neutral and useful

Does not own:

- MCP SDK types
- `clap` parsing
- stdio server startup
- domain persistence implementation
- renderer implementation internals

Reason to change:

- shared tool contract changes

### `workmesh-mcp-server`

Owns:

- MCP server handler
- MCP initialize/list/call plumbing
- conversion between MCP requests and shared tool calls
- conversion between shared results/errors and MCP `CallToolResult` / `CallToolError`

Does not own:

- canonical shared metadata
- generic shared resolution helpers
- CLI-facing metadata paths

Reason to change:

- MCP transport behavior changes

### `workmesh-mcp`

Owns:

- minimal stdio bootstrap wrapper
- version wiring
- command-line `--root` bootstrap argument for the MCP runtime

Does not own:

- tool behavior
- shared metadata
- domain logic

Reason to change:

- runtime startup/wrapper behavior changes

### `workmesh` CLI

Owns:

- CLI parsing and command aliases
- mapping CLI commands to shared tool contract and domain calls
- local render command fallback
- CLI-oriented presentation where JSON is not requested

Does not own:

- MCP adapter internals
- canonical shared tool metadata definitions

Reason to change:

- CLI UX changes

## What Must Move Out of `workmesh-mcp-server`

The following concerns are currently shared and should not remain owned by the MCP adapter:

1. Tool metadata and introspection catalog
   - `tool_info_payload`
   - shared tool description/spec helpers
   - canonical list of render-tool names and summaries

2. Mutation response-shaping policy
   - compact ack helpers
   - verbose-response helpers
   - bulk summary helpers (`updated_count`, `failed_count`, `failed_ids`)

3. Transport-neutral resolution helpers
   - root/backlog/repo resolution rules when the same semantics are needed by CLI and MCP
   - any helper that accepts simple Rust values and returns project data/errors without MCP types

4. Shared tool-contract tests
   - tests that assert metadata parity or response policy parity belong with the shared contract, not only in the MCP adapter tests

## What Must Stay Out of `workmesh-core`

It would be a mistake to move everything into `workmesh-core`.

The following should explicitly stay out:

1. MCP tool catalog metadata
   - this is adapter/tool-contract surface, not domain logic

2. CLI alias/help metadata
   - this is adapter UX

3. MCP SDK request/response conversion
   - transport concern

4. Adapter-specific compact/verbose response serialization shapes when they exist only to satisfy tool-surface contracts

Why:

- `workmesh-core` should not become a general "everything below the binaries" dumping ground.
- Domain/storage logic changes at a different rate than adapter/tool-contract logic.
- Keeping adapter-contract concerns above core preserves a clearer dependency graph and lowers rebuild/test scope for domain changes.

## What May Stay in `workmesh-mcp-server`

Not every helper in `tools.rs` must move.

It is acceptable for `workmesh-mcp-server` to keep:

- MCP `#[mcp_tool(...)]` declarations
- MCP handler registration/wiring
- `CallToolResult` creation helpers that are inherently MCP-specific
- MCP error mapping
- server initialization metadata assembly that directly depends on the MCP SDK

The criterion is simple:

- if a piece of code requires MCP SDK types or exists only because MCP needs it, it stays in `workmesh-mcp-server`
- if both CLI and MCP benefit from it and it does not require MCP types, it belongs below both adapters

## Current Inventory and Destination Mapping

### `tool_info_payload`

Current home:

- `crates/workmesh-mcp-server/src/tools.rs`

Target home:

- `crates/workmesh-tools/src/catalog.rs` or equivalent

Reason:

- shared introspection contract used by CLI and MCP

### render tool catalog summaries

Current home:

- `crates/workmesh-mcp-server/src/tools.rs`

Target home:

- `crates/workmesh-tools`

Reason:

- canonical tool list/summaries are not MCP-specific

### mutation response-shaping helpers

Current home:

- `crates/workmesh-mcp-server/src/tools.rs`

Target home:

- `crates/workmesh-tools/src/response.rs` or equivalent

Reason:

- shared token-saving behavior contract, not MCP transport logic

### root/backlog/repo resolution helpers

Current home:

- `crates/workmesh-mcp-server/src/tools.rs`

Target home:

- `crates/workmesh-tools/src/resolve.rs` if the logic is genuinely shared
- otherwise split shared pieces there and keep MCP-only bootstrap details in the adapter

Reason:

- current root-resolution inconsistency is a shared behavior problem, not an MCP-only problem

### MCP handler / tool-call adaptation

Current home:

- `crates/workmesh-mcp-server/src/tools.rs`

Target home:

- remains in `workmesh-mcp-server`

Reason:

- MCP-specific transport adaptation

### CLI `tool-info`

Current home:

- `crates/workmesh-cli/src/main.rs`, but it imports metadata from MCP server

Target home:

- CLI continues to own command handling
- metadata source moves to `workmesh-tools`

Reason:

- CLI command remains CLI-owned, but shared metadata must not come from MCP server

## Migration Sequence

The migration must be incremental and keep the repo buildable after each step.

### Step 1: Freeze the contract

Deliverable:

- this PRD

Why first:

- avoids moving code without a clear destination map

### Step 2: Scaffold `workmesh-tools`

Actions:

- add new crate to workspace
- add tests for metadata lookup and response-shaping helpers
- move `tool_info_payload` and close dependencies first

Why second:

- enables both adapters to converge on one shared source of truth early

### Step 3: Move shared response policy helpers

Actions:

- move compact/verbose response helpers into `workmesh-tools`
- add tests for:
  - minimal ack
  - verbose payloads
  - bulk failure summaries

Why here:

- response policy is already shared conceptually and provides quick architectural value

### Step 4: Extract shared resolution helpers

Actions:

- identify truly shared path/root resolution
- move transport-neutral pieces into `workmesh-tools`
- keep MCP wrapper-specific glue in `workmesh-mcp-server`
- add root-resolution regression tests before changing behavior

Why after metadata/policy:

- root resolution is riskier and should be handled after the shared crate exists

### Step 5: Refactor CLI to depend on `workmesh-tools`

Actions:

- remove `workmesh-mcp-server` dependency from `crates/workmesh-cli/Cargo.toml`
- update `tool-info` and related metadata paths
- run CLI regression tests/smoke checks

Why now:

- once shared metadata/helpers exist, the CLI can cut the bad dependency edge

### Step 6: Simplify `workmesh-mcp-server`

Actions:

- swap internal metadata/helper imports to `workmesh-tools`
- reduce `tools.rs` responsibilities
- keep only MCP adaptation logic

Why after CLI migration:

- proves the shared crate is not only theoretical and validates adapter independence

### Step 7: Add cross-adapter parity gate

Actions:

- add regression tests proving CLI and MCP read the same canonical metadata
- protect representative commands/tools and response policy

Why now:

- finalizes the architectural contract with automated enforcement

### Step 8: Update contributor docs and release notes

Actions:

- update README/README.json and contributor architecture guidance
- add changelog entry for the eventual release

## TDD and Regression Strategy

The refactor should not be done as a large move-then-fix exercise.

### Required test layers

1. Shared contract tests (`workmesh-tools`)
   - tool metadata lookup
   - metadata not found behavior
   - compact mutation response helpers
   - verbose mutation response helpers
   - bulk summary helpers
   - shared root-resolution semantics when extracted

2. CLI regression tests
   - `tool-info` still works
   - alias/help behavior remains stable
   - render fallback remains intact
   - CLI no longer depends on MCP server crate

3. MCP regression tests
   - initialize
   - tools/list
   - tool-info
   - representative mutation/read calls
   - response-policy parity after extraction

4. Workspace gate
   - `cargo test --workspace`

### High-value parity assertions

At minimum, assert that:

1. CLI `tool-info render_table --json` and MCP `tool_info("render_table")` come from the same canonical metadata source.
2. Compact mutation acknowledgements preserve documented fields and do not bloat unexpectedly.
3. Bulk mutation default responses preserve `failed_ids` semantics.
4. Shared root resolution behaves deterministically for:
   - repo root
   - backlog root
   - missing root
   - invalid root

### What to avoid

- brittle snapshot sprawl
- tests that assert giant opaque JSON blobs when only a few contract fields matter
- tests that indirectly lock in accidental formatting details not part of the documented contract

## Compatibility Expectations

This phase is an internal architecture refactor, so external behavior should remain stable.

Expected compatibility contract:

- no MCP tool removals
- no CLI command removals
- no intentional mutation response expansion by default
- no render-tool naming changes
- no new service/runtime mode

If any user-visible behavior must change, it must be:

1. explicitly called out in the implementing task
2. documented
3. covered by updated tests

## Risks

1. Over-extraction
   - moving too much into `workmesh-tools` could create another catch-all crate

Mitigation:

- keep its scope narrow: shared tool contract only

2. Under-extraction
   - leaving too much in `workmesh-mcp-server` would preserve the current smell

Mitigation:

- enforce the destination mapping above and the CLI dependency removal

3. Root-resolution regressions
   - this area already has inconsistency

Mitigation:

- add tests before changing semantics

4. False parity confidence
   - parity can appear green while both adapters share the same wrong implementation

Mitigation:

- combine shared-contract tests with adapter-level regression tests

## Acceptance Criteria

1. `workmesh-tools` exists and owns shared tool metadata and response-shaping logic.
2. `workmesh` CLI no longer depends on `workmesh-mcp-server`.
3. `workmesh-mcp-server` is reduced to MCP adaptation concerns.
4. Shared root-resolution and tool-contract semantics are documented and test-covered.
5. `cargo test --workspace` passes.

## Definition of Done

1. The target crate boundary is implemented as specified.
2. The CLI and MCP adapters both depend on the shared contract layer instead of on each other.
3. Automated tests protect metadata parity, response policy, and shared resolution behavior.
4. Contributor docs accurately describe the new architecture.
5. Description goals are met and all acceptance criteria are satisfied.
