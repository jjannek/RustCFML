#!/usr/bin/env bash
# JIT BASELINE — run all four workloads under interpreter + default JIT,
# 3 trials each, report min wall-clock + speedup. Use as the reference
# point for v0.88.0/v0.89.0/v0.90.0 perf claims.
#
# Usage: from repo root,
#   cargo build --release
#   bench/baseline/run_baseline.sh
set -u

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="$REPO_ROOT/target/release/rustcfml"
[[ -x "$BIN" ]] || { echo "release binary missing — cargo build --release first"; exit 1; }

bench_one() {
    local label="$1" file="$2"
    local mins_off=99999 mins_on=99999 t
    for i in 1 2 3; do
        t=$(/usr/bin/time -p env RUSTCFML_JIT=0 "$BIN" "$file" 2>&1 >/dev/null | awk '/^real/{print $2}')
        awk -v a="$t" -v b="$mins_off" 'BEGIN{ exit !(a<b) }' && mins_off="$t"
    done
    for i in 1 2 3; do
        t=$(/usr/bin/time -p "$BIN" "$file" 2>&1 >/dev/null | awk '/^real/{print $2}')
        awk -v a="$t" -v b="$mins_on" 'BEGIN{ exit !(a<b) }' && mins_on="$t"
    done
    local speedup
    speedup=$(awk -v a="$mins_off" -v b="$mins_on" 'BEGIN{ if (b>0) printf "%.2fx", a/b; else print "?" }')
    printf "%-26s  interp %6ss   jit %6ss   %s\n" "$label" "$mins_off" "$mins_on" "$speedup"
}

echo "JIT BASELINE  $(date '+%Y-%m-%d %H:%M:%S')  $(cd "$REPO_ROOT" && git describe --tags --dirty 2>/dev/null || echo 'unknown')"
echo "==================================================================="
bench_one "numeric_kernel"        "$REPO_ROOT/bench/baseline/numeric_kernel.cfm"
bench_one "udf_call_graph"        "$REPO_ROOT/bench/baseline/udf_call_graph.cfm"
bench_one "string_kernel"         "$REPO_ROOT/bench/baseline/string_kernel.cfm"
bench_one "struct_member_kernel"  "$REPO_ROOT/bench/baseline/struct_member_kernel.cfm"
echo "==================================================================="
echo "tests/runner.cfm full-suite as a representative breadth workload:"
bench_one "tests/runner.cfm"      "$REPO_ROOT/tests/runner.cfm"
