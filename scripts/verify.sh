#!/usr/bin/env bash
# Full pre-tag verification gate for RustCFML.
#
# Runs everything that must be green before cutting a release tag, INCLUDING
# the wasm32-only workspace members (cfml-worker, rustcfml-wasm) that a plain
# `cargo build` silently skips. Skipping them is how reference-semantics
# changes have broken the Cloudflare worker for downstream consumers while the
# host build + test suite stayed green (see CLAUDE.md "Build & Test").
#
# Usage:  scripts/verify.sh
set -euo pipefail
cd "$(dirname "$0")/.."   # repo root

step() { printf '\n\033[1m==> %s\033[0m\n' "$1"; }

step "host release build"
cargo build --release 2>&1 | tail -1

step "Rust unit tests (incl. size_probe ceilings)"
cargo test 2>&1 | grep -E 'test result:|FAILED' | tail -20

step "CFML test suite (tests/runner.cfm)"
./target/release/rustcfml tests/runner.cfm 2>&1 | grep -E "FAIL \||ERROR|SUMMARY"

step "wasm32 target members (cfml-worker, rustcfml-wasm)"
if ! rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown; then
    echo "wasm32-unknown-unknown target missing — run: rustup target add wasm32-unknown-unknown"
    exit 1
fi
cargo build -p cfml-worker -p rustcfml-wasm --target wasm32-unknown-unknown 2>&1 | tail -1

printf '\n\033[1m==> all checks passed\033[0m\n'
echo "Cross-engine: also run the Lucee suite (box server start cfengine=lucee@7) before tagging."
