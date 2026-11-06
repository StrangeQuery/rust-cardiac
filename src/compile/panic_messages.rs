//! Panics with a literal message: `panic!("negative card")`, `assert!(ok, "...")`, `unreachable!("...")`,
//! `unimplemented!("...")`, and `assert_eq!` and `assert_ne!` with or without one. rustc builds a
//! `core::fmt::Arguments` for the message and calls `panic_fmt` with it, or for `assert_eq!` calls
//! `core::panicking::assert_failed` with references to both values. CARDIAC has no text, so each becomes the call a
//! bare `assert!` makes, `core::panicking::panic(const "message")`, and the message goes in the panic map, as
//! `expect()`'s does. `assert_eq!`'s is Rust's first line, ``assertion `left == right` failed``, without the values
//! (the map is written before the program runs). A message with `{}` arguments keeps its literal text and leaves
//! out the values, for the same reason: `panic!("negative card: {}", a)` says `negative card: `.
//!
//! ```text
//!     _9 = Arguments::<'_>::from_str(const "negative card") -> [return: bb10, unwind continue];   (opt-level 1)
//! bb10: _8 = panic_fmt(move _9) -> unwind continue;
//! ```
//!
//! and at opt-level 2 the same `Arguments` built in the panic's own block: its template the string's address, and
//! its `args` the string's length, shifted, with the low bit set (what marks a template that is only a string).
//! A message with arguments is built there too, at opt-level 1 and 2 alike: its template a byte string that
//! interleaves the literal pieces with placeholders (`b"\x0fnegative card: \xc0\x00"`, core's `fmt::Arguments`
//! says how), and its `args` a borrow of an array of `fmt::rt::Argument`s, each a pointer to a value and the
//! function that formats it. Nothing a panicking block does can be seen once the program has panicked, so it
//! keeps no statements, and what was there only for the message (the borrows of `assert_eq!`'s values, the
//! arguments) goes.
use std::collections::HashSet;

use super::ir::{
    BasicBlock, BinOp, Block, BorrowKind, CastKind, Const, File, Item, Local, Locals, Operand, Path, Place, Projection,
    Rvalue, Segment, Statement, Terminator, Ty,
};

/// The names of a path's segments, generic arguments left out: `["core", "panicking", "assert_failed"]`.
fn names(p: &Path) -> Option<Vec<&str>> {
    if p.qself.is_some() {
        return None;
    }
    p.segments
        .iter()
        .map(|s| match s {
            Segment::Name { name, .. } => Some(name.as_str()),
            Segment::Impl(_) => None,
        })
        .collect()
}

/// Is it `core::...::name`, or the end of that path, as the MIR may print it trimmed (`panic_fmt`)?
fn calls(p: &Path, full: &[&str]) -> bool {
    names(p).is_some_and(|n| !n.is_empty() && full.ends_with(&n))
}

/// The value of `l` the block's statements leave: the last whole assignment to it.
fn def_in(statements: &[Statement], l: Local) -> Option<&Rvalue> {
    statements.iter().rev().find_map(|s| match s {
        Statement::Assign(p, rv) if p.as_local() == Some(l) => Some(rv),
        _ => None,
    })
}

/// The local an operand reads whole.
fn whole(o: &Operand) -> Option<Local> {
    match o {
        Operand::Copy(p) | Operand::Move(p) => p.as_local(),
        _ => None,
    }
}

/// The string a pointer was made from, through its casts and borrows: `_13 = copy _14 as NonNull<u8> (Transmute)`,
/// `_14 = copy _19 as *const u8 (PtrToPtr)`, `_19 = &raw const (*_12)`, `_12 = const "negative card"`.
fn string_of(statements: &[Statement], o: &Operand) -> Option<String> {
    if let Operand::Constant(Const::Str(s)) = o {
        return Some(s.clone());
    }
    match def_in(statements, whole(o)?)? {
        Rvalue::Use(o) | Rvalue::Cast(o, _, CastKind::PtrToPtr | CastKind::Transmute) => string_of(statements, o),
        Rvalue::Ref(BorrowKind::Shared | BorrowKind::RawConst, p) if p.projection == [Projection::Deref] => {
            string_of(statements, &Operand::Copy(Place::local(p.base_local()?)))
        }
        _ => None,
    }
}

