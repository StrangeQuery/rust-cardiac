#!/bin/sh
# The feature tests: one small program per Rust feature, in tests/features. Each is built for CARDIAC with every
# opt-level and overflow-check setting and run (by `cargo cardiac test`), and built and run natively, and the outputs
# are compared. It writes tests/features/results and tests/features/SUPPORT.md from what happened. A PATTERN runs
# only the rows whose path contains it.
#
#     ./test_features.sh [PATTERN]
#     ./test_features.sh --compare BASE OUT    # the rows whose outcomes changed between two sets of results
#
# SUPPORT_OUT=DIR writes the results there instead, to compare a branch with the committed ones.
# paths are the caller's: make them whole before moving to the harness's directory
abs() { case $1 in /*) echo "$1" ;; *) echo "$PWD/$1" ;; esac; }
if [ "$1" = --compare ] && [ $# -eq 3 ]; then set -- --compare "$(abs "$2")" "$(abs "$3")"; fi
if [ -n "${SUPPORT_OUT:-}" ]; then SUPPORT_OUT=$(abs "$SUPPORT_OUT"); export SUPPORT_OUT; fi
cd "$(dirname "$0")"
cargo build --release -q || exit 1
(cd driver && cargo build --release -q) || exit 1
cd tests/features && exec cargo run --release -q -- "$@"
