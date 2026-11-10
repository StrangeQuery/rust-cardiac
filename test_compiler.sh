#!/bin/sh
# The compiler's tests: every test program in tests/programs built natively and on CARDIAC, outputs compared; then
# card_overflow, which panics on CARDIAC by design where native Rust carries on, checked against its expected output.
# All of it twice: from rustc's MIR at opt-level 1, and at opt-level 2 (what `cargo cardiac build` uses: small
# functions inlined). Programs of two crates are in tests/programs/crates, a Cargo package. A program with overlay()
# (phases.rs) is split into phases and run as its decks are, one deck per phase; phases2.rs has a block both phases
# need. The example programs have a test of their own, test_examples.sh. Decks run on burn, heartburn's runtime: $BURN,
# or burn on the PATH.
cd "$(dirname "$0")"
MARK=$(mktemp)
BURN=${BURN:-burn}
command -v "$BURN" >/dev/null || { echo "test_compiler.sh needs burn, heartburn's runtime: put it on the PATH, or set BURN"; exit 1; }
panicked() {   # DECK [-- INPUT...]: why a run of the deck panicked, as burn says it from the panic map, and its exit
  why=$("$BURN" "$@" 2>&1 >/dev/null); code=$?
  echo "$why" | tail -1 | sed 's/^burn: [^ ]*: //'; echo "exit $code"
}
cards() {   # N V: a deck of N cards, each V
  i=0; while [ $i -lt $1 ]; do printf '%s ' $2; i=$((i+1)); done
}
cargo build --release -q || exit 1
# the CARDIAC driver (rustc on a pinned nightly, and the backend that compiles each program), as cargo cardiac finds it
(cd driver && cargo build --release -q) || exit 1
for opt in 1 2; do
  echo "-- MIR at opt-level $opt"
  checked="-C opt-level=$opt -C overflow-checks=on"
  unchecked="-C opt-level=$opt -C overflow-checks=off"
  sh native_diff.sh $checked tests/programs/countdown.rs tests/programs/underflow.rs tests/programs/fib.rs tests/programs/fib_overflow.rs tests/programs/add.rs \
    tests/programs/nested.rs tests/programs/divide.rs tests/programs/fizzbuzz.rs tests/programs/arrays.rs tests/programs/refs.rs tests/programs/raw.rs tests/programs/oob.rs \
    tests/programs/smul.rs tests/programs/scmp.rs tests/programs/sdiv2.rs tests/programs/digit.rs tests/programs/structs.rs tests/programs/tables.rs \
    tests/programs/forloops.rs tests/programs/forloops2.rs tests/programs/forloops3.rs tests/programs/forloops4.rs tests/programs/phases.rs tests/programs/phases2.rs \
    tests/programs/addr_out.rs tests/programs/addr_out_mut.rs tests/programs/resources.rs tests/programs/loop_call.rs tests/programs/array_by_value.rs tests/programs/compare_right.rs tests/programs/compare_left.rs tests/programs/compare_both.rs \
    tests/programs/compare_past.rs tests/programs/compare_past_left.rs tests/programs/promise_and.rs tests/programs/divide_branches.rs \
    tests/programs/const_arrays.rs tests/programs/checked_cast.rs tests/programs/joined_tails.rs \
    tests/programs/abs.rs tests/programs/minmax.rs tests/programs/minmax2.rs tests/programs/wrapping.rs \
    tests/programs/clamp_order.rs tests/programs/euclid_zero.rs tests/programs/saturating.rs \
    tests/programs/div_element.rs tests/programs/rem_element.rs \
    tests/programs/div_computed.rs tests/programs/rem_computed.rs tests/programs/sdiv_computed.rs tests/programs/srem_computed.rs tests/programs/div5_computed.rs tests/programs/div_in_place.rs tests/programs/div_by_own.rs \
    tests/programs/unread_checked.rs tests/programs/empty_for.rs \
    tests/programs/settled_oob.rs tests/programs/settled_rem.rs tests/programs/settled_neg.rs tests/programs/panic_after_loop.rs \
    tests/programs/switch_negative.rs tests/programs/switch_negative2.rs \
    tests/programs/oob_after_loop.rs tests/programs/oob_after_empty.rs \
    tests/programs/overwrite_checked.rs \
    tests/programs/fold_twice.rs \
    tests/programs/huge_usize.rs tests/programs/u8_max.rs tests/programs/u8_max_args.rs \
    tests/programs/enums_c.rs tests/programs/enums_c2.rs tests/programs/methods.rs \
    tests/programs/option_methods.rs tests/programs/try_result.rs tests/programs/try_option.rs tests/programs/enum_consts.rs tests/programs/enums_for.rs tests/programs/niche.rs tests/programs/niche_u8.rs tests/programs/niche_negative.rs tests/programs/checked.rs tests/programs/checked_mul.rs tests/programs/niche_eq.rs tests/programs/struct_const_arg.rs tests/programs/struct_refs.rs tests/programs/tuples_return.rs tests/programs/tuples_return2.rs tests/programs/enums_eq.rs tests/programs/enums_ne.rs tests/programs/past_card_divisor.rs tests/programs/result_in_place.rs tests/programs/result_in_place2.rs tests/programs/result_in_place3.rs tests/programs/not_element.rs tests/programs/switch_elements.rs tests/programs/tuple_struct_consts.rs tests/programs/tuple_struct_eq.rs tests/programs/not_compare.rs tests/programs/range_contains.rs tests/programs/digit_const.rs tests/programs/const_cast.rs tests/programs/option_map.rs tests/programs/result_map.rs tests/programs/array_return.rs tests/programs/enum_mut.rs tests/programs/enum_field.rs tests/programs/enum_field_mut.rs tests/programs/enum_payload.rs tests/programs/enum_payload_array.rs tests/programs/enum_payload_mut.rs tests/programs/struct_mut.rs tests/programs/struct_nested.rs tests/programs/struct_array_field.rs tests/programs/count_loops.rs tests/programs/count_down.rs tests/programs/count_result.rs tests/programs/match_ranges.rs tests/programs/switch_consecutive.rs tests/programs/count_param.rs tests/programs/count_reset.rs tests/programs/count_inner.rs tests/programs/phase_init.rs tests/programs/match_desc.rs tests/programs/contains_dead.rs tests/programs/div_euclid.rs tests/programs/option_tuple.rs tests/programs/mask_counter.rs tests/programs/mask_condition.rs tests/programs/mask_signed.rs tests/programs/mask_unsigned.rs tests/programs/bool_kept.rs tests/programs/bool_and_kept.rs tests/programs/add_times.rs tests/programs/ref_returned.rs tests/programs/card_limits.rs || exit 1
  # small integer methods rustc calls at opt-level 1; at 2 it inlines signum into a Cmp, which is refused
  if [ $opt = 1 ]; then sh native_diff.sh $checked tests/programs/signum.rs || exit 1; fi
  # and ones it inlines at 2 that test for i16::MIN, or wrap a u8
  if [ $opt = 2 ]; then
    for c in "$checked" "$unchecked"; do
      sh native_diff.sh $c tests/programs/checked_limits.rs tests/programs/overflowing_u8.rs || exit 1
    done
  fi
  # arrays of arrays laid out by tens, with the program's other cells in the holes between the rows, and the ones that
  # stay row after row; each ends in a panic, an index past a row's end or the last row
  for c in "$checked" "$unchecked"; do
    sh native_diff.sh $c tests/programs/grid_bool.rs tests/programs/grid_border.rs tests/programs/grid_column_past.rs \
      tests/programs/grid_copy.rs tests/programs/grid_offsets.rs tests/programs/grid_one.rs tests/programs/grid_refs.rs \
      tests/programs/grid_row_past.rs tests/programs/grid_ten.rs tests/programs/grid_wide.rs || exit 1
  done
  # `?` converting the error with From: rustc's MIR inliner, on at opt-level 1 too, inlines the conversion at both
  sh native_diff.sh $unchecked tests/programs/try_from.rs || exit 1
  # and as `cargo cardiac build` compiles, a release build (no overflow checks): the division routine, every sign
  sh native_diff.sh $unchecked tests/programs/sdivvar.rs tests/programs/sremvar.rs tests/programs/sremvar2.rs tests/programs/sdiv.rs tests/programs/smulvar.rs \
    tests/programs/smul2.rs tests/programs/structs2.rs tests/programs/repeats.rs tests/programs/repeats2.rs tests/programs/structs3.rs tests/programs/structs4.rs tests/programs/exprhelp.rs tests/programs/exprif.rs tests/programs/exprtuple.rs tests/programs/forloops.rs tests/programs/forloops2.rs tests/programs/forloops3.rs tests/programs/forloops4.rs tests/programs/phases.rs tests/programs/phases2.rs \
    tests/programs/forward.rs tests/programs/lin_plain.rs tests/programs/addr_out.rs tests/programs/addr_out_mut.rs tests/programs/compare_right.rs tests/programs/compare_left.rs tests/programs/compare_both.rs \
    tests/programs/compare_past.rs tests/programs/compare_past_left.rs tests/programs/promise_and.rs tests/programs/divide_branches.rs \
    tests/programs/const_arrays.rs tests/programs/joined_tails.rs tests/programs/joined_tails2.rs \
    tests/programs/abs.rs tests/programs/wrapping.rs tests/programs/euclid.rs tests/programs/euclid2.rs \
    tests/programs/div_element.rs tests/programs/rem_element.rs \
    tests/programs/div_computed.rs tests/programs/rem_computed.rs tests/programs/sdiv_computed.rs tests/programs/srem_computed.rs tests/programs/div5_computed.rs tests/programs/div_in_place.rs tests/programs/div_by_own.rs \
    tests/programs/dead_growth.rs tests/programs/empty_for.rs \
    tests/programs/settled_oob.rs tests/programs/settled_rem.rs tests/programs/settled_neg.rs tests/programs/panic_after_loop.rs \
    tests/programs/switch_negative2.rs \
    tests/programs/oob_after_loop.rs tests/programs/oob_after_empty.rs \
    tests/programs/fold_twice.rs \
    tests/programs/fold_stale.rs \
    tests/programs/huge_usize.rs tests/programs/u8_max.rs tests/programs/u8_max_args.rs \
    tests/programs/enums_c.rs tests/programs/enums_c2.rs tests/programs/methods.rs \
    tests/programs/option_methods.rs tests/programs/result_methods.rs tests/programs/try_result.rs tests/programs/try_option.rs tests/programs/enum_consts.rs tests/programs/enums_for.rs tests/programs/niche.rs tests/programs/niche_u8.rs tests/programs/niche_negative.rs tests/programs/checked.rs tests/programs/checked_mul.rs tests/programs/niche_eq.rs tests/programs/option_eq.rs tests/programs/struct_const_arg.rs tests/programs/struct_refs.rs tests/programs/tuples_return.rs tests/programs/tuples_return2.rs tests/programs/enums_eq.rs tests/programs/enums_ne.rs tests/programs/past_card_divisor.rs tests/programs/result_in_place.rs tests/programs/result_in_place2.rs tests/programs/result_in_place3.rs tests/programs/not_element.rs tests/programs/switch_elements.rs tests/programs/tuple_struct_consts.rs tests/programs/tuple_struct_eq.rs tests/programs/not_compare.rs tests/programs/range_contains.rs tests/programs/digit_const.rs tests/programs/const_cast.rs tests/programs/option_map.rs tests/programs/result_map.rs tests/programs/array_return.rs tests/programs/enum_mut.rs tests/programs/enum_field.rs tests/programs/enum_field_mut.rs tests/programs/enum_payload.rs tests/programs/enum_payload_array.rs tests/programs/enum_payload_mut.rs tests/programs/struct_mut.rs tests/programs/struct_nested.rs tests/programs/struct_array_field.rs tests/programs/count_loops.rs tests/programs/count_down.rs tests/programs/count_result.rs tests/programs/match_ranges.rs tests/programs/switch_consecutive.rs tests/programs/count_param.rs tests/programs/count_reset.rs tests/programs/count_inner.rs tests/programs/phase_init.rs tests/programs/match_desc.rs tests/programs/contains_dead.rs tests/programs/div_euclid.rs tests/programs/option_tuple.rs tests/programs/mask_counter.rs tests/programs/mask_condition.rs tests/programs/mask_signed.rs tests/programs/mask_unsigned.rs tests/programs/bool_kept.rs tests/programs/bool_and_kept.rs tests/programs/bool_and_loop.rs tests/programs/sub_two_elements.rs tests/programs/add_times.rs tests/programs/ref_returned.rs tests/programs/card_limits.rs || exit 1
  # with debug assertions on, rustc keeps core's checks of unsafe preconditions: each is checked, and a for
  # loop's step needs none; rustc inlines unchecked_add and its kin (and so their checks) at opt-level 2
  asserted="-C debug-assertions=on"
  sh native_diff.sh $unchecked $asserted tests/programs/precond_loops.rs || exit 1
  # assert_unchecked is checked at both: inlined at 2, a call of core's at 1, which the MIR doesn't show is a check
  # with debug assertions on, so the build's setting tells
  for c in "$checked" "$unchecked"; do
    sh native_diff.sh $c $asserted tests/programs/precond_assume.rs || exit 1
  done
  ./target/release/cargo-cardiac build -q $unchecked $asserted tests/programs/precond_assume.rs || exit 1
  out=$(panicked tests/programs/precond_assume.cqb)
  [ "$out" = "panicked in half: unsafe precondition(s) violated: hint::assert_unchecked must never be called when the condition is false
exit 134" ] && echo "MATCH  precond_assume: aborted, exit 134" || { echo "DIFFER precond_assume: $out"; exit 1; }
  if [ $opt = 2 ]; then
    for c in "$checked" "$unchecked"; do
      sh native_diff.sh $c $asserted tests/programs/precond.rs tests/programs/precond_u8.rs || exit 1
    done
    # natively a violated precondition aborts (exit 134, SIGABRT), and burn exits so too
    ./target/release/cargo-cardiac build -q $unchecked $asserted tests/programs/precond.rs || exit 1
    out=$(panicked tests/programs/precond.cqb)
    [ "$out" = "panicked in sub: unsafe precondition(s) violated: usize::unchecked_sub cannot overflow
exit 134" ] && echo "MATCH  precond: aborted, exit 134" || { echo "DIFFER precond: $out"; exit 1; }
  fi
  # programs of two crates, built with cargo cardiac: each uses the crate kit, or the built-ins crate's assume()
  # (xassume fits only if a call to assume is a promise; with debug assertions, xbroken's must still be checked)
  crates="kit=tests/programs/crates/kit/src/lib.rs cardiac=cardiac/src/lib.rs"
  CRATES=$crates sh native_diff.sh --profile opt$opt tests/programs/crates/xcall.rs tests/programs/crates/xtable.rs \
    tests/programs/crates/xchain.rs tests/programs/crates/xassume.rs || exit 1
  CRATES=$crates sh native_diff.sh --profile opt$opt-unchecked tests/programs/crates/xcall.rs tests/programs/crates/xtable.rs \
    tests/programs/crates/xchain.rs tests/programs/crates/xstruct.rs tests/programs/crates/xassume.rs || exit 1
  CRATES=$crates sh native_diff.sh --profile opt$opt-asserted tests/programs/crates/xbroken.rs tests/programs/crates/xhint.rs || exit 1
  # skip_cards() and add_times() from the built-ins crate on decks, run natively (cargo run: the cardiac
  # crate reads them) and on CARDIAC: the same cards out, the same way of ending; and cards read straight into a
  # function's result
  repeat="8 3 0 3 1 -7 9 999 1 12 10 -27 37 1 99 -10 99 -83 -111 9 7 37"
  skip="6 0 0 11 1 7 0 12 9 $(cards 9 1) 3 $(cards 3 2) 13 10 $(cards 10 3) 0 14 37 $(cards 37 4) 1 5 15 99 $(cards 99 6)
    11 $(cards 11 7) 16 $(cards 9 8) 17"
  for p in opt$opt opt$opt-unchecked; do
    ./target/release/cargo-cardiac build -q --manifest-path tests/programs/crates/Cargo.toml --profile $p --bin xrepeat --bin xskip --bin xread || exit 1
    for run in "xrepeat:$repeat" "xskip:$skip" "xskip:1 999 $(cards 999 1) 0 18 $(cards 9 2) 19" \
      "xread:4 -5 6 70 8 -9"; do
      bin=${run%%:*}; deck=${run#*:}
      native=$(echo $deck | cargo run -q --manifest-path tests/programs/crates/Cargo.toml --profile $p --bin $bin 2>/dev/null | tr '\n' ','; echo "exit $?")
      cardiac=$("$BURN" tests/programs/crates/$bin.cqb -- $deck 2>/dev/null | tr '\n' ','; echo "exit $?")
      [ "$native" = "$cardiac" ] && echo "MATCH  $bin ($p): $cardiac" || { echo "DIFFER $bin ($p): native=$native cardiac=$cardiac"; exit 1; }
    done
  done
  # a generic function from another crate has no MIR to compile: a clear error
  out=$(./target/release/cargo-cardiac test --manifest-path tests/programs/crates/Cargo.toml --profile opt$opt --bin xgeneric 2>&1)
  case "$out" in *"call to first::<3>, a generic function in another crate"*) echo "MATCH  xgeneric: rejected";;
    *) echo "DIFFER xgeneric: $out"; exit 1;; esac
  # programs that must not compile: recursion (no stack), and one the borrow checker rejects
  out=$(./target/release/cargo-cardiac test $checked tests/programs/recursive.rs 2>&1)
  case "$out" in *"recursion (main -> down -> up -> down) needs a stack"*) echo "MATCH  recursive: rejected";;
    *) echo "DIFFER recursive: $out"; exit 1;; esac
  out=$(./target/release/cargo-cardiac test $checked tests/programs/borrow_error.rs 2>&1)
  case "$out" in *E0499*) echo "MATCH  borrow_error: rejected by rustc (E0499)";;
    *) echo "DIFFER borrow_error: $out"; exit 1;; esac
  # a resource is loaded once, in main and on no loop, as the loader fills its slot once
  for r in "resource_loop:is in a loop" "resource_helper:is only allowed in main" "resource_twice:is loaded twice"; do
    out=$(./target/release/cargo-cardiac test $checked tests/programs/${r%%:*}.rs 2>&1)
    case "$out" in *"${r#*:}"*) echo "MATCH  ${r%%:*}: rejected";;
      *) echo "DIFFER ${r%%:*}: $out"; exit 1;; esac
  done
  # with auto-overlay the compiler splits a program too big for memory into phases itself, and an overlay() of the
  # program's own is an error
  sh native_diff.sh $checked --auto-overlay tests/programs/auto_overlay.rs || exit 1
  grep -q "^;@@PHASE" tests/programs/auto_overlay.asm && echo "MATCH  auto_overlay: split into phases" ||
    { echo "DIFFER auto_overlay: not split"; exit 1; }
  out=$(./target/release/cargo-cardiac test $checked --auto-overlay tests/programs/phases.rs 2>&1)
  case "$out" in *"overlay() in main with auto-overlay on"*) echo "MATCH  phases: rejected with auto-overlay";;
    *) echo "DIFFER phases with auto-overlay: $out"; exit 1;; esac
  # a constant no card holds is said as it is, with its line, though the program has a slice elsewhere
  out=$(./target/release/cargo-cardiac test $checked tests/programs/const_range.rs 2>&1)
  case "$out" in *"constant out of range: 2000 is past what a card holds (-999 to 999): "*"would overflow a card, in main: \`_"*" = AddWithOverflow("*)
      echo "MATCH  const_range: rejected";;
    *) echo "DIFFER const_range: $out"; exit 1;; esac
  # i16::MAX as a value is a constant past a card; midpoint, inlined, is a sum past one, and a u8's overflowing_mul
  # a product past one; `?` converting with From is core's from_residual where rustc calls it, which can't be
  # written out without the conversion
  out=$(./target/release/cargo-cardiac test $checked tests/programs/limit_value.rs 2>&1)
  case "$out" in *"constant out of range: i16::MAX (32767) is past what a card holds"*) echo "MATCH  limit_value: rejected";;
    *) echo "DIFFER limit_value: $out"; exit 1;; esac
  # the same divided by 1000, which is 0 for anything a card holds: refused all the same, with overflow checks or
  # without, though nothing reads the difference
  for checks in on off; do
    out=$(./target/release/cargo-cardiac test -C opt-level=$opt -C overflow-checks=$checks tests/programs/limit_quotient.rs 2>&1)
    case "$out" in *"constant out of range: i16::MAX (32767) is past what a card holds"*) echo "MATCH  limit_quotient, checks $checks: rejected";;
      *) echo "DIFFER limit_quotient, checks $checks: $out"; exit 1;; esac
    out=$(./target/release/cargo-cardiac test -C opt-level=$opt -C overflow-checks=$checks tests/programs/const_quotient.rs 2>&1)
    case "$out" in *"constant out of range: 1500 is past what a card holds (-999 to 999): "*"would overflow a card, in main: \`_"*"(const 1500_i16, copy _1)"*)
        echo "MATCH  const_quotient, checks $checks: rejected";;
      *) echo "DIFFER const_quotient, checks $checks: $out"; exit 1;; esac
  done
  if [ $opt = 2 ]; then
    out=$(./target/release/cargo-cardiac test $checked tests/programs/midpoint.rs 2>&1)
    case "$out" in *"core::num::<impl i16>::midpoint, inlined in main, works in i32 so that it can't overflow"*) echo "MATCH  midpoint: rejected";;
      *) echo "DIFFER midpoint: $out"; exit 1;; esac
    out=$(./target/release/cargo-cardiac test $checked tests/programs/overflowing_mul_u8.rs 2>&1)
    case "$out" in *"a u8 product with its overflow flag"*) echo "MATCH  overflowing_mul_u8: rejected";;
      *) echo "DIFFER overflowing_mul_u8: $out"; exit 1;; esac
  fi
  # a check rustc has proved fails is the panic, and the deck's panic map says which; a panic main runs off its end
  # into (its JMP to the routine dropped as the routine comes next) is one too, and one a phase starts with
  for r in "settled_neg:index out of bounds" "settled_rem:attempt to calculate the remainder with a divisor of zero" \
    "panic_after_loop:attempt to divide by zero" "only_panic:explicit panic" "panic_phase:index out of bounds"; do
    ./target/release/cargo-cardiac build -q $checked tests/programs/${r%%:*}.rs || exit 1
    out=$(panicked tests/programs/${r%%:*}.cqb)
    [ "$out" = "panicked in main: ${r#*:}
exit 101" ] && echo "MATCH  ${r%%:*}: panicked, ${r#*:}" || { echo "DIFFER ${r%%:*}: $out"; exit 1; }
  done
  out=$(./target/release/cargo-cardiac test $checked tests/programs/card_overflow.rs)
  [ "$out" = "2,5,14,41,122,365,-999" ] && echo "MATCH  card_overflow: $out" || { echo "DIFFER card_overflow: $out"; exit 1; }
  # checked_add and checked_sub past a card are None, where natively they are Some up to 32767
  for c in "$checked" "$unchecked"; do
    out=$(./target/release/cargo-cardiac test $c tests/programs/checked_card.rs)
    [ "$out" = "994,-994,999,-999,-1,-2" ] && echo "MATCH  checked_card: $out" || { echo "DIFFER checked_card: $out"; exit 1; }
  done
  # a u8 wraps without overflow checks, and a usize below 0 is one past a card, which native_diff's build (checks
  # on) would panic at: what a release build of each prints, the panic last
  out=$(./target/release/cargo-cardiac test $unchecked tests/programs/u8_wrap.rs)
  [ "$out" = "2,203,15,59,4,44,255,200,0,10,-999" ] && echo "MATCH  u8_wrap: $out" || { echo "DIFFER u8_wrap: $out"; exit 1; }
  # without overflow checks a result past a card is what its cell keeps (1200 is 200), and a test or a sum that
  # reads it again after it is stored sees that, not the whole result left in the accumulator; nor does the range
  # pass take the whole result for what the cell holds
  out=$(./target/release/cargo-cardiac test $unchecked tests/programs/past_card_reread.rs)
  [ "$out" = "200,0,400,0,100,0,-100,0,300,0,-300,0" ] && echo "MATCH  past_card_reread: $out" || { echo "DIFFER past_card_reread: $out"; exit 1; }
  out=$(./target/release/cargo-cardiac test $unchecked tests/programs/past_card_reread2.rs)
  [ "$out" = "200,2,201,200,0" ] && echo "MATCH  past_card_reread2: $out" || { echo "DIFFER past_card_reread2: $out"; exit 1; }
  out=$(./target/release/cargo-cardiac test $unchecked tests/programs/past_card_known.rs)
  [ "$out" = "200,0,40,0" ] && echo "MATCH  past_card_known: $out" || { echo "DIFFER past_card_known: $out"; exit 1; }
  out=$(./target/release/cargo-cardiac test $unchecked tests/programs/usize_below.rs)
  [ "$out" = "1,11,-1,-999" ] && echo "MATCH  usize_below: $out" || { echo "DIFFER usize_below: $out"; exit 1; }
done
# The tables-by-tens tip's programs in Rust: its Minesweeper count on a board of 4 rows by 6, four ways
# (minesweeper_*.rs), none of which fits in CARDIAC's memory yet, and its 3D tic-tac-toe, three ways
# (tictactoe_*.rs), of which the layer-to-a-row ones fit (tests/fuzz runs them against native Rust on every move).
# Each must take no more cells than it does now, without overflow checks and with them, at opt-level 2.
for want in minesweeper_edges:146:163 minesweeper_border:147:172 minesweeper_neighbours:127:156 \
    minesweeper_number:143:174 tictactoe_packed:98:125 tictactoe_tens:82:84 tictactoe_number:88:90; do
  n=${want%%:*}; limits=${want#*:}
  for checks in off on; do
    if [ $checks = off ]; then most=${limits%%:*}; else most=${limits#*:}; fi
    said=$(./target/release/cargo-cardiac build -C opt-level=2 -C overflow-checks=$checks tests/programs/$n.rs 2>&1)
    got=$(echo "$said" | sed -n 's/.* \([0-9]*\) cells.*/\1/p' | head -1)
    if [ -n "$got" ] && [ "$got" -le "$most" ]; then echo "MATCH  $n, overflow checks $checks: $got cells, no more than $most"
    else echo "DIFFER $n, overflow checks $checks: $said, not $most cells or fewer"; exit 1; fi
  done
done
# what skip_cards() and add_times() cost: one call each, for n = 0, 1, 9, 10, 37 and the largest n, the
# result and the instructions the whole run takes (the loader's and main's are the same for every n). add_times is
# 24 instructions and one an addition, T for the tens and u for the units; skip_cards 15, one a card and 2 a ten
for f in add_times_count skip_cards_count; do
  ./target/release/cargo-cardiac build -q -C opt-level=2 -C overflow-checks=off tests/programs/$f.rs || exit 1
done
for want in "0:0 147" "1:-9 148" "9:-81 156" "10:-90 148" "37:-333 157" "99:-891 165"; do
  n=${want%%:*}
  got=$("$BURN" tests/programs/add_times_count.cqb -- -9 $n 2>&1 | sed 's/ *halted after \([0-9]*\) instructions/\1/' | tr '\n' ' ')
  [ "$got" = "${want#*:} " ] && echo "MATCH  add_times(-9, $n): $got" || { echo "DIFFER add_times(-9, $n): $got, not ${want#*:}"; exit 1; }
done
for want in "0:90" "1:91" "9:99" "10:102" "37:133" "999:1287"; do
  n=${want%%:*}
  got=$("$BURN" tests/programs/skip_cards_count.cqb -- $n $(cards $n 7) 555 2>&1 | sed 's/ *halted after \([0-9]*\) instructions/\1/' | tr '\n' ' ')
  [ "$got" = "555 ${want#*:} " ] && echo "MATCH  skip_cards($n): $got" || { echo "DIFFER skip_cards($n): $got, not 555 ${want#*:}"; exit 1; }
done
# --emit: build writes what it is asked for. phases.rs in two decks (no # lines) and two six-column listings
# with the start, no assembly; and its decks output what a default build's do
E=$(mktemp -d); D=$(mktemp -d); cp tests/programs/phases.rs $E/; cp tests/programs/phases.rs $D/
./target/release/cargo-cardiac build -C opt-level=2 --emit deck,listing,map $E/phases.rs >/dev/null 2>&1
./target/release/cargo-cardiac build -C opt-level=2 $D/phases.rs >/dev/null 2>&1
if [ -f $E/phases.cqb ] && [ -f $E/phases.2.cqb ] && [ -f $E/phases.cq ] && [ -f $E/phases.2.cq ] &&
  [ ! -f $E/phases_1.asm ] && ! grep -q '#' $E/phases.cqb $E/phases.2.cqb && grep -q '^Start at ' $E/phases.cq &&
  [ -f $D/phases.2.cqb ] && [ -f $D/phases.map ] && [ ! -f $D/phases_1.asm ] && [ ! -f $D/phases.cq ]; then
  echo "MATCH  emit: decks and listings, no assembly; by default decks and map"
else echo "DIFFER emit: $(ls $E | tr '\n' ' ')/ $(ls $D | tr '\n' ' ')"; exit 1; fi
a=$("$BURN" $E/phases.cqb -- 5 3 9 1 7 2 8 4 2>/dev/null | tr '\n' ' ')
b=$("$BURN" $D/phases.cqb -- 5 3 9 1 7 2 8 4 2>/dev/null | tr '\n' ' ')
[ -n "$a" ] && [ "$a" = "$b" ] && echo "MATCH  emit: its decks run as a default build's: $a" ||
  { echo "DIFFER emit: $a, a default build's $b"; exit 1; }
sh ice_check.sh "$MARK" || exit 1
