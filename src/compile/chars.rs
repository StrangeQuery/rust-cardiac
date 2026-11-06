//! `char::to_digit` and `char::from_digit`, which rustc calls at every opt-level, written as the tests core's make,
//! for a radix that is a constant from 2 to 36 (any other panics in core, and stays refused here). A char is a usize
//! by now (`front::CharIsUsize`), and a digit and its u32 are on a card:
//!   `c.to_digit(R)` is `Some(c - '0')` for c from '0' up to the R-th digit, and past '9', `Some(c - 'a' + 10)` or
//!     `Some(c - 'A' + 10)` for the letters below R; else `None`. Its `Option<u32>` is an `Option<u8>`, and its
//!     digit, where it is read (`((o as Some).0: u32)`), is that u8 cast to u32, which `wide` then holds as a cell.
//!   `char::from_digit(n, R)` is `Some` of '0' + n, or 'a' + n - 10, for n below R, else `None`. n is a u32 that is
//!     compared with R and cast to a u8, which `wide` can do with a cell that holds it modulo 2^32: below R the cell
//!     is n itself.
use super::enums::EnumDef;
use super::ir::{
    BasicBlock, BinOp, Block, CastKind, Const, Decl, File, GenericArg, GenericArgs, IntTy, Item, Local, Operand, Path,
    Place, Projection, Rvalue, Segment, Statement, Terminator, Ty,
};

/// Which of the two a call is, by its path: `char::methods::<impl char>::to_digit`, `char::convert::from_digit`.
fn method(func: &Path) -> Option<&'static str> {
    let first = matches!(func.segments.first(), Some(Segment::Name { name, args: None }) if name == "char");
    let core = matches!(func.segments.as_slice(), [Segment::Name { name, .. }, Segment::Name { name: c, .. }, ..] if name == "core" && c == "char");
    if func.qself.is_some() || !(first || core) {
        return None;
    }
    match func.name() {
        "to_digit" => Some("to_digit"),
        "from_digit" => Some("from_digit"),
        _ => None,
    }
}

fn usize(v: i128) -> Operand {
    Operand::Constant(Const::Int {
        value: v,
        ty: IntTy::Usize,
    })
}

fn copy(l: Local) -> Operand {
    Operand::Copy(Place::local(l))
}

/// `core::option::Option::<T>`, and its variants' paths.
fn option(t: Ty) -> Ty {
    Ty::Path(Path {
        qself: None,
        segments: ["core", "option"]
            .iter()
            .map(|w| Segment::Name {
                name: w.to_string(),
                args: None,
            })
            .chain([Segment::Name {
                name: "Option".into(),
                args: Some(GenericArgs {
                    turbofish: false,
                    args: vec![GenericArg::Ty(t)],
                }),
            }])
            .collect(),
    })
}

fn variant(t: &Ty, v: &str) -> Path {
    let Ty::Path(p) = t else {
        unreachable!("an Option's type is a path")
    };
    let mut p = p.clone();
    p.segments.push(Segment::Name {
        name: v.into(),
        args: None,
    });
    p
}

/// The blocks a call becomes, built from the block that makes it.
struct Writer {
    next_local: u32,
    next_block: u32,
    decls: Vec<Decl>,
    blocks: Vec<Block>,
}

impl Writer {
    fn local(&mut self, ty: Ty) -> Local {
        let l = Local(self.next_local);
        self.next_local += 1;
        self.decls.push(Decl {
            local: l,
            mutable: false,
            ty,
        });
        l
    }

    fn block(&mut self) -> BasicBlock {
        self.next_block += 1;
        BasicBlock(self.next_block - 1)
    }

    /// A test of `op(x, k)`: on to `yes` where it holds, else to `no`, from a block of its own.
    fn test(&mut self, op: BinOp, x: Operand, k: Operand, yes: BasicBlock, no: BasicBlock) -> BasicBlock {
        let name = self.block();
        let b = self.local(Ty::Bool);
        self.blocks.push(Block {
            name,
            statements: vec![Statement::Assign(Place::local(b), Rvalue::BinaryOp(op, x, k))],
            terminator: Terminator::SwitchInt {
                discr: Operand::Move(Place::local(b)),
                arms: vec![(0, no)],
                otherwise: Some(yes),
            },
        });
        name
    }

    /// A block that sets `dest` and goes on to `ret`.
    fn set(&mut self, statements: Vec<Statement>, ret: BasicBlock) -> BasicBlock {
        let name = self.block();
        self.blocks.push(Block {
            name,
            statements,
            terminator: Terminator::Goto(ret),
        });
        name
    }
}

