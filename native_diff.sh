#!/bin/sh
# Builds each test program natively and compares its output with CARDIAC's. A test program is one file that defines
# its own built-ins at its root, as stubs (`pub fn out(_v: u8) {}`), which the compiler knows by name, so it needs no
# crate; natively the #![no_std] goes and out's stub gets a body that prints, and a panic is counted as -999 (the
# built-ins crate `cardiac`, in a program that uses it, prints that itself: its panic hook is set up before main).
# A program with a Cargo.toml beside it uses other crates: natively, the crates in $CRATES (NAME=PATH, PATH the
# crate's lib.rs) are built first and passed to it; on CARDIAC it is built with `cargo cardiac test --bin`.
# The arguments before the first FILE.rs are the CARDIAC build's settings, as `cargo cardiac test` takes them:
# rustc's flags for single files (-C opt-level=1 -C overflow-checks=on), a profile for a package's (--profile NAME).
set -e
SETTINGS=""
while [ $# -gt 0 ] && [ "${1%.rs}" = "$1" ]; do SETTINGS="$SETTINGS $1"; shift; done
NATIVE=${TMPDIR:-/tmp}/rustcardiac-native; mkdir -p $NATIVE
EXTERNS=""
for c in $CRATES; do
  name=${c%%=*}
  rustc --edition 2021 --crate-type rlib --crate-name $name -C overflow-checks=on -C opt-level=0 --out-dir $NATIVE "${c#*=}"
  EXTERNS="$EXTERNS --extern $name=$NATIVE/lib$name.rlib"
done
for f in "$@"; do
  n=$(basename "$f" .rs)
  sed -E -e "/#!\[no_std\]/d" -e "s#pub fn out\(_v: (u8|i16)\) \{\}#pub fn out(v: \1) { println!(\"{}\", v); }#" "$f" > $NATIVE/$n.rs
  rustc --edition 2021 -C overflow-checks=on -C opt-level=0 $EXTERNS -o $NATIVE/$n $NATIVE/$n.rs 2>/dev/null
  native=$($NATIVE/$n 2>/dev/null | tr '\n' ',' ; [ $? -ne 0 ] || true)
  # (an abort, as for a violated unsafe precondition, is not reported by this shell)
  if ! grep -q "^use cardiac::" "$f" && ! { $NATIVE/$n >/dev/null 2>&1; } 2>/dev/null; then native="${native}-999,"; fi
  manifest=$(dirname "$f")/Cargo.toml
  if [ -f "$manifest" ]; then
    cardiac=$(./target/release/cargo-cardiac test --manifest-path "$manifest" $SETTINGS --bin $n),
  else
    cardiac=$(./target/release/cargo-cardiac test $SETTINGS "$f"),
  fi
  [ "$native" = "$cardiac" ] && echo "MATCH  $n: $cardiac" || { echo "DIFFER $n: native=$native cardiac=$cardiac"; exit 1; }
done