/// Is it `Arguments`' `args` for a template that is only a string: `(len << 1) | 1`, cast to a pointer?
fn string_marked(statements: &[Statement], o: &Operand) -> bool {
    let one = |o: &Operand| o.int().is_some_and(|(v, _)| v == 1);
    let rv = whole(o).and_then(|l| def_in(statements, l));
    match rv {
        Some(Rvalue::Use(o) | Rvalue::Cast(o, _, CastKind::Transmute)) => string_marked(statements, o),
        Some(Rvalue::BinaryOp(BinOp::BitOr, shifted, bit)) if one(bit) => {
            let len = whole(shifted).and_then(|l| def_in(statements, l));
            matches!(len, Some(Rvalue::BinaryOp(BinOp::Shl, _, by)) if one(by))
        }
        _ => false,
    }
}

/// The operand a value was made from, through copies and transmutes: `_22 = copy _8 as NonNull<Argument> (Transmute)`
/// is `_8`.
fn source<'a>(statements: &'a [Statement], o: &'a Operand) -> &'a Operand {
    match whole(o).and_then(|l| def_in(statements, l)) {
        Some(Rvalue::Use(from) | Rvalue::Cast(from, _, CastKind::Transmute)) => source(statements, from),
        _ => o,
    }
}

/// The literal text of a template with placeholders, the placeholders left out, given how many arguments there
/// are: core's encoding (`core::fmt::Arguments`), which must be read to its end exactly. A piece is its length
/// in a byte below 0x80 and its bytes, or 0x80, a 16-bit length and its bytes; a placeholder is a byte with the top
/// two bits set, and after it, as its low bits say, 32 bits of flags, a 16-bit width, a 16-bit precision and the
/// 16-bit index of its argument (the next one when there is none); a 0 byte is the end. A width or precision taken
/// from an argument (bits 4 and 5) is refused, as anything else is.
fn template_text(bytes: &[u8], args: usize) -> Option<String> {
    let u16_at = |i: usize| Some(u16::from_le_bytes(bytes.get(i..i + 2)?.try_into().ok()?) as usize);
    let (mut text, mut i, mut next) = (vec![], 0, 0);
    loop {
        let n = *bytes.get(i)?;
        i += 1;
        match n {
            0 => break,
            1..=0x7f => {
                text.extend_from_slice(bytes.get(i..i + n as usize)?);
                i += n as usize;
            }
            0x80 => {
                let len = u16_at(i)?;
                text.extend_from_slice(bytes.get(i + 2..i + 2 + len)?);
                i += 2 + len;
            }
            0xc0.. if n & 0x30 == 0 => {
                i += 4 * (n & 1) as usize + 2 * (n >> 1 & 1) as usize + 2 * (n >> 2 & 1) as usize;
                let mut arg = next;
                if n & 8 != 0 {
                    arg = u16_at(i)?;
                    i += 2;
                }
                if arg >= args {
                    return None;
                }
                next = arg + 1;
            }
            _ => return None,
        }
    }
    // as rustc writes a string, which is how the IR keeps one; a message of values alone (`panic!("{}", a)`) has no
    // text for the map to give
    let text = String::from_utf8(text)
        .ok()
        .filter(|t| i == bytes.len() && !t.trim().is_empty())?;
    let quoted = format!("{text:?}");
    Some(quoted[1..quoted.len() - 1].to_string())
}

