//! Calls: to the program's own functions, by CARDIAC's convention (see the module above); to the built-ins, which
//! are instructions; to panics, which never come back; and pointer arithmetic.
use crate::compile::ir::{self, GenericArg, IntTy, Local, Segment, Terminator, Ty};
use crate::compile::machine::{Kind, Line, Op};
use crate::compile::Fail;

use super::places::{Loc, Value};
use super::symbols::{shift, Cell};
use super::{is_word, numbered, plain_unwind, Access, Lowering, Operand, Sym};

/// `core::ptr::mut_ptr::<impl *mut u8>::add` and its kin: p.add(n) on a raw pointer to u8.
fn is_pointer_add(func: &ir::Path) -> bool {
    let [module @ .., Segment::Impl(Ty::Ptr { to, .. }), method] = func.segments.as_slice() else {
        return false;
    };
    // a name without generic arguments, one of these
    fn named(s: &Segment, names: &[&str]) -> bool {
        matches!(s, Segment::Name { name, args: None } if names.contains(&name.as_str()))
    }
    let in_ptr = matches!(module, [core, ptr, kind]
        if named(core, &["core"]) && named(ptr, &["ptr"]) && named(kind, &["mut_ptr", "const_ptr"]));
    func.qself.is_none() && **to == Ty::Int(IntTy::U8) && in_ptr && named(method, &["add", "offset", "wrapping_add"])
}

/// A callee the lowering goes on to look at: a path whose generic arguments, at most one to a segment, are consts
/// or types made of names and raw pointers, as in `core::ptr::mut_ptr::<impl *mut u8>::add` or `load_resource::<3>`. A
/// callee with a tuple, an array, a reference or a lifetime in it, or two generic arguments to a segment, is a
/// terminator this backend does not have.
fn is_lowerable(func: &ir::Path) -> bool {
    fn path(p: &ir::Path) -> bool {
        let qself = p
            .qself
            .as_ref()
            .is_none_or(|q| ty(&q.ty) && q.as_trait.as_ref().is_none_or(path));
        qself && p.segments.iter().all(segment)
    }
    fn segment(s: &Segment) -> bool {
        match s {
            Segment::Name { args: None, .. } => true,
            Segment::Name { args: Some(a), .. } => match a.args.as_slice() {
                [] | [GenericArg::Const(_)] => true,
                [GenericArg::Ty(t)] => ty(t),
                _ => false,
            },
            Segment::Impl(t) => ty(t),
        }
    }
    fn ty(t: &Ty) -> bool {
        match t {
            Ty::Int(_) | Ty::Bool | Ty::Char | Ty::Str => true,
            Ty::Ptr { to, .. } => ty(to),
            Ty::Path(p) | Ty::Dyn(p) => path(p),
            _ => false,
        }
    }
    (func.qself.is_some() || !func.segments.is_empty()) && path(func)
}

/// `load_resource::<N>(..)`
fn is_load_resource(func: &ir::Path) -> bool {
    func.qself.is_none()
        && matches!(func.segments.as_slice(), [Segment::Name { name, args: Some(a) }]
            if name == "load_resource" && a.turbofish && matches!(a.args.as_slice(), [ir::GenericArg::Const(_)]))
}

/// What panic!, assert! and unreachable! call, and `unwrap()` and `expect()` on the failing variant:
/// functions that never come back.
const NEVER_RETURN: [&[&str]; 10] = [
    &["core", "panicking", "panic"],
    &["core", "panicking", "panic_fmt"],
    &["core", "panicking", "panic_bounds_check"],
    &["core", "panicking", "unreachable_display"],
    &["core", "hint", "unreachable_unchecked"],
    &["core", "intrinsics", "unreachable"],
    &["core", "option", "unwrap_failed"],
    &["core", "option", "expect_failed"],
    &["core", "result", "unwrap_failed"],
    &["core", "slice", "index", "slice_index_fail"],
];

