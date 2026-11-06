//! The bit methods of the integer types, as rustc leaves them, calls to intrinsics: `count_ones` is `ctpop::<T>`,
//! `leading_zeros` `ctlz::<T>` (`ctlz_nonzero` in `next_power_of_two`), `trailing_zeros` `cttz::<T>`, and
//! `rotate_left` and `rotate_right` the intrinsics of their names. An i16's work in u16, which `wide` makes a cell
//! holding the value modulo 2^16; a u8's and a usize's are the value itself, a usize past a card held below 0
//! (`ranges::huge_usizes`). Either way the cell is a number c from -999 to 999 whose bits, as two's complement has
//! them, are the value's low ones, the rest all 0 for c from 0, all 1 below it. The counts they return, `u32`s, are
//! on a card, and `wide` keeps them in a cell (`exact_result`).
//!
//! Each becomes code here, before the passes that follow see it. A count walks the low ten bits of c from the top,
//! held as in `runtime::bitwise`: s, the bits not yet read less 512, so that the next bit is whether s is 0 or
//! more, and the rest move up as s becomes 2(s - 256) after a 1 and 2(s + 256) after a 0. The walk is done when s
//! is -512: every bit left is 0. Below 0, c's bits are those of -1 - c turned over, or for the trailing zeros those
//! of -c. A rotation of a u8 by one is 2x, less 255 past 255; by k, that k times.
use super::ir::{
    BasicBlock, BinOp, Block, CastKind, Const, GenericArg, IntTy, Local, Operand, Path, Place, Rvalue, Segment,
    Statement, Terminator, Ty, Unwind,
};
use super::methods::Gains;
use super::structs::Function;

/// An intrinsic this file writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Bits {
    CountOnes,
    LeadingZeros,
    /// on a value that isn't 0, which needs no test for it
    LeadingZerosNonzero,
    TrailingZeros,
    TrailingZerosNonzero,
    RotateLeft,
    RotateRight,
}

/// The intrinsic a call is to and its type: `ctpop::<u16>`, as rustc prints it where the name is unique, or
/// `core::intrinsics::ctpop::<u16>`. A function of the program's own of that name (`defined`) stays its own.
fn called(func: &Path, defined: &impl Fn(&str) -> bool) -> Option<(Bits, IntTy)> {
    if func.qself.is_some() {
        return None;
    }
    let (Segment::Name { name, args: Some(a) }, before) = func.segments.split_last()? else {
        return None;
    };
    let core = match before {
        [] => !defined(name),
        [Segment::Name { name: c, args: None }, Segment::Name { name: i, args: None }] => {
            (c == "core" || c == "std") && i == "intrinsics"
        }
        _ => false,
    };
    let [GenericArg::Ty(Ty::Int(t @ (IntTy::U8 | IntTy::U16 | IntTy::Usize)))] = a.args.as_slice() else {
        return None;
    };
    let bits = match name.as_str() {
        "ctpop" => Bits::CountOnes,
        "ctlz" => Bits::LeadingZeros,
        "ctlz_nonzero" => Bits::LeadingZerosNonzero,
        "cttz" => Bits::TrailingZeros,
        "cttz_nonzero" => Bits::TrailingZerosNonzero,
        "rotate_left" if *t == IntTy::U8 => Bits::RotateLeft,
        "rotate_right" if *t == IntTy::U8 => Bits::RotateRight,
        _ => return None,
    };
    core.then_some((bits, *t))
}

fn width(t: IntTy) -> i128 {
    match t {
        IntTy::U8 => 8,
        IntTy::U16 => 16,
        _ => 64,
    }
}

/// The bounds of a count's `u32`, 0 to the type's bits: the cells `wide` may hold it in. (A rotation's result is
/// the type's own.)
pub fn exact_result(func: &Path, defined: &impl Fn(&str) -> bool) -> Option<[i128; 2]> {
    match called(func, defined)? {
        (Bits::RotateLeft | Bits::RotateRight, _) => None,
        (_, t) => Some([0, width(t)]),
    }
}

/// Whether a call reads its arguments' bits only, which a u16 held modulo 2^16 has (`wide`).
pub fn reads_bits(func: &Path, defined: &impl Fn(&str) -> bool) -> bool {
    called(func, defined).is_some()
}