/// How many arguments `args` holds, an array of `fmt::rt::Argument`s built in the block: each one a placeholder
/// whose value is a pointer to an integer, a bool, a char or a str (or a reference to one), so that what formats
/// it is core's, which does nothing the program could see. A value of the program's own type is formatted by its
/// own `Display` or `Debug`, which could, and is refused.
fn arguments(statements: &[Statement], types: &[(Local, Ty)], args: &Operand) -> Option<usize> {
    let Some(Rvalue::Ref(BorrowKind::Shared, array)) =
        whole(source(statements, args)).and_then(|l| def_in(statements, l))
    else {
        return None;
    };
    let Rvalue::Array(elements) = def_in(statements, array.as_local()?)? else {
        return None;
    };
    let core_formats = |ty: &Ty| {
        let mut ty = ty;
        while let Ty::Ref { to, .. } = ty {
            ty = to;
        }
        matches!(ty, Ty::Int(_) | Ty::Bool | Ty::Char | Ty::Str)
    };
    let placeholder = |o: &Operand| -> Option<()> {
        let Rvalue::Struct(argument, fields) = def_in(statements, whole(o)?)? else {
            return None;
        };
        let [(ty, kind)] = fields.as_slice() else { return None };
        let Rvalue::Struct(variant, fields) = def_in(statements, whole(kind)?)? else {
            return None;
        };
        let value = fields.iter().find(|(f, _)| f == "value").map(|(_, o)| o)?;
        let Rvalue::Ref(BorrowKind::RawConst, pointee) = def_in(statements, whole(source(statements, value))?)? else {
            return None;
        };
        if argument.name() != "Argument" || ty != "ty" || variant.name() != "Placeholder" {
            return None;
        }
        // `&raw const _1`, or `&raw const (*_5)` of a reference
        let declared = types
            .iter()
            .find(|(l, _)| Some(*l) == pointee.base_local())
            .map(|(_, t)| t);
        let pointee = match (pointee.projection.as_slice(), declared?) {
            ([], t) => t,
            ([Projection::Deref], Ty::Ref { to, .. }) => to,
            _ => return None,
        };
        core_formats(pointee).then_some(())
    };
    elements.iter().try_for_each(placeholder)?;
    Some(elements.len())
}

/// A message `panic_fmt` is given, as the literal it was written as: the `Arguments` built in the panic's block
/// from a string or from a template and its arguments, or made by `Arguments::from_str` in the block that comes
/// only here (that block, to go straight on, as its second value).
fn literal(blocks: &[Block], types: &[(Local, Ty)], at: usize, o: &Operand) -> Option<(String, Option<usize>)> {
    let l = whole(o)?;
    let statements = &blocks[at].statements;
    if let Some(Rvalue::Struct(p, fields)) = def_in(statements, l) {
        let field = |name: &str| fields.iter().find(|(f, _)| f == name).map(|(_, o)| o);
        let (template, args) = (field("template")?, field("args")?);
        if p.name() != "Arguments" || fields.len() != 2 {
            return None;
        }
        if string_marked(statements, args) {
            return Some((string_of(statements, template)?, None));
        }
        let Operand::Constant(Const::Bytes(bytes)) = source(statements, template) else {
            return None;
        };
        return Some((template_text(bytes, arguments(statements, types, args)?)?, None));
    }
    // made once, by a call whose only way on is the panic's block
    let assigned = |b: &Block| b.statements.iter().any(|s| s.def() == Some(l));
    let calls_into: Vec<usize> = (0..blocks.len())
        .filter(|&i| blocks[i].terminator.def() == Some(l))
        .collect();
    let ([from], false) = (calls_into.as_slice(), blocks.iter().any(|b| assigned(b))) else {
        return None;
    };
    let Terminator::Call { func, args, target, .. } = &blocks[*from].terminator else {
        return None;
    };
    let from_str = calls(func, &["core", "fmt", "Arguments", "from_str"])
        || calls(func, &["core", "fmt", "Arguments", "from_str_nonconst"]);
    match (from_str, args.as_slice(), target) {
        (true, [Operand::Constant(Const::Str(s))], Some(t)) if *t == blocks[at].name => Some((s.clone(), Some(*from))),
        _ => None,
    }
}

/// What Rust says when `assert_failed` is called: `assertion `left == right` failed`, and the message after a
/// colon where there is one (`assert_eq!(a, b, "the cards differ")`). Its fourth argument is `None`, or
/// `Some(arguments)` built in the panic's block.
fn assert_message(
    blocks: &[Block],
    types: &[(Local, Ty)],
    at: usize,
    args: &[Operand],
) -> Option<(String, Option<usize>)> {
    let [Operand::Constant(Const::Path(kind)), _, _, message] = args else {
        return None;
    };
    let names = names(kind)?;
    let op = match names.as_slice() {
        [.., "AssertKind", "Eq"] => "==",
        [.., "AssertKind", "Ne"] => "!=",
        _ => return None,
    };
    let head = format!("assertion `left {op} right` failed");
    match message {
        Operand::Constant(Const::Path(none)) if none.name() == "None" => Some((head, None)),
        _ => match def_in(&blocks[at].statements, whole(message)?)? {
            Rvalue::Variant(some, inner) if some.name() == "Some" && inner.len() == 1 => {
                let (text, from) = literal(blocks, types, at, &inner[0])?;
                Some((format!("{head}: {text}"), from))
            }
            _ => None,
        },
    }
}

