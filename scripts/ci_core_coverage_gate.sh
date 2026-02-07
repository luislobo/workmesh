#!/usr/bin/env bash
set -euo pipefail

min="${1:-80}"

if ! command -v rg >/dev/null 2>&1; then
  echo "error: rg (ripgrep) is required for this script" >&2
  exit 2
fi

cargo llvm-cov clean --workspace >/dev/null

out="$(
  cargo llvm-cov --workspace --all-features \
    --ignore-filename-regex 'crates/workmesh-(cli|mcp)/' \
    --summary-only
)"

total_line="$(printf '%s\n' "$out" | rg '^TOTAL' | tail -n 1 || true)"
if [[ -z "${total_line}" ]]; then
  echo "error: coverage output did not contain a TOTAL line" >&2
  exit 2
fi

# Extract the three percentages from the TOTAL row: Regions, Functions, Lines.
mapfile -t covs < <(printf '%s\n' "$total_line" | awk '{for(i=1;i<=NF;i++) if($i ~ /%$/) print $i}')
if [[ "${#covs[@]}" -lt 3 ]]; then
  echo "error: could not parse TOTAL row percentages from:" >&2
  echo "$total_line" >&2
  exit 2
fi

regions="${covs[0]%\%}"
functions="${covs[1]%\%}"
lines="${covs[2]%\%}"

fail=0
check() {
  local name="$1"
  local value="$2"
  awk -v v="$value" -v m="$min" 'BEGIN{exit (v+0 < m+0)}' || {
    echo "coverage gate failed: ${name} ${value}% < ${min}%" >&2
    fail=1
  }
}

check "regions" "$regions"
check "functions" "$functions"
check "lines" "$lines"

if [[ "$fail" -ne 0 ]]; then
  echo "TOTAL: $total_line" >&2
  exit 1
fi

echo "coverage gate ok (min ${min}%): regions=${regions} functions=${functions} lines=${lines}"

