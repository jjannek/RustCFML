#!/usr/bin/env bash
# PR-0 benchmark runner for the RustCFML performance plan.
#
# For each bench in examples/perf/bench_*.cfm:
#   - runs it REPEATS times in release mode
#   - takes the MEDIAN of the bench's self-reported wall-clock (getTickCount,
#     more precise than wrapping the whole process which includes startup)
#   - records peak RSS from /usr/bin/time -l (macOS) / -v (GNU)
#   - appends one row per bench to results.csv, tagged with the git short SHA
#
# Usage:
#   examples/perf/run.sh            # build + run all benches, append to CSV
#   REPEATS=7 examples/perf/run.sh  # more samples
#   examples/perf/run.sh --no-build # skip cargo build (reuse existing binary)
#
# Compare across PRs by diffing results.csv rows by their `sha` column.
set -euo pipefail

cd "$(dirname "$0")/../.."   # repo root
REPEATS="${REPEATS:-5}"
BIN="./target/release/rustcfml"
CSV="examples/perf/results.csv"

if [[ "${1:-}" != "--no-build" ]]; then
    echo "building release..."
    cargo build --release 2>&1 | tail -1
fi
[[ -x "$BIN" ]] || { echo "missing $BIN — build first"; exit 1; }

SHA="$(git rev-parse --short HEAD 2>/dev/null || echo nogit)"
DIRTY=""
git diff --quiet 2>/dev/null || DIRTY="+dirty"
STAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# peak RSS in KB, cross-platform: macOS `time -l` reports bytes, GNU `-v` reports KB.
peak_rss_kb() {
    local tmp; tmp="$(mktemp)"
    if /usr/bin/time -l "$@" >/dev/null 2>"$tmp"; then :; fi
    # macOS: "  N  maximum resident set size" (bytes)
    local bytes; bytes="$(awk '/maximum resident set size/ {print $1}' "$tmp" | head -1)"
    rm -f "$tmp"
    if [[ -n "$bytes" ]]; then echo $(( bytes / 1024 )); else echo 0; fi
}

median() { printf '%s\n' "$@" | sort -n | awk '{a[NR]=$1} END{print (NR%2)? a[(NR+1)/2] : int((a[NR/2]+a[NR/2+1])/2)}'; }

[[ -f "$CSV" ]] || echo "sha,stamp,bench,median_ms,min_ms,max_ms,peak_rss_kb,repeats" > "$CSV"

echo "sha=$SHA$DIRTY repeats=$REPEATS"
printf '%-16s %10s %10s %10s %12s\n' bench median_ms min_ms max_ms peak_rss_kb

for f in examples/perf/bench_*.cfm; do
    name="$(basename "$f" .cfm | sed 's/^bench_//')"
    samples=()
    for _ in $(seq "$REPEATS"); do
        ms="$("$BIN" "$f" | awk '/^RESULT/ {print $2}')"
        [[ -n "$ms" ]] || { echo "  $name: no RESULT line — bench failed"; ms=0; }
        samples+=("$ms")
    done
    med="$(median "${samples[@]}")"
    mn="$(printf '%s\n' "${samples[@]}" | sort -n | head -1)"
    mx="$(printf '%s\n' "${samples[@]}" | sort -n | tail -1)"
    rss="$(peak_rss_kb "$BIN" "$f")"
    printf '%-16s %10s %10s %10s %12s\n' "$name" "$med" "$mn" "$mx" "$rss"
    echo "$SHA$DIRTY,$STAMP,$name,$med,$mn,$mx,$rss,$REPEATS" >> "$CSV"
done

echo "appended to $CSV"