/// What Rust says when one of NEVER_RETURN is called. `panic`'s message is its argument, a string: a bare
/// `panic!()` or `assert!(c)`, or a literal message (`panic_messages`). So is `expect()`'s, and `Result::unwrap()`'s
/// (Rust adds the `Err`'s value, which isn't here).
fn called_panic(func: &ir::Path, args: &[ir::Operand]) -> String {
    let calls_panicking = |name: &str| func.is_path(&["core", "panicking", name]) || func.is_path(&[name]);
    let calls = |module: &str, name: &str| {
        func.is_path(&["core", module, name]) || func.is_path(&[module, name]) || func.is_path(&[name])
    };
    if calls("option", "unwrap_failed") {
        return "called `Option::unwrap()` on a `None` value".into();
    }
    if calls_panicking("panic") || calls("option", "expect_failed") || calls("result", "unwrap_failed") {
        if let Some(ir::Operand::Constant(ir::Const::Str(s))) = args.first() {
            return s.clone();
        }
    }
    // a range out of a slice's bounds, `slice_index_fail(start, end, len)`: Rust's message names the bound that
    // failed, which a start of 0, or an end that is the length, tells
    if func.is_path(&["core", "slice", "index", "slice_index_fail"])
        || func.is_path(&["slice", "index", "slice_index_fail"])
    {
        return match args {
            [s, ..] if s.int().is_some_and(|(v, _)| v == 0) => "range end index out of range for slice",
            [_, e, n] if e == n => "range start index out of range for slice",
            _ => "slice index out of range",
        }
        .into();
    }
    if calls_panicking("panic_bounds_check") {
        "index out of bounds"
    } else if calls_panicking("unreachable_display")
        || func.is_path(&["core", "hint", "unreachable_unchecked"])
        || func.is_path(&["core", "intrinsics", "unreachable"])
        || func.is_path(&["unreachable"])
    {
        "internal error: entered unreachable code"
    } else {
        "explicit panic"
    }
    .into()
}

/// One of NEVER_RETURN, by its full path or by the end of it (`panic`, `option::unwrap_failed`), as the MIR may
/// print it.
pub fn never_returns(func: &ir::Path) -> bool {
    NEVER_RETURN
        .iter()
        .any(|path| (1..=path.len()).any(|n| func.is_path(&path[path.len() - n..])))
}

