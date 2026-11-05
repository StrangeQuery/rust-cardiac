#!/bin/sh
# The example programs, run both ways: `cargo run` on this machine, and their decks, built by `cargo cardiac build`,
# on burn, cardiaq's runtime ($BURN, or burn on the PATH), each with the same input on standard input. The two must
# print exactly the same, and end the same: a panic exits 101 both ways (CARDIAC outputs -999; burn says why from the
# panic map beside the deck, as `cargo run` does).
cd "$(dirname "$0")"
MARK=$(mktemp)
BURN=${BURN:-burn}
command -v "$BURN" >/dev/null || { echo "test_examples.sh needs burn, from cardiaq: put it on the PATH, or set BURN"; exit 1; }
cargo build --release -q || exit 1
export PATH="$PWD/target/release:$PATH"
cd examples
cargo build -q --bins || exit 1
status=0
profile=""   # the profile both runs use: cargo's dev, or --profile NAME
build() {   # the decks, with the profile; a build with another writes the same files
  cargo cardiac build -q $profile && cargo cardiac build -q $profile -p collatz || exit 1
}
deck() {   # NAME: its deck; an example that is a package of its own is in a directory of its name
  if [ -d "$1" ]; then echo "$1/$1.cqb"; else echo "$1.cqb"; fi
}
check() {   # NAME INPUT...
  name=$1; shift
  # an example that is a package of its own, in a directory of its name, is picked with -p as cargo picks it
  if [ -d "$name" ]; then which="-p $name"; else which="--bin $name"; fi
  native=$(echo "$@" | cargo run -q $profile $which 2>/dev/null; echo "exit $?")
  cardiac=$(echo "$@" | "$BURN" "$(deck $name)" 2>/dev/null; echo "exit $?")
  label="$name${profile:+ ($profile)}"
  if [ "$native" = "$cardiac" ]; then echo "SAME   $label $*: $(echo $cardiac)"
  else echo "DIFFER $label $*"; echo "  cargo run: $(echo $native)"; echo "  burn:      $(echo $cardiac)"; status=1; fi
}
# MapReduce: each shard (a quoted list of words) through `map`, its output kept; the outputs stacked behind a
# card saying how many, through `reduce`. All of it natively and all of it on CARDIAC; the totals must be the same.
run() {   # WAY NAME, the input on standard input
  if [ "$1" = native ]; then cargo run -q $profile --bin "$2" 2>/dev/null; else "$BURN" "$2.cqb" 2>/dev/null; fi
}
mapreduce() {   # SHARD...
  for way in native cardiac; do
    cards="$#"
    for shard in "$@"; do cards="$cards $(echo $shard -999 | run $way map | tr '\n' ' ')"; done
    eval "$way=\"$(echo $cards | run $way reduce | tr '\n' ' ')\""
  done
  label="mapreduce${profile:+ ($profile)}"
  if [ "$native" = "$cardiac" ]; then echo "SAME   $label $#: $cardiac"
  else echo "DIFFER $label $#"; echo "  native:  $native"; echo "  cardiac: $cardiac"; status=1; fi
}
examples() {
check hello
check fibonacci
check fizzbuzz
check primes
check gcd 252 105
check gcd 17 5
check collatz 7
check collatz 27                       # climbs past 999: both print -999 and stop
check sort 31 -4 15 9 2 0 -999 8 15 600
check sort 5 4 3 2 1 999 999 -1 0 1
check powers 20
check powers 99                        # two phases
check powers 0
check cipher 120 7 999                 # the key is the resource key, examples/resources/key
check days 2
check days 13                          # past the table: both panic, index out of bounds
check days 0                           # a negative index: both panic too
check runs 3 0 -2 5 7 0 -4 -6 2 0 9 -999 # flags: cardiac::Flag
check runs 4 5 -999                     # no zero seen
check rot13 8 5 12 12 15 0 23 15 18 12 4  # HELLO WORLD: URYYB JBEYQ
check rot13 21 18 25 25 2 0 27 -5        # and back; what isn't a letter goes through
check guess 123 50 75 62 56 59 61          # seed 123: the number is 61, found on the sixth guess
check guess 0 1 99 2                      # seed 0: the next number is 17, so 2 (17 / 10 + 1)
check nim 0 1 1                            # CARDIAC first: takes 2, 3, 3; you can't move, 999
check nim 1 2 3 3                          # you first, and right: CARDIAC takes 1, 1, then can't move, 0
check nim 1 2 2 1 3                        # your 1 just after CARDIAC's 1 is answered -1; then CARDIAC wins, 999
mapreduce "3 1 4 1 5 9 2 6 5 3 5" "8 9 7 9 3 2 3 8 4 6" "2 6 4 3 3 8 3 2 7 9 5 0 17"   # three shards; 0 and 17 skipped
check days_assumed 2
check days_assumed 12
check voxel 12 30 $(i=0; while [ $i -lt 96 ]; do h=$((i * 7 % 50)); printf '%s ' $((36 + i / 12)) $((10 * h + i % 6)) $((i * 3 % 50)) $((i * 5 % 50)); i=$((i + 1)); done)   # a slice: land above and below the camera, lit from every side
}
build; examples
# every example again, built with debug assertions on both ways
profile="--profile checked"; build; examples
# a broken assumption: natively it panics; on CARDIAC, built checked (debug assertions on), it panics too
native=$(echo 13 | cargo run -q --bin days_assumed 2>/dev/null; echo "exit $?")
cardiac=$(echo 13 | "$BURN" days_assumed.cqb 2>/dev/null; echo "exit $?")
[ "$native" = "$cardiac" ] && echo "SAME   days_assumed 13, checked: $(echo $cardiac)" || { echo "DIFFER days_assumed 13, checked"; echo "  cargo run: $(echo $native)"; echo "  burn: $(echo $cardiac)"; status=1; }
profile=""; build
# the reason for a panic, from the panic map beside the deck
reason=$(echo 13 | "$BURN" days.cqb 2>&1 >/dev/null | tail -1)
[ "$reason" = "burn: days.cqb: panicked in main: index out of bounds" ] && echo "SAME   days 13: $reason" || { echo "DIFFER days 13: $reason"; status=1; }
# --message-format=json: each message on standard error a JSON object, its kind in "reason"
reasons() {   # ARGS...
  cargo cardiac build --message-format=json "$@" 2>&1 >/dev/null | sed 's/^{.*"reason":"\([a-z-]*\)".*}$/\1/' | tr '\n' ' ' | sed 's/ $//'
}
json() {   # WHAT EXPECTED GOT
  [ "$2" = "$3" ] && echo "SAME   $1: $3" || { echo "DIFFER $1"; echo "  expected: $2"; echo "  got:      $3"; status=1; }
}
json "build days, json" "compiling compiled" "$(reasons --bin days)"
json "build days -q, json" "" "$(reasons -q --bin days)"
sh ../ice_check.sh "$MARK" || status=1
exit $status