/// Each call of the two in the program, written out. Returns the `Option<u8>` the program had no definition of,
/// made from its `Option<u32>`'s.
pub fn char_digits(file: &mut File, enums: &[EnumDef]) -> Vec<EnumDef> {
    let mut made = vec![];
    for item in &mut file.items {
        let Item::Fn(f) = item else { continue };
        let calls: Vec<usize> = (0..f.body.blocks.len())
            .filter(|i| {
                f.body.blocks[*i]
                    .terminator
                    .as_call()
                    .is_some_and(|(_, func, _)| method(func).is_some())
            })
            .collect();
        if calls.is_empty() {
            continue;
        }
        let mut w = Writer {
            next_local: f
                .params
                .iter()
                .map(|(l, _)| l.0)
                .chain(f.body.decls.iter().map(|d| d.local.0))
                .max()
                .unwrap_or(0)
                + 1,
            next_block: f.body.blocks.iter().map(|b| b.name.0).max().unwrap_or(0) + 1,
            decls: vec![],
            blocks: vec![],
        };
        for i in calls {
            let Terminator::Call {
                dest,
                func,
                args,
                target: Some(ret),
                ..
            } = f.body.blocks[i].terminator.clone()
            else {
                continue;
            };
            let ([x, Operand::Constant(Const::Int { value: radix, .. })], Some(o)) = (args.as_slice(), dest.as_local())
            else {
                continue;
            };
            if !(2..=36).contains(radix) {
                continue;
            }
            // (each operand is read more than once)
            let x = match x {
                Operand::Move(p) => &Operand::Copy(p.clone()),
                x => x,
            };
            let start = match method(&func) {
                Some("to_digit") => {
                    let option_of = |t: IntTy| {
                        enums
                            .iter()
                            .find(|d| d.path == ["core", "option", "Option"] && d.args == [Ty::Int(t)])
                    };
                    let Some(u32_def) = option_of(IntTy::U32) else { continue };
                    if !digit_read_alone(&mut f.body, o) {
                        continue;
                    }
                    if option_of(IntTy::U8).is_none() && made.is_empty() {
                        let mut d = u32_def.clone();
                        d.args = vec![Ty::Int(IntTy::U8)];
                        for v in d.variants.iter_mut().filter(|v| !v.fields.is_empty()) {
                            v.fields = vec![Ty::Int(IntTy::U8)];
                        }
                        made.push(d);
                    }
                    to_digit(&mut w, x.clone(), *radix, o, ret)
                }
                _ => from_digit(&mut w, x.clone(), *radix, o, ret),
            };
            f.body.blocks[i].terminator = Terminator::Goto(start);
        }
        f.body.decls.append(&mut w.decls);
        f.body.blocks.append(&mut w.blocks);
    }
    made
}

/// Whether `to_digit`'s result, local o, is read only by its discriminant and its digit copied into a whole
/// local, `_11 = copy ((_9 as Some).0: u32)`; if so, o is made an `Option<u8>` and each such copy reads a u8 and
/// casts it.
fn digit_read_alone(body: &mut super::ir::Body, o: Local) -> bool {
    let digit = |p: &Place| {
        p.projection
            == [
                Projection::Downcast("Some".into()),
                Projection::Field(0, Ty::Int(IntTy::U32)),
            ]
    };
    let mut reads = 0;
    for b in &body.blocks {
        for s in &b.statements {
            match s {
                Statement::Assign(_, Rvalue::Discriminant(p)) if p.as_local() == Some(o) => reads += 1,
                Statement::Assign(d, Rvalue::Use(Operand::Copy(p) | Operand::Move(p)))
                    if p.base_local() == Some(o) && digit(p) && d.as_local().is_some() =>
                {
                    reads += 1
                }
                Statement::StorageLive(l) | Statement::StorageDead(l) if *l == o => reads += 1,
                _ => {}
            }
        }
        // (the call that sets it)
        if b.terminator
            .as_call()
            .is_some_and(|(d, func, _)| d.as_local() == Some(o) && method(func) == Some("to_digit"))
        {
            reads += 1;
        }
    }
    let named: usize = body.blocks.iter().flat_map(|b| b.lines()).map(|l| l.count(o)).sum();
    if reads != named {
        return false;
    }
    for d in body.decls.iter_mut().filter(|d| d.local == o) {
        d.ty = option(Ty::Int(IntTy::U8));
    }
    let mut next = body.decls.iter().map(|d| d.local.0).max().unwrap_or(0) + 1;
    let mut added = vec![];
    for b in body.blocks.iter_mut() {
        let mut statements = vec![];
        for s in std::mem::take(&mut b.statements) {
            match s {
                Statement::Assign(d, Rvalue::Use(Operand::Copy(mut p) | Operand::Move(mut p)))
                    if p.base_local() == Some(o) =>
                {
                    let t = Local(next);
                    next += 1;
                    added.push(Decl {
                        local: t,
                        mutable: false,
                        ty: Ty::Int(IntTy::U8),
                    });
                    p.projection[1] = Projection::Field(0, Ty::Int(IntTy::U8));
                    statements.push(Statement::Assign(Place::local(t), Rvalue::Use(Operand::Copy(p))));
                    statements.push(Statement::Assign(
                        d,
                        Rvalue::Cast(copy(t), Ty::Int(IntTy::U32), CastKind::IntToInt),
                    ));
                }
                s => statements.push(s),
            }
        }
        b.statements = statements;
    }
    body.decls.extend(added);
    true
}