impl<'a> Lowering<'a> {
    /// A call terminator, or any other this backend does not have.
    pub(super) fn call(&mut self, t: &Terminator) -> Result<(), Fail> {
        let line = t.to_string();
        if let Terminator::Call {
            dest,
            func,
            args,
            target,
            unwind,
        } = t
        {
            // panic!, assert!, unreachable!: a call that never comes back
            let ends_ok = target.is_some() || plain_unwind(unwind);
            if dest.as_local().and_then(numbered).is_some() && ends_ok && never_returns(func) {
                let p = self.use_panic();
                let c = format!("{}(...)", &line[..line.find('(').unwrap()]);
                self.panic = Some(called_panic(func, args));
                self.emit_c(Op::Jmp, p, &c);
                return Ok(());
            }
        }
        // a call of the program's own function that returns `!` (`fn fail() -> !`): its arguments passed and the
        // jump, with no way back
        if let Terminator::Call {
            func,
            args,
            target: None,
            unwind,
            ..
        } = t
        {
            let callee = func.to_string();
            let never = self.callees.sigs.get(&callee).is_some_and(|s| s.ret == Ty::Never);
            if plain_unwind(unwind) && is_word(&callee) && never {
                self.pass_args(&callee, args, &line)?;
                let entry = Sym::Entry(callee.clone());
                self.emit_c(Op::Jmp, entry, &format!("call {callee}, which never returns"));
                return Ok(());
            }
        }
        let Terminator::Call {
            dest,
            func,
            args,
            target: Some(ret),
            unwind,
        } = t
        else {
            fail!(unsupported_terminator(line))
        };
        if !plain_unwind(unwind) || !is_lowerable(func) {
            fail!(unsupported_terminator(line))
        }
        let callee = func.to_string();
        let back = self.label(*ret);
        if is_pointer_add(func) {
            // pointer arithmetic: addresses add
            let mut ops = vec![];
            for x in args {
                let o = self.operand(x)?;
                ops.push(self.mat(o));
            }
            let n = ops
                .get(1)
                .cloned()
                .ok_or_else(|| crate::codes::unsupported_call(line))?;
            self.load(&ops[0])?;
            let x = self.arith(&n)?;
            self.emit_c(Op::Add, x, &format!("ptr.{}(...)", func.name()));
            let d = self.place_any(dest)?;
            self.store(&d, "")?;
            self.emit(Op::Jmp, back);
            return Ok(());
        }
        if func.is_path(&["inp"]) {
            // the card reader: the next card after the program on the deck. A local no other line names is never
            // read, unless it is a result the caller reads
            let unread = dest
                .as_local()
                .and_then(numbered)
                .is_some_and(|l| l != Local(0) && !self.results.contains(&l) && self.count_lines(l) == 1);
            if unread {
                self.emit_c(
                    Op::Inp,
                    Operand::CELL_00,
                    "inp(), not kept: the card is read and dropped",
                );
                self.emit(Op::Jmp, back);
                return Ok(());
            }
            match self.place_any(dest)? {
                Loc::Computed(d) => {
                    self.run_dyn(&d, None);
                    let c = self.fresh("sm");
                    self.emit_c(Op::Sto, c.clone(), "inp() straight into the element");
                    self.cell_here(&c);
                }
                Loc::At(c) => self.emit_c(Op::Inp, c, "inp()"),
            }
            self.emit(Op::Jmp, back);
            return Ok(());
        }
        if func.is_path(&["digit"]) {
            // digit k of v: SFT (k+1)3, built for k and run; the shift first, so v (a table read, say) can come
            // straight into the accumulator after it
            let Some(ks) = args.get(1) else {
                fail!(unsupported_place("undefined"))
            };
            let k = self.operand(ks)?;
            if let Some(kc @ 0..=2) = k.k() {
                // a constant k: the shift is known, `SFT (k+1)3`
                let v = self.operand(&args[0])?;
                self.load(&v)?;
                self.emit_c(Op::Sft, shift(kc + 1, 3), &format!("digit(v, {kc})"));
                let d = self.place_any(dest)?;
                self.store(&d, "")?;
                self.emit(Op::Jmp, back);
                return Ok(());
            }
            let k = self.mat(k);
            let op = self.fresh("sft");
            self.load(&k)?;
            self.emit_c(Op::Sft, shift(1, 0), "digit(): 10 k");
            let c = self.konst(413);
            self.emit_c(Op::Add, c, "SFT 13 + 10 k");
            self.emit(Op::Sto, op.clone());
            let v = self.operand(&args[0])?;
            self.load(&v)?;
            self.cell_here(&op);
            self.code.last_mut().unwrap().note("becomes SFT (k+1)3");
            let d = self.place_any(dest)?;
            self.store(&d, "")?;
            self.emit(Op::Jmp, back);
            return Ok(());
        }
        if func.is_path(&["skip_cards"]) {
            // n cards read and dropped: a few in place, else the shared row of INP 00, entered by a TAC it builds
            let [n] = args.as_slice() else {
                fail!(skip_cards_arguments())
            };
            let n = self.operand(n)?;
            match n.k() {
                Some(k) if !(0..=999).contains(&k) => fail!(skip_cards_range(k)),
                Some(k @ 0..=3) => {
                    for i in 0..k {
                        let note = if i == 0 {
                            format!("skip_cards({k})")
                        } else {
                            String::new()
                        };
                        self.emit_c(Op::Inp, Operand::CELL_00, &note);
                    }
                }
                _ => {
                    self.sh.pool.skip = true;
                    self.load(&n)?;
                    self.emit_c(Op::Jmp, Sym::Runtime("rt_skip_entry"), "skip_cards(n)");
                    let r = self.fresh("rr");
                    self.define(&r);
                }
            }
            self.emit(Op::Jmp, back);
            return Ok(());
        }
        if func.is_path(&["add_times"]) {
            // n c by n additions: for a constant n, c * n in place (adds and a shift); else the shared row of ADD c
            let [c, n] = args.as_slice() else {
                fail!(add_times_arguments())
            };
            let (c, n) = (self.operand(c)?, self.operand(n)?);
            match n.k() {
                Some(k) if !(0..=99).contains(&k) => fail!(add_times_range(k)),
                Some(_) => {
                    let c = self.mat(c);
                    self.arith_acc(ir::BinOp::Mul, c, n, None, "add_times(c, n)")?;
                }
                None => {
                    self.sh.pool.times = true;
                    self.pass(&c, &Sym::Runtime("rt_times_c"), "add_times(c, n)")?;
                    self.load(&n)?;
                    self.emit(Op::Jmp, Sym::Runtime("rt_times_entry"));
                    let r = self.fresh("rr");
                    self.define(&r);
                }
            }
            let d = self.place_any(dest)?;
            self.store(&d, "")?;
            self.emit(Op::Jmp, back);
            return Ok(());
        }
        if is_load_resource(func) {
            // a resource: cells the loader fills, so no code
            let [ir::Operand::Constant(ir::Const::Str(name))] = args.as_slice() else {
                fail!(resource_not_literal())
            };
            if !is_word(name) {
                fail!(resource_bad_name(format!("{name:?}")))
            }
            if self.sh.resources.contains(name) {
                fail!(resource_twice(name))
            }
            // the loader fills a slot once, and natively each call reads the file afresh: they agree only when the
            // call runs once, so it is in main and on no loop (the program may write to the array after)
            if self.func != "main" {
                fail!(resource_outside_main(name))
            }
            if self.in_loop {
                fail!(resource_in_loop(name))
            }
            let Some(l) = dest.as_local() else {
                fail!(resource_unbound(name))
            };
            let c = self.cell(l, None);
            self.sh.resources.push(name.clone());
            self.resources.insert(c, name.clone());
            self.emit_c(
                Op::Jmp,
                back,
                &format!("load_resource(\"{name}\"): the loader fills it"),
            );
            return Ok(());
        }
        if func.is_path(&["overlay"]) {
            // end of a phase: the machine stops with the bug on the number of the tape to load next, by the overlay
            // convention (the bug on 00 is done); the operator loads it and starts the loader at 00, which reads
            // the next phase's code over this one
            if self.func != "main" {
                fail!(overlay_outside_main())
            }
            let tape = self.sh.phase_starts.len() + 2;
            if tape > 98 {
                fail!(too_many_phases())
            }
            self.emit_c(
                Op::Hrs,
                Operand::Fixed(tape as u8),
                &format!("overlay(): stop for tape {tape}"),
            );
            self.code.push(Line::new(Kind::Phase(back.to_string())));
            self.sh.phase_starts.push(back.to_string());
            return Ok(());
        }
        if func.is_path(&["out"]) {
            if args.len() != 1 {
                fail!(unsupported_place(
                    args.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ")
                ))
            }
            let o = self.operand(&args[0])?;
            if let Value::Computed(d) = &o {
                // out(xs[i]): OUT built on the element's address
                let f = self.run_dyn(d, Some(Access::Out));
                let c = self.selfmod(Access::Out, "out(...) straight from the element", f);
                self.cell_here(&c);
                self.emit(Op::Jmp, back);
                return Ok(());
            }
            let v = self.mat(o);
            let x = self.arith(&v)?;
            self.emit_c(Op::Out, x, "out(...)");
            self.emit(Op::Jmp, back);
            return Ok(());
        }
        if !is_word(&callee) {
            fail!(unsupported_call(callee))
        }
        let Some(sig) = self.callees.sigs.get(&callee) else {
            fail!(unknown_function(callee))
        };
        let returns = sig.ret != Ty::Unit;
        self.pass_args(&callee, args, &line)?;
        let entry = Sym::Entry(callee.clone());
        self.emit_c(
            Op::Jmp,
            entry,
            &format!("call {callee}: the return instruction goes to cell 99"),
        );
        let r = self.fresh("ret");
        self.define(&r); // the callee comes back here; this label keeps the peephole pass off it
        if returns {
            let p = self.place(dest)?;
            let result = Sym::local(&callee, Local(0));
            match self.array_at(&p).map(|a| a.len as u32) {
                // an array: each element (not a grid's holes)
                Some(n) => {
                    for k in self.kept(&p, n) {
                        self.emit(Op::Cla, Cell::element(&result, k));
                        self.emit(Op::Sto, Cell::element(&p.sym, k));
                    }
                }
                None => {
                    self.emit(Op::Cla, result);
                    self.emit(Op::Sto, p);
                }
            }
        }
        self.emit(Op::Jmp, back);
        Ok(())
    }

