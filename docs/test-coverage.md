# Test Coverage

WorkMesh uses `cargo-llvm-cov` for coverage measurement.

## What We Enforce In CI

CI enforces a minimum coverage floor for `workmesh-core` (the library where most logic lives):

- Minimum: `80%`
- Metrics: `Regions` and `Lines`
- Scope: core files only (excludes `crates/workmesh-cli/` and `crates/workmesh-mcp/`)
- Script: `scripts/ci_core_coverage_gate.sh`

This is intentionally strict on core and intentionally *not* strict on the CLI/MCP binaries, because
those layers are mostly plumbing and integration (clap parsing, IO, MCP protocol wrappers).

## Current Coverage (Local Measurement)

As of the last local run:

- Core gate (core-only view): `regions=88.35% functions=83.45% lines=89.16%`
- Workspace totals (all crates): `regions=63.55% functions=61.93% lines=64.94%`

Note: workspace totals include `workmesh-mcp/src/tools.rs` and `workmesh-cli/src/main.rs`, which are
large integration-heavy surfaces and will generally have lower line coverage.

## How To Measure Locally

Core gate (exactly what CI runs):

```bash
./scripts/ci_core_coverage_gate.sh 80
```

Workspace summary (useful for trend tracking, not currently enforced):

```bash
cargo llvm-cov --workspace --all-features --summary-only
```

If you want an LCOV file (for tooling/report uploads):

```bash
cargo llvm-cov --workspace --all-features --lcov --output-path target/lcov.info
```

## How To Use This For Regression Detection

- If `workmesh-core` drops below the floor: CI fails (hard stop).
- For the overall workspace %:
  - Use `cargo llvm-cov --summary-only` periodically, or on release tags.
  - If it trends down, add tests in the appropriate crate, or move logic down into `workmesh-core`
    where the coverage floor applies.

