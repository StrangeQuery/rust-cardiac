#!/bin/sh
# The fuzzer (tests/fuzz/fuzz.rs): the programs in tests/fuzz/targets, run natively and on CARDIAC on
# generated inputs in every setting, every run compared. Arguments go to it: --runs N (300), --seed S (1),
# --only NAME. With --generate N it makes N programs of its own instead (from --start K, --batch B a build, 40
# runs each; --enums PCT of them with enums, 30), and --repro FILE.rs [-- INPUT...] builds and runs one, as a finding is checked again; --tool PATH
# names another cargo-cardiac. The decks run on burn: $BURN, or burn on the PATH. Its packages, decks, own build
# and findings are in target/fuzz/.
cd "$(dirname "$0")"
cargo build --release -q || exit 1
mkdir -p target/fuzz
rustc --edition 2021 -O -o target/fuzz/fuzz tests/fuzz/fuzz.rs || exit 1
MARK=$(mktemp)
target/fuzz/fuzz . "$@"
status=$?
sh ice_check.sh "$MARK" || status=1
exit $status