fn card(v: i128) -> Operand {
    Operand::Constant(Const::Int {
        value: v,
        ty: IntTy::I16,
    })
}

fn set(p: Local, v: Rvalue) -> Statement {
    Statement::Assign(Place::local(p), v)
}

fn read(l: Local) -> Operand {
    Operand::Copy(Place::local(l))
}

fn op(o: BinOp, a: Operand, b: Operand) -> Rvalue {
    Rvalue::BinaryOp(o, a, b)
}

/// A block whose statements and terminator are set later: a loop's head, which the blocks after it jump back to.
fn later(gains: &mut Gains) -> BasicBlock {
    gains.block(vec![], Terminator::Unreachable)
}

fn fill(gains: &mut Gains, name: BasicBlock, statements: Vec<Statement>, terminator: Terminator) {
    let b: &mut Block = gains
        .blocks
        .iter_mut()
        .find(|b| b.name == name)
        .expect("a block made here");
    b.statements = statements;
    b.terminator = terminator;
}

/// The walk over the bits of a number from 0 to 999 in s, which starts as it less 512: a 1 adds 1 to `ones`
/// where there is one, and every bit read takes 1 from `left` where there is one. It goes on to `done` once s is
/// -512. s, and the block the walk starts at.
fn walk(gains: &mut Gains, ones: Option<Local>, left: Option<Local>, done: BasicBlock) -> (Local, BasicBlock) {
    let s = gains.local(Ty::Int(IntTy::I16));
    let head = later(gains);
    let mut moved = vec![set(s, op(BinOp::Add, read(s), read(s)))];
    if let Some(l) = left {
        moved.push(set(l, op(BinOp::Sub, read(l), card(1))));
    }
    let (st, t) = gains.test(BinOp::Lt, &read(s), &card(-511), done, head);
    moved.push(st);
    let next = gains.block(moved, t);
    let mut one = vec![set(s, op(BinOp::Sub, read(s), card(256)))];
    if let Some(n) = ones {
        one.push(set(n, op(BinOp::Add, read(n), card(1))));
    }
    let one = gains.block(one, Terminator::Goto(next));
    let zero = gains.block(vec![set(s, op(BinOp::Add, read(s), card(256)))], Terminator::Goto(next));
    let (st, t) = gains.test(BinOp::Lt, &read(s), &card(0), zero, one);
    fill(gains, head, vec![st], t);
    (s, head)
}

