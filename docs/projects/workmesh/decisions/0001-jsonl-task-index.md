# Decision: JSONL task index

Date: 2026-02-04
Status: Accepted

## Context
We need a structured index derived from Markdown tasks to accelerate queries
(`ready`, reporting) while keeping Markdown as the source of truth.
We considered a binary index (B-Tree/SQLite) but want low friction, git-friendly
storage, and deterministic rebuilds.

## Decision
Use a newline-delimited JSON (JSONL) index at `workmesh/.index/tasks.jsonl`.
Each line is a full task snapshot with `mtime` and a `sha256` hash of the source
task file. The index is rebuildable and can be verified against Markdown.

## Rationale
- Git-friendly: JSONL merges are easy to review and resolve.
- Deterministic rebuilds from Markdown (source of truth).
- Simple to consume in CLIs and MCP.
- Good enough performance for the current scope; binary indexing can be added later.

## Consequences
- The index is optional and can be regenerated at any time.
- Query speed is improved without introducing a hard dependency on a database engine.
