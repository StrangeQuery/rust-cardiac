#!/bin/sh
# The oracle (driver/src/check.rs) over everything the compiler is tested on: the IR the CARDIAC driver builds from
# rustc_public against the IR the compiler parses from rustc's MIR text (made in memory), item by item, for every
# crate rustc compiles for CARDIAC. A program's report has two parts: its own crate; and each crate it uses,
# converted from that crate's metadata, against rustc's text of the same bodies ("PROGRAM > CRATE"). Each
# test program at opt-level 1 and 2, overflow checks on and off (and debug assertions on); the two-crate package in
# each of its profiles; the examples (dev, release, checked); the fuzzer's programs and as many of its generated ones
# as FUZZ says (300).
#
#     sh ir_check.sh
#
# The reports are in target/ir_check/SOURCE.log, one line per crate and the first difference of each item that
# differs; the summary is printed at the end. "renamed" counts the conversion's one deliberate difference: an
# impl's function a call reaches is named as the call names it, and compared under the text's name. It
# exits 1 if any item differs or a file doesn't read back.
cd "$(dirname "$0")"
ROOT=$PWD
cargo build --release -q || exit 1
(cd driver && cargo build --release -q) || exit 1
LOG=$ROOT/target/ir_check
rm -rf "$LOG"; mkdir -p "$LOG/meta"
DRIVER=$ROOT/driver/target/release/cardiac-driver
TOOL=$ROOT/target/release/cargo-cardiac
SYSROOT=$(rustup run nightly-2026-05-22 rustc --print sysroot) || exit 1
export DYLD_FALLBACK_LIBRARY_PATH="$SYSROOT/lib" LD_LIBRARY_PATH="$SYSROOT/lib"

# the test programs, each compiled as `cargo cardiac test` compiles one (driver::single), its metadata the only file
for f in tests/programs/*.rs; do
  n=$(basename "$f" .rs)
  for o in 1 2; do
    for flags in "-C overflow-checks=on" "-C overflow-checks=off" "-C overflow-checks=off -C debug-assertions=on"; do
      tag=$(echo "$flags" | sed -e 's/-C //g' -e 's/overflow-checks=/checks-/' -e 's/debug-assertions=on/asserted/' -e 's/ /-/g')
      CARDIAC_IR_CHECK=$LOG/programs.log $DRIVER --edition 2021 --crate-type lib --cfg cardiac -A dead_code \
        -C opt-level=$o $flags --emit=metadata -o "$LOG/meta/$n-opt$o-$tag.rmeta" "$f" 2>/dev/null
    done
  done
done

# packages, built as `cargo cardiac build` builds them, in a target directory of their own (so cargo builds every
# crate again, and the driver sees each, and keeps each dependency's IR for its programs to check theirs against)
package() {   # LOG-NAME TARGET-NAME ARGS...
  log=$1; t=$2; shift 2
  CARGO_TARGET_DIR=$LOG/target-$t CARDIAC_IR_CHECK=$LOG/$log.log $TOOL build -q "$@" >/dev/null 2>&1
}
for p in opt1 opt1-unchecked opt2 opt2-unchecked opt1-asserted opt2-asserted; do
  package crates crates --manifest-path tests/programs/crates/Cargo.toml --profile $p
done
for p in dev release checked; do
  package examples examples --manifest-path examples/Cargo.toml --profile $p
  package examples examples --manifest-path examples/Cargo.toml --profile $p -p collatz
done

# the fuzzer's programs, built in every setting it has (its packages' CARDIAC builds started afresh)
rm -rf target/fuzz/pkg/target/cardiac-unknown-none target/fuzz/gen/pkg/target/cardiac-unknown-none
CARDIAC_IR_CHECK=$LOG/fuzz.log sh fuzz.sh --runs 1 >/dev/null 2>&1
CARDIAC_IR_CHECK=$LOG/fuzz-generated.log sh fuzz.sh --generate "${FUZZ:-300}" --runs 1 --no-shrink >/dev/null 2>&1

status=0
echo "source                 crates    items    equal  unequal  renamed  read back"
for l in "$LOG"/*.log; do
  awk -v src="$(basename "$l" .log)" '
    / items, / { c++; n = split($0, w, " ")
      for (k = 1; k <= n; k++) if (w[k] == "items,") { i += w[k-1]; e += w[k+1]; u += w[k+3] } else if (w[k] == "renamed") r += w[k-1]
      if ($0 !~ /text ok, converted ok/) b++ }
    END { printf "%-20s %8d %8d %8d %8d %8d  %s\n", src, c, i, e, u, r, (b ? b " DIFFER" : "ok"); exit (u || b) }' "$l" || status=1
done
[ $status = 0 ] || { echo "differences (first of each item):"; grep -h -A2 ' differs$\|only in the\|only converted' "$LOG"/*.log | head -60; }
exit $status