/// The code of `dest = intrinsic(args)`, in place of the call: the statements for the call's own block and its new
/// terminator.
fn expand(
    gains: &mut Gains,
    bits: Bits,
    ty: IntTy,
    dest: &Place,
    args: &[Operand],
    back: BasicBlock,
) -> Option<(Vec<Statement>, Terminator)> {
    let w = width(ty);
    let i16 = Ty::Int(IntTy::I16);
    let to_dest = |v: Rvalue| Statement::Assign(dest.clone(), v);
    // a u8 is never below 0
    let signed = ty != IntTy::U8;
    let x = args.first()?;
    let c = gains.local(i16.clone());
    let cast = set(c, Rvalue::Cast(x.clone(), i16.clone(), CastKind::IntToInt));
    Some(match (bits, args) {
        // the ones of c, or below 0 the bits less the ones of -1 - c
        (Bits::CountOnes, [_]) => {
            let n = gains.local(i16.clone());
            let plain = gains.set(dest, Rvalue::Use(read(n)), back);
            let done = if signed {
                let less = gains.set(dest, op(BinOp::Sub, card(w), read(n)), back);
                let (st, t) = gains.test(BinOp::Lt, &read(c), &card(0), less, plain);
                gains.block(vec![st], t)
            } else {
                plain
            };
            let (s, head) = walk(gains, Some(n), None, done);
            let from_c = vec![set(n, Rvalue::Use(card(0))), set(s, op(BinOp::Sub, read(c), card(512)))];
            if !signed {
                return Some((vec![cast].into_iter().chain(from_c).collect(), Terminator::Goto(head)));
            }
            let turned = gains.local(i16.clone());
            let from_turned = vec![
                set(n, Rvalue::Use(card(0))),
                set(turned, op(BinOp::Sub, card(-1), read(c))),
                set(s, op(BinOp::Sub, read(turned), card(512))),
            ];
            let from_c = gains.block(from_c, Terminator::Goto(head));
            let from_turned = gains.block(from_turned, Terminator::Goto(head));
            let (st, t) = gains.test(BinOp::Lt, &read(c), &card(0), from_turned, from_c);
            (vec![cast, st], t)
        }
        // 0 below 0 (the top bit is 1), all the bits at 0; else the bits less 10, and one more for each 0 read before
        // the first 1
        (Bits::LeadingZeros | Bits::LeadingZerosNonzero, [_]) => {
            let (lz, s) = (gains.local(i16.clone()), gains.local(i16.clone()));
            let head = later(gains);
            let found = gains.set(dest, Rvalue::Use(read(lz)), back);
            let zero = gains.block(
                vec![
                    set(s, op(BinOp::Add, read(s), card(256))),
                    set(s, op(BinOp::Add, read(s), read(s))),
                    set(lz, op(BinOp::Add, read(lz), card(1))),
                ],
                Terminator::Goto(head),
            );
            let (st, t) = gains.test(BinOp::Lt, &read(s), &card(0), zero, found);
            fill(gains, head, vec![st], t);
            let start = vec![
                set(lz, Rvalue::Use(card(w - 10))),
                set(s, op(BinOp::Sub, read(c), card(512))),
            ];
            let mut rest = gains.block(start, Terminator::Goto(head));
            if bits == Bits::LeadingZeros {
                let all = gains.set(dest, Rvalue::Use(card(w)), back);
                let (st, t) = gains.test(BinOp::Eq, &read(c), &card(0), all, rest);
                rest = gains.block(vec![st], t);
            }
            if signed {
                let none = gains.set(dest, Rvalue::Use(card(0)), back);
                let (st, t) = gains.test(BinOp::Lt, &read(c), &card(0), none, rest);
                rest = gains.block(vec![st], t);
            }
            (vec![cast], Terminator::Goto(rest))
        }
        // all the bits at 0; else 10 less the bits read up to the last 1, of c or -c
        (Bits::TrailingZeros | Bits::TrailingZerosNonzero, [_]) => {
            let tz = gains.local(i16.clone());
            let done = gains.set(dest, Rvalue::Use(read(tz)), back);
            let (s, head) = walk(gains, None, Some(tz), done);
            let from_c = vec![
                set(tz, Rvalue::Use(card(10))),
                set(s, op(BinOp::Sub, read(c), card(512))),
            ];
            let mut rest = gains.block(from_c, Terminator::Goto(head));
            if signed {
                let minus = gains.local(i16.clone());
                let from_minus = vec![
                    set(tz, Rvalue::Use(card(10))),
                    set(minus, op(BinOp::Sub, card(0), read(c))),
                    set(s, op(BinOp::Sub, read(minus), card(512))),
                ];
                let from_minus = gains.block(from_minus, Terminator::Goto(head));
                let (st, t) = gains.test(BinOp::Lt, &read(c), &card(0), from_minus, rest);
                rest = gains.block(vec![st], t);
            }
            if bits == Bits::TrailingZeros {
                let all = gains.set(dest, Rvalue::Use(card(w)), back);
                let (st, t) = gains.test(BinOp::Eq, &read(c), &card(0), all, rest);
                rest = gains.block(vec![st], t);
            }
            (vec![cast], Terminator::Goto(rest))
        }
        // a u8 by a constant: k steps left (8 - k for right), each 2v, less 255 past 255, in an i16
        (Bits::RotateLeft | Bits::RotateRight, [_, k]) => {
            let (k, _) = k.int()?;
            let k = if bits == Bits::RotateLeft {
                k
            } else {
                8 - k.rem_euclid(8)
            }
            .rem_euclid(8);
            if k == 0 {
                return Some((vec![to_dest(Rvalue::Use(x.clone()))], Terminator::Goto(back)));
            }
            let u8 = Ty::Int(IntTy::U8);
            let out = gains.set(dest, Rvalue::Cast(read(c), u8, CastKind::IntToInt), back);
            let i = gains.local(i16.clone());
            let head = later(gains);
            let (st, t) = gains.test(BinOp::Gt, &read(i), &card(0), head, out);
            let next = gains.block(vec![set(i, op(BinOp::Sub, read(i), card(1))), st], t);
            let past = gains.block(vec![set(c, op(BinOp::Sub, read(c), card(255)))], Terminator::Goto(next));
            let (st, t) = gains.test(BinOp::Gt, &read(c), &card(255), past, next);
            fill(gains, head, vec![set(c, op(BinOp::Add, read(c), read(c))), st], t);
            (vec![cast, set(i, Rvalue::Use(card(k)))], Terminator::Goto(head))
        }
        _ => return None,
    })
}