    /// A call's arguments, each into its parameter's cell (`line` is the call, for the first one's comment).
    fn pass_args(&mut self, callee: &str, args: &[ir::Operand], line: &str) -> Result<(), Fail> {
        let Some(sig) = self.callees.sigs.get(callee) else {
            fail!(unknown_function(callee))
        };
        let params = sig.params.clone();
        let widths = sig.widths.clone();
        if args.len() != params.len() {
            fail!(internal(format!(
                "call to {callee} with {} arguments, expected {}",
                args.len(),
                params.len()
            )))
        }
        for (i, a) in args.iter().enumerate() {
            let o = self.operand(a)?;
            let c = if i == 0 {
                line[..line.find(" -> ").unwrap()].to_string()
            } else {
                String::new()
            };
            let to = self.callees.home(Sym::local(callee, params[i]));
            if widths[i] > 1 {
                // an array passed by value: every element copied into the parameter's cells
                let Value::At(Cell {
                    sym: from,
                    offset: None,
                }) = o
                else {
                    fail!(array_by_value(callee))
                };
                let whole = Cell::from(from.clone());
                for k in self.kept(&whole, widths[i] as u32) {
                    self.emit_c(Op::Cla, Cell::element(&from, k), if k == 0 { &c } else { "" });
                    self.emit(Op::Sto, Cell::element(&to, k));
                }
                continue;
            }
            self.pass(&o, &to, &c)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The callee of a call line, as rustc writes one.
    fn callee(path: &str) -> ir::Path {
        let line = format!("_3 = {path}(copy _2) -> [return: bb1, unwind continue];");
        match ir::parse_line(&line) {
            Ok(ir::Line::Terminator(Terminator::Call { func, .. })) => func,
            other => panic!("not a call: {other:?}"),
        }
    }

    #[test]
    fn a_callee_is_lowered_when_its_generic_arguments_are_simple() {
        let lowerable = [
            "out",
            "kit::arith::twice",
            "load_resource::<3>",
            "core::ptr::mut_ptr::<impl *mut u8>::add",
            "<core::ops::Range<i16> as Iterator>::next",
            "<Rev<core::ops::RangeInclusive<u8>> as IntoIterator>::into_iter",
        ];
        for p in lowerable {
            assert!(is_lowerable(&callee(p)), "{p}");
        }
        let not = [
            "core::slice::<impl [i16]>::len",
            "<&mut core::ops::Range<i16> as Iterator>::next",
            "core::mem::swap::<(i16, i16)>",
            "pick::<i16, u8>",
        ];
        for p in not {
            assert!(!is_lowerable(&callee(p)), "{p}");
        }
    }

    #[test]
    fn pointer_arithmetic_is_found_by_its_path() {
        for p in [
            "core::ptr::mut_ptr::<impl *mut u8>::add",
            "core::ptr::const_ptr::<impl *const u8>::offset",
            "core::ptr::mut_ptr::<impl *const u8>::wrapping_add",
        ] {
            assert!(is_pointer_add(&callee(p)), "{p}");
        }
        for p in [
            "core::ptr::mut_ptr::<impl *mut i16>::add",
            "core::ptr::mut_ptr::<impl *mut u8>::sub",
            "ptr::mut_ptr::<impl *mut u8>::add",
            "core::ptr::mut_ptr::<impl *mut u8>::add::<u8>",
        ] {
            assert!(!is_pointer_add(&callee(p)), "{p}");
        }
    }
}