/// `c.to_digit(radix)` into o, an `Option<u8>`: the blocks, the first of them returned.
fn to_digit(w: &mut Writer, c: Operand, radix: i128, o: Local, ret: BasicBlock) -> BasicBlock {
    let t = option(Ty::Int(IntTy::U8));
    let none = w.set(
        vec![Statement::Assign(
            Place::local(o),
            Rvalue::Variant(variant(&t, "None"), vec![]),
        )],
        ret,
    );
    // `Some(c - less)`, where c is in from..to
    let range = |w: &mut Writer, from: i128, to: i128, less: i128, otherwise: BasicBlock| {
        let (d, u) = (w.local(Ty::Int(IntTy::Usize)), w.local(Ty::Int(IntTy::U8)));
        let some = w.set(
            vec![
                Statement::Assign(Place::local(d), Rvalue::BinaryOp(BinOp::Sub, c.clone(), usize(less))),
                Statement::Assign(
                    Place::local(u),
                    Rvalue::Cast(copy(d), Ty::Int(IntTy::U8), CastKind::IntToInt),
                ),
                Statement::Assign(Place::local(o), Rvalue::Variant(variant(&t, "Some"), vec![copy(u)])),
            ],
            ret,
        );
        let below = w.test(BinOp::Lt, c.clone(), usize(to), some, otherwise);
        w.test(BinOp::Ge, c.clone(), usize(from), below, otherwise)
    };
    let zero = '0' as i128;
    let letters = radix - 10;
    if letters <= 0 {
        return range(w, zero, zero + radix, zero, none);
    }
    // (a letter's digit is 10 more than its place in the alphabet)
    let (a, upper_a) = ('a' as i128, 'A' as i128);
    let upper = range(w, upper_a, upper_a + letters, upper_a - 10, none);
    let lower = range(w, a, a + letters, a - 10, upper);
    range(w, zero, zero + 10, zero, lower)
}

/// `char::from_digit(n, radix)` into o, an `Option<char>` (a usize's): the blocks, the first of them returned.
fn from_digit(w: &mut Writer, n: Operand, radix: i128, o: Local, ret: BasicBlock) -> BasicBlock {
    let t = option(Ty::Int(IntTy::Usize));
    let none = w.set(
        vec![Statement::Assign(
            Place::local(o),
            Rvalue::Variant(variant(&t, "None"), vec![]),
        )],
        ret,
    );
    let u = w.local(Ty::Int(IntTy::U8));
    let some = |w: &mut Writer, base: i128| {
        let (sum, ch) = (w.local(Ty::Int(IntTy::U8)), w.local(Ty::Int(IntTy::Usize)));
        let u8k = Operand::Constant(Const::Int {
            value: base,
            ty: IntTy::U8,
        });
        w.set(
            vec![
                Statement::Assign(Place::local(sum), Rvalue::BinaryOp(BinOp::Add, copy(u), u8k)),
                Statement::Assign(
                    Place::local(ch),
                    Rvalue::Cast(copy(sum), Ty::Int(IntTy::Usize), CastKind::IntToInt),
                ),
                Statement::Assign(Place::local(o), Rvalue::Variant(variant(&t, "Some"), vec![copy(ch)])),
            ],
            ret,
        )
    };
    let digit = some(w, '0' as i128);
    let chosen = if radix > 10 {
        let letter = some(w, 'a' as i128 - 10);
        let u8k = Operand::Constant(Const::Int {
            value: 10,
            ty: IntTy::U8,
        });
        w.test(BinOp::Lt, copy(u), u8k, digit, letter)
    } else {
        digit
    };
    // below the radix, n is on a card, and its u8 is n
    let cast = w.set(
        vec![Statement::Assign(
            Place::local(u),
            Rvalue::Cast(n.clone(), Ty::Int(IntTy::U8), CastKind::IntToInt),
        )],
        chosen,
    );
    let r32 = Operand::Constant(Const::Int {
        value: radix,
        ty: IntTy::U32,
    });
    w.test(BinOp::Lt, n, r32, cast, none)
}