/// The unsigned type an operand is all the ones of: `255_u8`, `usize::MAX`.
fn all_ones(o: &Operand) -> Option<IntTy> {
    match o {
        Operand::Constant(Const::Int {
            value: 255,
            ty: IntTy::U8,
        }) => Some(IntTy::U8),
        Operand::Constant(Const::Path(p)) => match p.segments.as_slice() {
            [Segment::Name { name: t, args: None }, Segment::Name { name: m, args: None }]
                if t == "usize" && m == "MAX" =>
            {
                Some(IntTy::Usize)
            }
            _ => None,
        },
        _ => None,
    }
}

/// `dest = T::MAX >> n` by a variable, which `next_power_of_two` makes of the leading zeros: 2^(bits - n) - 1, with
/// n taken modulo the bits, as 0 doubled and 1 added that many times. For a usize each doubling is checked, as
/// from 1023 on the result is past a card, and it panics; but `usize::MAX >> 0` is usize::MAX. The statements for
/// the block it is in, up to it, and their terminator; the rest of the block goes on in `back`.
fn ones_shifted(
    gains: &mut Gains,
    dest: &Place,
    ty: IntTy,
    n: &Operand,
    back: BasicBlock,
) -> (Vec<Statement>, Terminator) {
    let i16 = Ty::Int(IntTy::I16);
    let t = Ty::Int(ty);
    let w = width(ty);
    let (m, q) = (gains.local(i16.clone()), gains.local(t.clone()));
    let in_type = |v: i128| Operand::Constant(Const::Int { value: v, ty });
    let out = gains.set(dest, Rvalue::Use(read(q)), back);
    let head = later(gains);
    let more = vec![
        set(q, op(BinOp::Add, read(q), in_type(1))),
        set(m, op(BinOp::Sub, read(m), card(1))),
    ];
    let double = if ty == IntTy::U8 {
        // 254 at most
        let mut s = vec![set(q, op(BinOp::Add, read(q), read(q)))];
        s.extend(more);
        gains.block(s, Terminator::Goto(head))
    } else {
        let pair = gains.local(Ty::Tuple(vec![t.clone(), Ty::Bool]));
        let mut s = vec![set(q, Rvalue::Use(Operand::Copy(Place::field(pair, 0, t.clone()))))];
        s.extend(more);
        let take = gains.block(s, Terminator::Goto(head));
        let check = Terminator::Assert {
            cond: Operand::Move(Place::field(pair, 1, Ty::Bool)),
            expected: false,
            msg: super::runtime::NO_CARD.into(),
            msg_args: vec![],
            target: take,
            unwind: Unwind::Continue,
        };
        gains.block(vec![set(pair, op(BinOp::AddWithOverflow, read(q), read(q)))], check)
    };
    let (st, t) = gains.test(BinOp::Gt, &read(m), &card(0), double, out);
    fill(gains, head, vec![st], t);
    let masked = gains.local(i16.clone());
    let mask = set(masked, op(BinOp::BitAnd, n.clone(), card(w - 1)));
    let start = vec![
        set(m, op(BinOp::Sub, card(w), read(masked))),
        set(q, Rvalue::Use(in_type(0))),
    ];
    if ty == IntTy::U8 {
        return (vec![mask].into_iter().chain(start).collect(), Terminator::Goto(head));
    }
    // a usize shifted by 0 is usize::MAX itself, held as -1 (`ranges::huge_usizes`)
    let shifted = gains.block(start, Terminator::Goto(head));
    let minus = gains.local(i16.clone());
    let max = gains.block(
        vec![
            set(minus, Rvalue::Use(card(-1))),
            Statement::Assign(dest.clone(), Rvalue::Cast(read(minus), Ty::Int(ty), CastKind::IntToInt)),
        ],
        Terminator::Goto(back),
    );
    let (st, test) = gains.test(BinOp::Eq, &read(masked), &card(0), max, shifted);
    (vec![mask, st], test)
}