/// Each panic with a literal message, in every function, as a call to `core::panicking::panic` with the message.
pub fn literal_panics(file: &mut File) {
    for item in &mut file.items {
        let Item::Fn(f) = item else { continue };
        // each local's type, a parameter's or a declared one's, for what a message's arguments point to
        let types: Vec<(Local, Ty)> = f
            .params
            .iter()
            .cloned()
            .chain(f.body.decls.iter().map(|d| (d.local, d.ty.clone())))
            .collect();
        let blocks = &mut f.body.blocks;
        // the locals the lines taken out named: those nothing else reads now go too
        let mut freed: HashSet<Local> = HashSet::new();
        for at in 0..blocks.len() {
            let Terminator::Call { func, args, .. } = &blocks[at].terminator else {
                continue;
            };
            let found = if calls(func, &["core", "panicking", "panic_fmt"]) && args.len() == 1 {
                literal(blocks, &types, at, &args[0])
            } else if calls(func, &["core", "panicking", "assert_failed"]) {
                assert_message(blocks, &types, at, args)
            } else {
                None
            };
            let Some((message, from)) = found else { continue };
            let b = &mut blocks[at];
            for line in b.lines() {
                freed.extend(line.locals());
            }
            b.statements.clear();
            let Terminator::Call { func, args, .. } = &mut b.terminator else {
                unreachable!()
            };
            *func = Path {
                qself: None,
                segments: ["core", "panicking", "panic"]
                    .iter()
                    .map(|w| Segment::Name {
                        name: w.to_string(),
                        args: None,
                    })
                    .collect(),
            };
            *args = vec![Operand::Constant(Const::Str(message))];
            if let Some(from) = from {
                let to: BasicBlock = blocks[at].name;
                freed.extend(blocks[from].terminator.locals());
                blocks[from].terminator = Terminator::Goto(to);
            }
        }
        if freed.is_empty() {
            continue;
        }
        // what only fed the message: a local that nothing names now but its own assignments (and the locals
        // those assignments read, in turn)
        let mut work: Vec<Local> = freed.into_iter().collect();
        let mut gone: HashSet<Local> = HashSet::new();
        while let Some(l) = work.pop() {
            if gone.contains(&l) || f.params.iter().any(|(p, _)| *p == l) || l == Local(0) || l == Local::UNIT {
                continue;
            }
            let own = |s: &Statement| match s {
                Statement::Assign(p, _) => p.as_local() == Some(l),
                Statement::StorageLive(x) | Statement::StorageDead(x) => *x == l,
                _ => false,
            };
            let named = f
                .body
                .blocks
                .iter()
                .any(|b| b.statements.iter().any(|s| !own(s) && s.mentions(l)) || b.terminator.mentions(l));
            if named {
                continue;
            }
            gone.insert(l);
            for b in f.body.blocks.iter_mut() {
                b.statements.retain(|s| {
                    if !own(s) {
                        return true;
                    }
                    work.extend(s.locals());
                    false
                });
            }
        }
        f.body.decls.retain(|d| !gone.contains(&d.local));
        f.body.inlined.retain(|(l, _)| !gone.contains(l));
        f.body
            .debug
            .retain(|d| !Local::parse(&d.value).is_some_and(|l| gone.contains(&l)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::ir;

    fn compiled(body: &str) -> (Vec<String>, Vec<String>) {
        let mut file = ir::parse_file(&format!("fn main() -> () {{\n    let mut _0: ();\n{body}\n}}\n"));
        literal_panics(&mut file);
        let Item::Fn(f) = &file.items[0] else { panic!("no fn") };
        let lines = f
            .body
            .blocks
            .iter()
            .flat_map(|b| {
                b.statements
                    .iter()
                    .map(|s| s.to_string())
                    .chain([b.terminator.to_string()])
            })
            .collect();
        (lines, f.body.decls.iter().map(|d| d.local.to_string()).collect())
    }

    #[test]
    fn a_message_built_in_the_panics_block_is_the_panics_string() {
        let (lines, decls) = compiled(
            r#"    let _3: !;
    let mut _4: core::fmt::Arguments<'_>;
    let mut _12: &str;
    let mut _13: core::ptr::NonNull<u8>;
    let mut _14: *const u8;
    let mut _15: core::ptr::NonNull<core::fmt::rt::Argument<'_>>;
    let mut _16: usize;
    let mut _17: usize;
    let mut _19: *const str;
    bb0: {
        _12 = const "negative card";
        _19 = &raw const (*_12);
        _14 = copy _19 as *const u8 (PtrToPtr);
        _13 = copy _14 as core::ptr::NonNull<u8> (Transmute);
        _17 = Shl(const 13_usize, const 1_i32);
        _16 = BitOr(move _17, const 1_usize);
        _15 = move _16 as core::ptr::NonNull<core::fmt::rt::Argument<'_>> (Transmute);
        _4 = Arguments::<'_> { template: move _13, args: move _15 };
        _3 = panic_fmt(move _4) -> unwind continue;
    }"#,
        );
        assert_eq!(
            lines,
            [r#"_3 = core::panicking::panic(const "negative card") -> unwind continue;"#]
        );
        assert_eq!(decls, ["_0", "_3"]);
    }

    #[test]
    fn a_message_from_str_makes_goes_straight_on_to_the_panic() {
        let (lines, _) = compiled(
            r#"    let _8: !;
    let mut _9: core::fmt::Arguments<'_>;
    bb0: {
        _9 = Arguments::<'_>::from_str_nonconst(const "not implemented: negative cards") -> [return: bb1, unwind continue];
    }
    bb1: {
        _8 = panic_fmt(move _9) -> unwind continue;
    }"#,
        );
        assert_eq!(
            lines,
            [
                "goto -> bb1;",
                r#"_8 = core::panicking::panic(const "not implemented: negative cards") -> unwind continue;"#
            ]
        );
    }

    #[test]
    fn assert_eq_says_rusts_first_line_and_its_values_borrows_go() {
        let body = r#"    let mut _8: &i16;
    let mut _9: &i16;
    let mut _10: bool;
    let _11: !;
    let mut _12: core::option::Option<core::fmt::Arguments<'_>>;
    let mut _13: core::fmt::Arguments<'_>;
    let _6: i16;
    let _7: i16;
    bb0: {
        _8 = &_6;
        _9 = &_7;
        _10 = Eq(copy _6, copy _7);
        switchInt(move _10) -> [0: bb1, otherwise: bb3];
    }
    bb1: {
        _13 = Arguments::<'_>::from_str(const "the cards differ") -> [return: bb2, unwind continue];
    }
    bb2: {
        _12 = Option::<Arguments<'_>>::Some(move _13);
        _11 = core::panicking::assert_failed::<i16, i16>(const core::panicking::AssertKind::Eq, move _8, move _9, move _12) -> unwind continue;
    }
    bb3: {
        return;
    }"#;
        let (lines, decls) = compiled(body);
        assert_eq!(
            lines[..3],
            [
                "_10 = Eq(copy _6, copy _7);",
                "switchInt(move _10) -> [0: bb1, otherwise: bb3];",
                "goto -> bb2;"
            ]
        );
        assert_eq!(
            lines[3],
            r#"_11 = core::panicking::panic(const "assertion `left == right` failed: the cards differ") -> unwind continue;"#
        );
        assert_eq!(decls, ["_0", "_10", "_11", "_6", "_7"]);
        // without a message, and assert_ne!
        let bare = body
            .replace(
                "AssertKind::Eq, move _8, move _9, move _12",
                "AssertKind::Ne, move _8, move _9, const Option::<Arguments<'_>>::None",
            )
            .replace(
                r#"_13 = Arguments::<'_>::from_str(const "the cards differ") -> [return: bb2, unwind continue];"#,
                "goto -> bb2;",
            );
        let (lines, _) = compiled(&bare);
        assert_eq!(
            lines[3],
            r#"_11 = core::panicking::panic(const "assertion `left != right` failed") -> unwind continue;"#
        );
    }

    #[test]
    fn a_formatted_message_is_left_as_it_is() {
        let body = r#"    let _8: !;
    let mut _9: core::fmt::Arguments<'_>;
    let mut _13: &[core::fmt::rt::Argument<'_>; 1];
    bb0: {
        _9 = Arguments::<'_>::new::<18, 1>(const "negative card: ", move _13) -> [return: bb1, unwind continue];
    }
    bb1: {
        _8 = panic_fmt(move _9) -> unwind continue;
    }"#;
        let (lines, decls) = compiled(body);
        assert_eq!(lines[1], "_8 = panic_fmt(move _9) -> unwind continue;");
        assert_eq!(decls.len(), 4);
    }

    #[test]
    fn a_formatted_message_keeps_its_literal_text() {
        let body = r#"    let _1: i16;
    let _3: !;
    let mut _4: core::fmt::Arguments<'_>;
    let mut _5: &i16;
    let mut _7: core::fmt::rt::Argument<'_>;
    let _8: &[core::fmt::rt::Argument<'_>; 1];
    let _6: [core::fmt::rt::Argument<'_>; 1];
    let mut _16: core::fmt::rt::ArgumentType<'_>;
    let mut _17: core::ptr::NonNull<()>;
    let mut _20: *const i16;
    let mut _21: core::ptr::NonNull<u8>;
    let mut _22: core::ptr::NonNull<core::fmt::rt::Argument<'_>>;
    bb0: {
        _5 = &_1;
        _20 = &raw const (*_5);
        _17 = copy _20 as core::ptr::NonNull<()> (Transmute);
        _19 = <i16 as core::fmt::Display>::fmt as for<'a, 'b, 'c> fn(&'a i16, &'b mut core::fmt::Formatter<'c>) -> core::result::Result<(), core::fmt::Error> (PointerCoercion(ReifyFnPointer(Safe), Implicit));
        _18 = copy _19 as for<'a, 'b> unsafe fn(core::ptr::NonNull<()>, &'a mut core::fmt::Formatter<'b>) -> core::result::Result<(), core::fmt::Error> (Transmute);
        _16 = fmt::rt::ArgumentType::<'_>::Placeholder { value: move _17, formatter: copy _18, _lifetime: const PhantomData::<&()> };
        _7 = fmt::rt::Argument::<'_> { ty: move _16 };
        _6 = [move _7];
        _8 = &_6;
        _21 = const b"\x0fnegative card: \xc0\x00" as core::ptr::NonNull<u8> (Transmute);
        _22 = copy _8 as core::ptr::NonNull<core::fmt::rt::Argument<'_>> (Transmute);
        _4 = Arguments::<'_> { template: move _21, args: move _22 };
        _3 = panic_fmt(move _4) -> unwind continue;
    }"#;
        let (lines, decls) = compiled(body);
        assert_eq!(
            lines,
            [r#"_3 = core::panicking::panic(const "negative card: ") -> unwind continue;"#]
        );
        assert_eq!(decls, ["_0", "_3"]);
    }

    #[test]
    fn a_template_is_read_to_its_end_or_refused() {
        assert_eq!(
            template_text(b"\x06hello \xc0\x01\n\x00", 1).as_deref(),
            Some(r"hello \n")
        );
        // a long piece; a placeholder with flags, a width, a precision and its argument's index
        let long = [
            &[0x80, 3, 0][..],
            b"abc",
            &[0xcf, 0, 0, 0, 0, 5, 0, 2, 0, 1, 0, 1, b'!', 0],
        ]
        .concat();
        assert_eq!(template_text(&long, 2).as_deref(), Some("abc!"));
        assert_eq!(template_text(&long, 1), None, "argument 1 of 1");
        assert_eq!(
            template_text(b"\x01a\xc0\xc0\x00", 1),
            None,
            "two placeholders, one argument"
        );
        assert_eq!(
            template_text(b"\x01a\xd2\x05\x00\x00\x00\x00", 2),
            None,
            "a width from an argument"
        );
        assert_eq!(template_text(b"\x01a\x90\x00", 0), None, "no such part");
        assert_eq!(template_text(b"\x05ab\x00", 0), None, "past the end");
        assert_eq!(template_text(b"\x01a\x00\x00", 0), None, "after the end");
        assert_eq!(template_text(b"\x01 \xc0\x00", 1), None, "no text but space");
    }
}
