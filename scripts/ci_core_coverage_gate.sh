#!/usr/bin/env bash
set -euo pipefail

min="${1:-80}"

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/llvm-cov-ci}"
export LLVM_PROFILE_FILE="${LLVM_PROFILE_FILE:-${CARGO_TARGET_DIR}/profraw/%p-%m.profraw}"
# Reduce flakiness in profile merging by running tests single-threaded.
export RUST_TEST_THREADS="${RUST_TEST_THREADS:-1}"

mkdir -p "$(dirname "$LLVM_PROFILE_FILE")"

cargo llvm-cov clean --workspace >/dev/null

out="$(
  cargo llvm-cov --workspace --all-features \
    --ignore-filename-regex 'crates/workmesh-(cli|mcp)/' \
    --summary-only
)"

total_line="$(printf '%s\n' "$out" | awk '$1=="TOTAL"{line=$0} END{print line}')"
if [[ -z "${total_line}" ]]; then
  echo "error: coverage output did not contain a TOTAL line" >&2
  exit 2
fi

table="$(
  # Extract the per-file table (Filename..TOTAL) from the summary.
  # The table has stable whitespace-delimited columns.
  printf '%s\n' "$out" | awk '
    $1=="Filename"{in_table=1; next}
    in_table==1 && $1=="TOTAL"{print; exit}
    in_table==1 {print}
  '
)"
if [[ -z "${table}" ]]; then
  echo "error: coverage output did not contain a per-file table" >&2
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

# Enforce the same minimum across all core files.
#
# We enforce per-file Lines and Regions. Per-file Functions coverage is often noisy in Rust
# because iterator/closure desugarings are counted as separate "functions" by llvm-cov.
#
# Column positions (whitespace-delimited):
# 1=Filename 4=RegionsCover 10=LinesCover
while IFS= read -r row; do
  [[ -z "$row" ]] && continue
  name="$(printf '%s\n' "$row" | awk '{print $1}')"
  [[ "$name" == "TOTAL" ]] && continue
  # Only enforce on Rust source files.
  [[ "$name" != *.rs ]] && continue

  r="$(printf '%s\n' "$row" | awk '{print $4}' | sed 's/%$//')"
  l="$(printf '%s\n' "$row" | awk '{print $10}' | sed 's/%$//')"

  check "${name} regions" "$r"
  check "${name} lines" "$l"
done <<<"$table"

if [[ "$fail" -ne 0 ]]; then
  echo "TOTAL: $total_line" >&2
  echo "" >&2
  echo "Per-file coverage table (core-only view):" >&2
  echo "$table" >&2
  exit 1
fi

echo "coverage gate ok (min ${min}%): regions=${regions} functions=${functions} lines=${lines}"