/// Every call in `f` to one of the intrinsics becomes the code for it, and every `T::MAX >> n` by a variable.
pub fn inline_bit_intrinsics(f: &mut Function, defined: impl Fn(&str) -> bool) {
    let mut gains = Gains::new(&f.params, &f.body.blocks, &mut f.body.decls);
    for b in f.body.blocks.iter_mut() {
        let Terminator::Call {
            dest,
            func,
            args,
            target: Some(back),
            unwind: Unwind::Continue | Unwind::Unreachable,
        } = &b.terminator
        else {
            continue;
        };
        let Some((bits, ty)) = called(func, &defined) else {
            continue;
        };
        // each argument is read more than once now: a copy, not a move
        let args: Vec<Operand> = args
            .iter()
            .map(|a| match a {
                Operand::Move(p) => Operand::Copy(p.clone()),
                a => a.clone(),
            })
            .collect();
        let (dest, back) = (dest.clone(), *back);
        if let Some((statements, t)) = expand(&mut gains, bits, ty, &dest, &args, back) {
            b.statements.extend(statements);
            b.terminator = t;
        }
    }
    // each `T::MAX >> n` splits its block: the rest goes on in a block of its own
    let blocks = &mut f.body.blocks;
    let mut bi = 0;
    while bi < blocks.len() {
        let found = blocks[bi].statements.iter().position(|s| match s {
            Statement::Assign(_, Rvalue::BinaryOp(BinOp::Shr, k, n)) => all_ones(k).is_some() && n.int().is_none(),
            _ => false,
        });
        let Some(i) = found else {
            bi += 1;
            continue;
        };
        let b = &mut blocks[bi];
        let rest = b.statements.split_off(i + 1);
        let Some(Statement::Assign(dest, Rvalue::BinaryOp(_, k, n))) = b.statements.pop() else {
            unreachable!("a shift")
        };
        let end = std::mem::replace(&mut b.terminator, Terminator::Unreachable);
        let back = gains.block(rest, end);
        let ty = all_ones(&k).expect("all ones");
        let (statements, t) = ones_shifted(&mut gains, &dest, ty, &n, back);
        b.statements.extend(statements);
        b.terminator = t;
        blocks.append(&mut gains.blocks);
    }
    blocks.append(&mut gains.blocks);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::ir;

    fn callee(path: &str) -> Path {
        let line = format!("_3 = {path}(copy _2) -> [return: bb1, unwind continue];");
        match ir::parse_line(&line) {
            Ok(ir::Line::Terminator(Terminator::Call { func, .. })) => func,
            other => panic!("not a call: {other:?}"),
        }
    }

    #[test]
    fn the_intrinsics_are_found_by_their_paths_bare_or_whole() {
        let none = |_: &str| false;
        assert_eq!(
            called(&callee("ctpop::<u16>"), &none),
            Some((Bits::CountOnes, IntTy::U16))
        );
        assert_eq!(
            called(&callee("core::intrinsics::ctlz_nonzero::<usize>"), &none),
            Some((Bits::LeadingZerosNonzero, IntTy::Usize))
        );
        assert_eq!(
            called(&callee("rotate_left::<u8>"), &none),
            Some((Bits::RotateLeft, IntTy::U8))
        );
        // not a rotation in 16 bits, nor a function of the program's own of the name
        assert_eq!(called(&callee("rotate_left::<u16>"), &none), None);
        assert_eq!(called(&callee("ctpop::<u16>"), &|name: &str| name == "ctpop"), None);
        assert_eq!(exact_result(&callee("cttz::<u8>"), &none), Some([0, 8]));
        assert_eq!(exact_result(&callee("ctpop::<u16>"), &none), Some([0, 16]));
    }
}
