//! The compiler's IR (`ir::File`) built from rustc_public's MIR, rather than parsed from the text rustc prints: the
//! same `File`, item for item, as `ir::parse_file` makes of `rustc --emit=mir`'s text (the oracle, `check.rs`, holds
//! the two equal).
//!
//! What the IR has as structure comes from rustc_public's: the items and their bodies, locals and their types'
//! shapes, statements, places and projections, operands, rvalues, terminators, switch arms, assert messages, and
//! integer, bool and unit constants from their bits. What the IR has as a name or as text is rustc's own printer's,
//! as the text's is, through `rustc_internal`:
//! - an item's name (trimmed paths, and `<impl at FILE:L:C: L:C>` for an impl's items), a called function's, a
//!   struct's or variant's, and the path in a named type: how rustc trims a path depends on what its resolver saw,
//!   which rustc_public doesn't show;
//! - a constant other than an integer, a bool or `()` (a struct's value, a named or promoted constant, a string);
//! - a line the IR doesn't represent (`Unsupported`), and the items it keeps only as text (a promoted constant, a
//!   static, an allocation's bytes);
//! - a debug name for a field of a variable (`((p: Point).0: i16)`).
//!
//! And from rustc's own MIR, what rustc_public leaves out: the scope each local is declared in (the text declares
//! them scope by scope, and the IR keeps that order: a function's cells are in it), a pointer coercion's source
//! (`Implicit`, `AsCast`) and why an unwind terminates.
//!
//! The parser has its oddities, kept here so that the two agree; each is said where it is matched.
use std::collections::HashMap;

use cargo_cardiac::compile::ir;
use rustc_middle::mir as mmir;
use rustc_middle::ty::print::{ForcedImplGuard, NoTrimmedGuard};
use rustc_middle::ty::{self as mty, TyCtxt};
use rustc_public::mir as pmir;
use rustc_public::rustc_internal::{internal, stable};
use rustc_public::ty as pty;
use rustc_public::CrateDef;
use rustc_span::def_id::DefId;

/// How a type's paths are printed: trimmed (`Range<i16>`, a signature's, a constant's), or in full
/// (`core::ops::Range<i16>`, a local's declaration, a field's, a cast's).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Paths {
    Trimmed,
    Full,
}

pub struct Convert<'tcx> {
    tcx: TyCtxt<'tcx>,
    /// For another crate's items, that crate's name: rustc names them for the program's session, which starts
    /// their paths with it (`kit::tables::SQUARES`); the linker takes them without it.
    krate: Option<String>,
    /// The items named otherwise than the text names them (an impl's functions): each one's index, and the
    /// text's name for it.
    renamed: std::cell::RefCell<Vec<(usize, ir::ItemPath)>>,
    /// The index of the item being made.
    made: std::cell::Cell<usize>,
    /// The crate's own functions its calls reach, each resolved to the definition it runs (a trait's function to
    /// the impl's).
    called: std::collections::HashSet<DefId>,
}

/// The local crate's MIR as the IR, item for item as `--emit=mir` writes it (`rustc_middle::mir::write_mir_pretty`),
/// and the items it names otherwise than the text does: each one's index and the text's name for it.
///
/// The one deliberate difference: a function in an impl is named as a call to it is, `P::sum` or
/// `<P as Twice>::twice`, where the text names it by where the impl is, `<impl at src/main.rs:12:1: 12:7>::sum`, and
/// a call to it could never be matched to it. Only one that a call in the crate reaches is (each call resolved to
/// the definition it runs, the impl's for a trait's function): the compiler reads every function named so, and
/// one no call reaches, as the text names it, it leaves out, as before. (An impl's const keeps the text's name, by
/// which the front end knows whose it is.)
pub fn file(tcx: TyCtxt<'_>) -> (ir::File, Vec<(usize, ir::ItemPath)>) {
    let local: Vec<DefId> = rustc_public::all_local_items()
        .into_iter()
        .map(|item| internal(tcx, item.def_id()))
        .collect();
    items(tcx, &local, None)
}

/// The items of another crate that have MIR in its metadata (all its functions', built with -Zalways-encode-mir),
/// in the order its own `--emit=mir` would write them.
pub fn crate_items(tcx: TyCtxt<'_>, krate: rustc_span::def_id::CrateNum) -> Vec<DefId> {
    let mut defs: Vec<DefId> = (0..tcx.num_extern_def_ids(krate))
        .map(|i| DefId {
            krate,
            index: rustc_span::def_id::DefIndex::from_usize(i),
        })
        .filter(|&def| {
            use rustc_hir::def::{CtorKind, DefKind as K};
            match tcx.def_kind(def) {
                K::Fn | K::AssocFn | K::Closure | K::SyntheticCoroutineBody => tcx.is_mir_available(def),
                // a tuple struct's or variant's constructor as a function: another crate's metadata has its MIR
                // only when rustc encoded it, and uses of it are written as the value built anyway
                K::Ctor(_, kind) => kind == CtorKind::Fn && tcx.is_mir_available(def),
                K::Const { .. } => true,
                K::AnonConst | K::InlineConst => tcx.trivial_const(def).is_some() || ctfe_mir_encoded(tcx, def),
                // a trait's const without a value has no body
                K::AssocConst { .. } => tcx.associated_item(def).defaultness(tcx).has_value(),
                _ => false,
            }
        })
        .collect();
    let _ = &mut defs;
    defs
}

/// Whether another crate's metadata has an anonymous constant's MIR. rustc makes a definition for every constant
/// argument, and one it lowers to a path (`[i16; N]`, N a const parameter) has no body and nothing encoded for it, and
/// no query says so without panicking: its MIR is asked for, and the panic (rustc's "does not have a mir_for_ctfe")
/// taken as no.
fn ctfe_mir_encoded(tcx: TyCtxt<'_>, def: DefId) -> bool {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let found = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = tcx.mir_for_ctfe(def);
    }))
    .is_ok();
    std::panic::set_hook(hook);
    found
}

/// These items of one crate, the local one or another (`krate`, its name, for another's), as the IR: as `file`
/// makes the local crate's, each named as rustc names it in this session.
pub fn items(tcx: TyCtxt<'_>, defs: &[DefId], krate: Option<&str>) -> (ir::File, Vec<(usize, ir::ItemPath)>) {
    let c = Convert {
        tcx,
        krate: krate.map(str::to_string),
        renamed: Default::default(),
        made: Default::default(),
        called: called(tcx, defs),
    };
    let mut items = vec![];
    for &def in defs {
        if tcx.is_const_fn(def) {
            // a const fn: its body, then the body compile-time evaluation runs (its promoteds are shared)
            c.made.set(items.len());
            items.extend(c.body_items(tcx.optimized_mir(def), true));
            c.made.set(items.len());
            items.extend(c.body_items(tcx.mir_for_ctfe(def), false));
        } else if let Some((val, ty)) = tcx.trivial_const(def) {
            items.push(c.trivial_const(def, val, ty));
        } else {
            c.made.set(items.len());
            let mut made = c.body_items(tcx.instance_mir(mty::InstanceKind::Item(def)), true);
            if let Some(ir::Item::Const(k)) = made.first_mut() {
                c.evaluated_array(def, k);
            }
            items.extend(made);
        }
    }
    (ir::File { items }, c.renamed.into_inner())
}

/// The functions of these items' crate that their calls reach, resolved to the definitions they run.
fn called(tcx: TyCtxt<'_>, defs: &[DefId]) -> std::collections::HashSet<DefId> {
    let mut called = calls(tcx, defs);
    called.retain(|d| defs.first().is_some_and(|f| d.krate == f.krate));
    called
}

/// The functions the calls in these items reach, in any crate, each resolved to the definition it runs.
pub fn calls(tcx: TyCtxt<'_>, defs: &[DefId]) -> std::collections::HashSet<DefId> {
    let mut called = std::collections::HashSet::new();
    for &def in defs {
        if tcx.trivial_const(def).is_some() {
            continue;
        }
        let mut bodies = vec![tcx.instance_mir(mty::InstanceKind::Item(def))];
        if tcx.is_const_fn(def) {
            bodies.push(tcx.mir_for_ctfe(def));
        }
        for body in bodies {
            let env = mty::TypingEnv::post_analysis(tcx, def);
            for b in body.basic_blocks.iter() {
                let mmir::TerminatorKind::Call { func, .. } = &b.terminator().kind else {
                    continue;
                };
                let Some((callee, args)) = func.const_fn_def() else {
                    continue;
                };
                if let Ok(Some(instance)) = mty::Instance::try_resolve(tcx, env, callee, args) {
                    called.insert(instance.def_id());
                    // a trait's own default method (`PartialEq::ne`) calls the trait's other methods, as the
                    // type implements them (its `eq`): those are reached too
                    if let Some(trait_def) = tcx.trait_of_assoc(instance.def_id()) {
                        // the trait's own arguments (Self and its parameters), not the method's own after them
                        // (`RangeBounds::contains::<U>`): resolved with those, rustc records a delayed bug
                        let trait_args = args.truncate_to(tcx, tcx.generics_of(trait_def));
                        for &other in tcx.associated_item_def_ids(trait_def) {
                            // only what the impl must define: one with a default is core's body, and resolving an
                            // item the impl can't have (a bound unmet) is a delayed bug in rustc; and none with
                            // parameters of its own, which the call doesn't give
                            if tcx.def_kind(other) != rustc_hir::def::DefKind::AssocFn
                                || tcx.defaultness(other).has_value()
                                || tcx.generics_of(other).own_requires_monomorphization()
                            {
                                continue;
                            }
                            if let Ok(Some(i)) = mty::Instance::try_resolve(tcx, env, other, trait_args) {
                                called.insert(i.def_id());
                            }
                        }
                    }
                }
            }
        }
    }
    called
}

impl<'tcx> Convert<'tcx> {
    // ---- items ----

    /// The item's name as the text has it: rustc's printer, impls named by where they are.
    fn name(&self, def: DefId) -> String {
        let _impls = ForcedImplGuard::new();
        let name = self.tcx.def_path_str(def);
        match &self.krate {
            Some(k) => name
                .strip_prefix(&format!("{k}::"))
                .map_or(name.clone(), str::to_string),
            None => name,
        }
    }

    /// Text rustc's printer made with trimmed paths: as it is, for this session.
    fn trimmed(&self, text: String) -> String {
        text
    }

    /// Text rustc's printer made without trimmed paths, or both ways: as it is.
    fn full(&self, text: String) -> String {
        text
    }

    /// A function in an impl as a call names it (`P::sum`, `<P as Twice>::twice`: rustc's printer, without naming
    /// the impl by where it is), if it is one, a call in the crate reaches it, and the name reads as an item's.
    fn call_name(&self, def: DefId) -> Option<ir::ItemPath> {
        use rustc_hir::def::DefKind;
        let parent = self.tcx.opt_parent(def)?;
        if self.tcx.def_kind(def) != DefKind::AssocFn
            || !matches!(self.tcx.def_kind(parent), DefKind::Impl { .. })
            || !self.called.contains(&def)
        {
            return None;
        }
        ir::parse_item_path(&self.trimmed(self.tcx.def_path_str(def)))
    }

    /// rustc's text of a body, as it writes it into the file: the body, then the allocations it uses.
    fn text_of(&self, body: &mmir::Body<'tcx>) -> String {
        let mut out = vec![];
        mmir::pretty::MirWriter::new(self.tcx)
            .write_mir_fn(body, &mut out)
            .expect("writing MIR to memory");
        self.full(String::from_utf8_lossy(&out).into_owned())
    }

    /// A body's items: itself, the allocations it uses, and (with `promoteds`) its promoted constants and theirs.
    fn body_items(&self, body: &mmir::Body<'tcx>, promoteds: bool) -> Vec<ir::Item> {
        let mut items = vec![self.body_item(body)];
        // the allocations' bytes, which the IR keeps as text
        let mut allocs = vec![];
        mmir::pretty::write_allocations(self.tcx, body, &mut allocs).expect("writing MIR to memory");
        items.extend(ir::parse_file(&self.full(String::from_utf8_lossy(&allocs).into_owned())).items);
        if promoteds {
            for p in self.tcx.promoted_mir(body.source.def_id()) {
                // `const main::promoted[0]: ..`, which the parser keeps as text: so does the IR
                items.extend(ir::parse_file(&self.text_of(p)).items);
            }
        }
        items
    }

    /// The item a body is: a function, a const with its body, or (a static, an anonymous constant, a promoted)
    /// text, as the parser keeps it.
    fn body_item(&self, body: &mmir::Body<'tcx>) -> ir::Item {
        use rustc_hir::def::DefKind;
        let def = body.source.def_id();
        let kind = self.tcx.def_kind(def);
        let function = matches!(
            kind,
            DefKind::Fn | DefKind::AssocFn | DefKind::Ctor(..) | DefKind::SyntheticCoroutineBody
        ) || self.tcx.is_closure_like(def);
        let constant = matches!(kind, DefKind::Const { .. } | DefKind::AssocConst { .. });
        let as_text = || {
            let text = self.text_of(body);
            ir::parse_file(&text)
                .items
                .into_iter()
                .next()
                .unwrap_or(ir::Item::Other(text))
        };
        let path = ir::parse_item_path(&self.name(def));
        let (Some(path), true) = (path, body.source.promoted.is_none() && (function || constant)) else {
            return as_text();
        };
        let public: pmir::Body = stable(body);
        let converted = self.body(&public, body);
        let path = match self.call_name(def) {
            Some(name) if name != path => {
                self.renamed.borrow_mut().push((self.made.get(), path));
                name
            }
            _ => path,
        };
        if function {
            let params = (1..=body.arg_count)
                .map(|l| (ir::Local(l as u32), self.ty(public.locals()[l].ty, Paths::Trimmed)))
                .collect();
            ir::Item::Fn(ir::FnItem {
                path,
                params,
                ret: self.ty(public.locals()[0].ty, Paths::Trimmed),
                body: converted,
            })
        } else {
            ir::Item::Const(ir::ConstItem {
                path,
                ty: self.ty(public.locals()[0].ty, Paths::Trimmed),
                value: ir::ConstValue::Body(converted),
            })
        }
    }

    /// `const N: i16 = const 40_i16;`: a const rustc has no body for, only its value.
    fn trivial_const(&self, def: DefId, val: mmir::ConstValue, ty: mty::Ty<'tcx>) -> ir::Item {
        let name = self.name(def);
        let konst = mmir::Const::Val(val, ty);
        let public: pty::MirConst = stable(konst);
        let Some(path) = ir::parse_item_path(&name) else {
            let (ty, konst) = (self.trimmed(ty.to_string()), self.full(konst.to_string()));
            return ir::Item::Other(format!("const {name}: {ty} = const {konst};"));
        };
        let value = match self.constant(&public) {
            Some(c) => ir::ConstValue::Const(c),
            // as the parser keeps a value it doesn't read: the text after ` = `, `;` and all
            None => ir::ConstValue::Other(self.full(format!("const {konst};"))),
        };
        ir::Item::Const(ir::ConstItem {
            path,
            ty: self.ty(stable(ty), Paths::Trimmed),
            value,
        })
    }

    /// A const array of numbers or bools whose body works it out (`[A, B, A + B]`, a const fn's table): its body
    /// is made the literal rustc evaluates it to, `_0 = [const 12_i16, ..]`, as the compiler reads a const table. The
    /// text keeps the body that works it out, so this is one place the two differ.
    fn evaluated_array(&self, def: DefId, k: &mut ir::ConstItem) {
        let ir::ConstValue::Body(body) = &k.value else { return };
        let ir::Ty::Array(elem, ir::ArrayLen::Known(n)) = &k.ty else {
            return;
        };
        let literal = |r: &ir::Rvalue| match r {
            ir::Rvalue::Array(os) => os.iter().all(|o| matches!(o, ir::Operand::Constant(_))),
            ir::Rvalue::Repeat(ir::Operand::Constant(_), _) => true,
            _ => false,
        };
        let assigned = body.blocks.iter().flat_map(|b| &b.statements).find_map(|s| match s {
            ir::Statement::Assign(p, r) if p.as_local() == Some(ir::Local(0)) => Some(r),
            _ => None,
        });
        if assigned.is_some_and(literal) {
            return;
        }
        let (size, ty) = match &**elem {
            ir::Ty::Int(t) => (int_size(*t), Some(*t)),
            ir::Ty::Bool => (1, None),
            _ => return,
        };
        let Ok(value) = self.tcx.const_eval_poly(def) else {
            return;
        };
        let konst: pty::MirConst = stable(mmir::Const::Val(
            value,
            self.tcx.type_of(def).instantiate_identity().skip_normalization(),
        ));
        let pty::ConstantKind::Allocated(alloc) = konst.kind() else {
            return;
        };
        let little = self.tcx.data_layout.endian == rustc_abi::Endian::Little;
        let mut values = vec![];
        for e in 0..*n as usize {
            let Some(bytes) = alloc.bytes.get(e * size..(e + 1) * size) else {
                return;
            };
            let Some(mut bytes) = bytes.iter().copied().collect::<Option<Vec<u8>>>() else {
                return;
            };
            if little {
                bytes.reverse();
            }
            let bits = bytes.iter().fold(0u128, |v, b| v << 8 | *b as u128);
            let c = match ty {
                Some(t) => int_const(bits, t),
                None => Some(ir::Const::Bool(bits != 0)),
            };
            let Some(c) = c else { return };
            values.push(ir::Operand::Constant(c));
        }
        k.value = ir::ConstValue::Body(ir::Body {
            decls: vec![ir::Decl {
                local: ir::Local(0),
                mutable: true,
                ty: k.ty.clone(),
            }],
            debug: vec![],
            blocks: vec![ir::Block {
                name: ir::BasicBlock(0),
                statements: vec![ir::Statement::Assign(
                    ir::Place::local(ir::Local(0)),
                    ir::Rvalue::Array(values),
                )],
                terminator: ir::Terminator::Return,
            }],
        });
    }

    // ---- bodies ----

    /// A body: its debug names and its locals' declarations in the order the text gives them (scope by scope, each
    /// scope's names then its locals, then the scopes within it), and its blocks but the cleanup ones.
    fn body(&self, public: &pmir::Body, body: &mmir::Body<'tcx>) -> ir::Body {
        let mut children: HashMap<mmir::SourceScope, Vec<mmir::SourceScope>> = HashMap::new();
        for (s, data) in body.source_scopes.iter_enumerated() {
            if let Some(parent) = data.parent_scope {
                children.entry(parent).or_default().push(s);
            }
        }
        let mut order = vec![];
        let mut todo = vec![mmir::OUTERMOST_SOURCE_SCOPE];
        while let Some(s) = todo.pop() {
            order.push(s);
            if let Some(c) = children.get(&s) {
                todo.extend(c.iter().rev());
            }
        }
        let mut out = ir::Body::default();
        for s in order {
            for (k, info) in body.var_debug_info.iter().enumerate() {
                if info.source_info.scope == s {
                    out.debug.push(self.debug(&public.var_debug_info[k], info, public));
                }
            }
            for (l, decl) in body.local_decls.iter_enumerated() {
                let l = l.as_usize();
                if (1..=body.arg_count).contains(&l) || decl.source_info.scope != s {
                    continue;
                }
                out.decls.push(ir::Decl {
                    local: ir::Local(l as u32),
                    mutable: decl.mutability == mmir::Mutability::Mut,
                    ty: self.ty(public.locals()[l].ty, Paths::Full),
                });
            }
        }
        for (k, (b, internal)) in public.blocks.iter().zip(body.basic_blocks.iter()).enumerate() {
            if internal.is_cleanup {
                continue;
            }
            let statements = b
                .statements
                .iter()
                .zip(&internal.statements)
                .filter_map(|(s, i)| self.statement(s, i, public.locals()))
                .collect();
            out.blocks.push(ir::Block {
                name: ir::BasicBlock(k as u32),
                statements,
                terminator: self.terminator(&b.terminator, internal.terminator(), public.locals()),
            });
        }
        out
    }

    /// `debug x => _3;`: the name, and the value as the IR prints it (a place or a constant), else as rustc does.
    fn debug(&self, public: &pmir::VarDebugInfo, info: &mmir::VarDebugInfo<'tcx>, body: &pmir::Body) -> ir::Debug {
        let text = self.full(format!("{info:?}"));
        let (name_text, value_text) = text.rsplit_once(" => ").unwrap_or((&text, ""));
        // a variable's field (`((p: Point).0: i16)`) is named as rustc prints it
        let name = match public.composite {
            None => public.name.clone(),
            Some(_) => name_text.to_string(),
        };
        let value = match &public.value {
            pmir::VarDebugInfoContents::Place(p) => self.place(p, body.locals()).map(|p| p.to_string()),
            pmir::VarDebugInfoContents::Const(c) => self.const_operand(&c.const_).map(|o| o.to_string()),
        };
        ir::Debug {
            name,
            value: value.unwrap_or_else(|| value_text.to_string()),
        }
    }

    // ---- statements ----

    /// A statement, or none for one the parser leaves out (storage markers, rustc's bookkeeping, `nop`).
    fn statement(
        &self,
        s: &pmir::Statement,
        internal: &mmir::Statement<'tcx>,
        locals: &[pmir::LocalDecl],
    ) -> Option<ir::Statement> {
        use pmir::StatementKind as K;
        let converted = match &s.kind {
            K::StorageLive(_)
            | K::StorageDead(_)
            | K::PlaceMention(_)
            | K::FakeRead(..)
            | K::AscribeUserType { .. }
            | K::Nop => return None,
            K::Assign(p, r) => {
                let rvalue = match &internal.kind {
                    mmir::StatementKind::Assign(b) => Some(&b.1),
                    _ => None,
                };
                self.place(p, locals)
                    .zip(rvalue.and_then(|i| self.rvalue(r, i, locals)))
                    .map(|(p, r)| ir::Statement::Assign(p, r))
            }
            K::Intrinsic(pmir::NonDivergingIntrinsic::Assume(o)) => self.operand(o, locals).map(ir::Statement::Assume),
            _ => None,
        };
        Some(
            converted.unwrap_or_else(|| {
                ir::Statement::Unsupported(ir::Unsupported::of(&self.full(format!("{internal:?};"))))
            }),
        )
    }

    fn rvalue(
        &self,
        r: &pmir::Rvalue,
        internal: &mmir::Rvalue<'tcx>,
        locals: &[pmir::LocalDecl],
    ) -> Option<ir::Rvalue> {
        use pmir::Rvalue as R;
        Some(match r {
            // without a retag it prints `no_retag copy _1`, which the parser doesn't read
            R::Use(o, pmir::WithRetag::Yes) => ir::Rvalue::Use(self.operand(o, locals)?),
            R::Use(_, pmir::WithRetag::No) => return None,
            R::Repeat(o, n) => ir::Rvalue::Repeat(self.operand(o, locals)?, n.eval_target_usize().ok()?),
            R::Ref(_, kind, p) => {
                let kind = match kind {
                    pmir::BorrowKind::Shared => ir::BorrowKind::Shared,
                    pmir::BorrowKind::Mut { .. } => ir::BorrowKind::Mut,
                    // `&fake _1`
                    pmir::BorrowKind::Fake(_) => return None,
                };
                ir::Rvalue::Ref(kind, self.place(p, locals)?)
            }
            R::AddressOf(kind, p) => {
                let kind = match kind {
                    pmir::RawPtrKind::Const => ir::BorrowKind::RawConst,
                    pmir::RawPtrKind::Mut => ir::BorrowKind::RawMut,
                    // `&raw const (fake) _1`
                    pmir::RawPtrKind::FakeForPtrMetadata => return None,
                };
                ir::Rvalue::Ref(kind, self.place(p, locals)?)
            }
            R::Cast(kind, o, t) => {
                let mmir::Rvalue::Cast(internal_kind, ..) = internal else {
                    return None;
                };
                ir::Rvalue::Cast(
                    self.operand(o, locals)?,
                    self.ty_read(*t, Paths::Full)?,
                    cast_kind(kind, internal_kind),
                )
            }
            R::BinaryOp(op, a, b) => {
                let (a, b) = (self.operand(a, locals)?, self.operand(b, locals)?);
                match bin_op(*op) {
                    Some(op) => ir::Rvalue::BinaryOp(op, a, b),
                    // an operation the IR has no name for (`Cmp`, `ShlUnchecked`): the parser reads `Cmp(a, b)` as
                    // a variant built, `Cmp` with two fields
                    None => ir::Rvalue::Variant(name_path(&format!("{op:?}")), vec![a, b]),
                }
            }
            R::CheckedBinaryOp(op, a, b) => {
                let op = match op {
                    pmir::BinOp::Add => ir::BinOp::AddWithOverflow,
                    pmir::BinOp::Sub => ir::BinOp::SubWithOverflow,
                    pmir::BinOp::Mul => ir::BinOp::MulWithOverflow,
                    _ => return None,
                };
                ir::Rvalue::BinaryOp(op, self.operand(a, locals)?, self.operand(b, locals)?)
            }
            R::UnaryOp(op, a) => {
                let a = self.operand(a, locals)?;
                match op {
                    pmir::UnOp::Neg => ir::Rvalue::UnaryOp(ir::UnOp::Neg, a),
                    pmir::UnOp::Not => ir::Rvalue::UnaryOp(ir::UnOp::Not, a),
                    // as for `Cmp`: a variant built, to the parser
                    pmir::UnOp::PtrMetadata => ir::Rvalue::Variant(name_path("PtrMetadata"), vec![a]),
                }
            }
            R::Discriminant(p) => ir::Rvalue::Discriminant(self.place(p, locals)?),
            R::Aggregate(kind, ops) => {
                let ops: Vec<ir::Operand> = ops.iter().map(|o| self.operand(o, locals)).collect::<Option<_>>()?;
                match kind {
                    // `[]` doesn't parse
                    pmir::AggregateKind::Array(_) if !ops.is_empty() => ir::Rvalue::Array(ops),
                    // nor do `()` and `(copy _1,)`
                    pmir::AggregateKind::Tuple if ops.len() >= 2 => ir::Rvalue::Tuple(ops),
                    pmir::AggregateKind::Adt(def, variant, ..) => {
                        let mmir::Rvalue::Aggregate(internal_kind, _) = internal else {
                            return None;
                        };
                        let mmir::AggregateKind::Adt(adt, v, args, ..) = **internal_kind else {
                            return None;
                        };
                        let variant_def = self.tcx.adt_def(adt).variant(v);
                        let name =
                            mty::print::FmtPrinter::print_string(self.tcx, rustc_hir::def::Namespace::ValueNS, |p| {
                                use rustc_middle::ty::print::Printer;
                                p.print_def_path(variant_def.def_id, args)
                            })
                            .ok()?;
                        let path = ir::parse_path(&self.trimmed(name)).ok()?;
                        let public_variant = def.variant(*variant)?;
                        match variant_def.ctor_kind() {
                            // a unit variant of an enum with data, `Shape::Dot` (a C-like enum's is a constant)
                            Some(rustc_hir::def::CtorKind::Const) if self.tcx.adt_def(adt).is_enum() => {
                                ir::Rvalue::Variant(path, vec![])
                            }
                            // a unit struct, or any with no fields, doesn't parse
                            _ if ops.is_empty() => return None,
                            Some(rustc_hir::def::CtorKind::Fn) => ir::Rvalue::Variant(path, ops),
                            Some(rustc_hir::def::CtorKind::Const) => return None,
                            None => {
                                // fields by name, in order; a union's one field is printed under its first field's
                                // name, as rustc's printer pairs them
                                let names = public_variant.fields().into_iter().map(|f| f.name);
                                ir::Rvalue::Struct(path, names.zip(ops).collect())
                            }
                        }
                    }
                    _ => return None,
                }
            }
            _ => return None,
        })
    }

    // ---- terminators ----

    fn terminator(
        &self,
        t: &pmir::Terminator,
        internal: &mmir::Terminator<'tcx>,
        locals: &[pmir::LocalDecl],
    ) -> ir::Terminator {
        self.terminator_kind(&t.kind, &internal.kind, locals)
            .unwrap_or_else(|| {
                ir::Terminator::Unsupported(ir::Unsupported::of(&self.full(format!("{:?};", internal.kind))))
            })
    }

    fn terminator_kind(
        &self,
        t: &pmir::TerminatorKind,
        internal: &mmir::TerminatorKind<'tcx>,
        locals: &[pmir::LocalDecl],
    ) -> Option<ir::Terminator> {
        use pmir::TerminatorKind as T;
        let bb = |b: usize| ir::BasicBlock(b as u32);
        Some(match t {
            T::Goto { target } => ir::Terminator::Goto(bb(*target)),
            T::Return => ir::Terminator::Return,
            T::Unreachable => ir::Terminator::Unreachable,
            T::SwitchInt { discr, targets } => ir::Terminator::SwitchInt {
                discr: self.operand(discr, locals)?,
                // an arm is its bits, as MIR prints it, read as an i128 (u128's past i128::MAX wrap, as the
                // parser's `as i128` does)
                arms: targets.branches().map(|(v, b)| (v as i128, bb(b))).collect(),
                otherwise: Some(bb(targets.otherwise())),
            },
            T::Call {
                func,
                args,
                destination,
                target,
                unwind,
            } => {
                // a call of a function by name; one through a pointer (`move _3(..)`) isn't read
                let pmir::Operand::Constant(f) = func else { return None };
                if !matches!(f.ty().kind(), pty::TyKind::RigidTy(pty::RigidTy::FnDef(..))) {
                    return None;
                }
                let mmir::TerminatorKind::Call {
                    func: internal_func, ..
                } = internal
                else {
                    return None;
                };
                let func = ir::parse_path(&self.trimmed(format!("{internal_func:?}"))).ok()?;
                // a call that doesn't return, with a cleanup block, prints as `-> bb3`: not read
                if target.is_none() && matches!(unwind, pmir::UnwindAction::Cleanup(_)) {
                    return None;
                }
                ir::Terminator::Call {
                    dest: self.place(destination, locals)?,
                    func,
                    args: args.iter().map(|a| self.operand(a, locals)).collect::<Option<_>>()?,
                    target: target.map(bb),
                    unwind: self.unwind(unwind, internal)?,
                }
            }
            T::Assert {
                cond,
                expected,
                msg,
                target,
                unwind,
            } => {
                let (msg, args) = assert_message(msg)?;
                ir::Terminator::Assert {
                    cond: self.operand(cond, locals)?,
                    expected: *expected,
                    msg: msg.to_string(),
                    msg_args: args
                        .into_iter()
                        .map(|a| self.operand(a, locals))
                        .collect::<Option<_>>()?,
                    target: bb(*target),
                    unwind: self.unwind(unwind, internal)?,
                }
            }
            _ => return None,
        })
    }

    fn unwind(&self, u: &pmir::UnwindAction, internal: &mmir::TerminatorKind<'tcx>) -> Option<ir::Unwind> {
        Some(match u {
            pmir::UnwindAction::Continue => ir::Unwind::Continue,
            pmir::UnwindAction::Unreachable => ir::Unwind::Unreachable,
            pmir::UnwindAction::Cleanup(b) => ir::Unwind::Cleanup(ir::BasicBlock(*b as u32)),
            // why it terminates, which rustc_public doesn't say
            pmir::UnwindAction::Terminate => match internal.unwind() {
                Some(mmir::UnwindAction::Terminate(why)) => ir::Unwind::Terminate(why.as_short_str().to_string()),
                _ => return None,
            },
        })
    }

    // ---- places, operands, constants ----

    fn place(&self, p: &pmir::Place, locals: &[pmir::LocalDecl]) -> Option<ir::Place> {
        let mut ty = locals.get(p.local)?.ty;
        let mut projection = vec![];
        for e in &p.projection {
            projection.push(match e {
                pmir::ProjectionElem::Deref => ir::Projection::Deref,
                pmir::ProjectionElem::Field(k, t) => ir::Projection::Field(*k as u32, self.ty_read(*t, Paths::Full)?),
                pmir::ProjectionElem::Index(l) => ir::Projection::Index(ir::Local(*l as u32)),
                pmir::ProjectionElem::ConstantIndex {
                    offset,
                    min_length,
                    from_end: false,
                } => ir::Projection::ConstantIndex {
                    offset: *offset,
                    min_length: *min_length,
                },
                pmir::ProjectionElem::Downcast(v) => match ty.kind() {
                    pty::TyKind::RigidTy(pty::RigidTy::Adt(def, _)) => {
                        ir::Projection::Downcast(def.variant(*v)?.name())
                    }
                    _ => return None,
                },
                // `[-1 of 3]`, `[1:]`, `[1..3]` don't parse; an opaque type's cast (`(_1 as T)`) isn't modelled
                pmir::ProjectionElem::ConstantIndex { from_end: true, .. }
                | pmir::ProjectionElem::Subslice { .. }
                | pmir::ProjectionElem::OpaqueCast(_) => return None,
            });
            ty = e.ty(ty).ok()?;
        }
        Some(ir::Place {
            base: ir::PlaceBase::Local(ir::Local(p.local as u32)),
            projection,
        })
    }

    fn operand(&self, o: &pmir::Operand, locals: &[pmir::LocalDecl]) -> Option<ir::Operand> {
        match o {
            pmir::Operand::Copy(p) => self.place(p, locals).map(ir::Operand::Copy),
            pmir::Operand::Move(p) => self.place(p, locals).map(ir::Operand::Move),
            pmir::Operand::Constant(c) => self.const_operand(&c.const_),
            // `UbChecks`, which doesn't parse
            pmir::Operand::RuntimeChecks(_) => None,
        }
    }

    /// A constant as an operand. A function's (`foo`, a zero-sized value) prints without `const`, which the parser
    /// reads only as the function called.
    fn const_operand(&self, c: &pty::MirConst) -> Option<ir::Operand> {
        if matches!(c.ty().kind(), pty::TyKind::RigidTy(pty::RigidTy::FnDef(..))) {
            return None;
        }
        self.constant(c).map(ir::Operand::Constant)
    }

    /// A constant's value: an integer, a bool or `()` from its bits; anything else as rustc prints it, read by the
    /// parser (a struct's value, a named constant, a string).
    fn constant(&self, c: &pty::MirConst) -> Option<ir::Const> {
        use pty::RigidTy as R;
        let bits = || match c.kind() {
            pty::ConstantKind::Allocated(a) => a.read_uint().ok(),
            pty::ConstantKind::Ty(t) => match t.kind() {
                pty::TyConstKind::Value(_, a) => a.read_uint().ok(),
                _ => None,
            },
            _ => None,
        };
        let structural = match c.ty().kind() {
            pty::TyKind::RigidTy(R::Int(t)) => bits().and_then(|b| int_const(b, int_ty(t))),
            pty::TyKind::RigidTy(R::Uint(t)) => bits().and_then(|b| int_const(b, uint_ty(t))),
            pty::TyKind::RigidTy(R::Bool) => bits().and_then(|b| match b {
                0 => Some(ir::Const::Bool(false)),
                1 => Some(ir::Const::Bool(true)),
                _ => None,
            }),
            pty::TyKind::RigidTy(R::Char) => bits().and_then(|b| char::from_u32(b as u32)).map(ir::Const::Char),
            pty::TyKind::RigidTy(R::Tuple(ts)) if ts.is_empty() => Some(ir::Const::Unit),
            _ => None,
        };
        if structural.is_some() {
            return structural;
        }
        let konst: mmir::Const<'tcx> = internal(self.tcx, c);
        // a pointer to a static, as the text names it, `{alloc1: &[i16; 4]}`: the text gives `&[u8; N]`'s as
        // `<static(DefId(..))>`, which it can't read, so this is one place the two differ
        if let mmir::Const::Val(mmir::ConstValue::Scalar(mmir::interpret::Scalar::Ptr(ptr, _)), ty) = konst {
            let alloc = ptr.provenance.alloc_id();
            if let Some(mmir::interpret::GlobalAlloc::Static(_)) = self.tcx.try_get_global_alloc(alloc) {
                return Some(ir::Const::Alloc(alloc.to_string(), self.ty(stable(ty), Paths::Trimmed)));
            }
        }
        match ir::parse_operand(&self.full(format!("const {konst}"))) {
            Ok(ir::Operand::Constant(c)) => Some(c),
            _ => None,
        }
    }

    // ---- types ----

    /// A type, as the parser reads it: one it can't read is kept as rustc printed it.
    fn ty(&self, t: pty::Ty, paths: Paths) -> ir::Ty {
        self.ty_read(t, paths)
            .unwrap_or_else(|| ir::Ty::Other(self.ty_text(t, paths)))
    }

    /// rustc's text of a type.
    fn ty_text(&self, t: pty::Ty, paths: Paths) -> String {
        let t: mty::Ty<'tcx> = internal(self.tcx, t);
        match paths {
            Paths::Trimmed => self.trimmed(t.to_string()),
            Paths::Full => {
                let _full = NoTrimmedGuard::new();
                self.full(t.to_string())
            }
        }
    }

    /// A type as the parser reads it, or none if it can't (a closure's, a function pointer's): within a place or a
    /// cast, the line is then unread. Its shape is rustc_public's; a named type (a struct, `Range<i16>`, a type
    /// parameter) is read from rustc's text, whose paths the IR keeps.
    fn ty_read(&self, t: pty::Ty, paths: Paths) -> Option<ir::Ty> {
        use pty::RigidTy as R;
        let pty::TyKind::RigidTy(r) = t.kind() else {
            return ir::parse_ty(&self.ty_text(t, paths)).ok();
        };
        Some(match r {
            R::Bool => ir::Ty::Bool,
            R::Char => ir::Ty::Char,
            R::Str => ir::Ty::Str,
            R::Never => ir::Ty::Never,
            R::Int(i) => ir::Ty::Int(int_ty(i)),
            R::Uint(u) => ir::Ty::Int(uint_ty(u)),
            R::Tuple(ts) if ts.is_empty() => ir::Ty::Unit,
            R::Tuple(ts) => ir::Ty::Tuple(ts.iter().map(|t| self.ty_read(*t, paths)).collect::<Option<_>>()?),
            R::Array(e, n) => {
                let len = match n.kind() {
                    pty::TyConstKind::Value(..) => ir::ArrayLen::Known(n.eval_target_usize().ok()?),
                    pty::TyConstKind::Param(p) => ir::ArrayLen::Param(p.name.clone()),
                    _ => return ir::parse_ty(&self.ty_text(t, paths)).ok(),
                };
                ir::Ty::Array(Box::new(self.ty_read(e, paths)?), len)
            }
            R::Slice(e) => ir::Ty::Slice(Box::new(self.ty_read(e, paths)?)),
            R::Ref(region, to, m) => {
                let lifetime = match region.kind {
                    pty::RegionKind::ReErased => None,
                    pty::RegionKind::ReStatic => Some("'static".to_string()),
                    _ => return ir::parse_ty(&self.ty_text(t, paths)).ok(),
                };
                ir::Ty::Ref {
                    lifetime,
                    mutable: m == pmir::Mutability::Mut,
                    to: Box::new(self.ty_read(to, paths)?),
                }
            }
            R::RawPtr(to, m) => ir::Ty::Ptr {
                mutable: m == pmir::Mutability::Mut,
                to: Box::new(self.ty_read(to, paths)?),
            },
            _ => return ir::parse_ty(&self.ty_text(t, paths)).ok(),
        })
    }
}

fn int_ty(t: pty::IntTy) -> ir::IntTy {
    match t {
        pty::IntTy::Isize => ir::IntTy::Isize,
        pty::IntTy::I8 => ir::IntTy::I8,
        pty::IntTy::I16 => ir::IntTy::I16,
        pty::IntTy::I32 => ir::IntTy::I32,
        pty::IntTy::I64 => ir::IntTy::I64,
        pty::IntTy::I128 => ir::IntTy::I128,
    }
}

fn uint_ty(t: pty::UintTy) -> ir::IntTy {
    match t {
        pty::UintTy::Usize => ir::IntTy::Usize,
        pty::UintTy::U8 => ir::IntTy::U8,
        pty::UintTy::U16 => ir::IntTy::U16,
        pty::UintTy::U32 => ir::IntTy::U32,
        pty::UintTy::U64 => ir::IntTy::U64,
        pty::UintTy::U128 => ir::IntTy::U128,
    }
}

/// An integer constant from its bits, as the parser reads what rustc prints for it. rustc prints a type's limits by
/// name (`i16::MAX`, `u8::MAX`, `i8::MIN`; an unsigned 0 is `0_u8`, not `u8::MIN`), and the parser reads one a card
/// holds as its number (`u8::MAX` is 255_u8) and keeps the others as names (`ir::limit_on_a_card`). A value past an
/// i128 (a u128's) doesn't parse.
/// The bytes an integer of this type takes.
fn int_size(ty: ir::IntTy) -> usize {
    use ir::IntTy as I;
    match ty {
        I::I8 | I::U8 => 1,
        I::I16 | I::U16 => 2,
        I::I32 | I::U32 => 4,
        I::I64 | I::U64 | I::Isize | I::Usize => 8,
        I::I128 | I::U128 => 16,
    }
}

fn int_const(bits: u128, ty: ir::IntTy) -> Option<ir::Const> {
    use ir::IntTy as I;
    let width = match ty {
        I::I8 | I::U8 => 8,
        I::I16 | I::U16 => 16,
        I::I32 | I::U32 => 32,
        I::I64 | I::U64 | I::Isize | I::Usize => 64,
        I::I128 | I::U128 => 128,
    };
    let signed = matches!(ty, I::I8 | I::I16 | I::I32 | I::I64 | I::I128 | I::Isize);
    let mask = if width == 128 { u128::MAX } else { (1u128 << width) - 1 };
    let bits = bits & mask;
    let limit = if signed {
        let min = 1u128 << (width - 1);
        (bits == min).then_some("MIN").or((bits == min - 1).then_some("MAX"))
    } else {
        (bits == mask).then_some("MAX")
    };
    if let Some(which) = limit {
        let path = ir::Path {
            qself: None,
            segments: [ty.name(), which]
                .iter()
                .map(|s| ir::Segment::Name {
                    name: s.to_string(),
                    args: None,
                })
                .collect(),
        };
        return Some(ir::limit_on_a_card(&path).unwrap_or(ir::Const::Path(path)));
    }
    let value = if signed && width < 128 && bits >= 1u128 << (width - 1) {
        bits as i128 - (1i128 << width)
    } else if signed {
        bits as i128
    } else {
        i128::try_from(bits).ok()?
    };
    Some(ir::Const::Int { value, ty })
}

/// A path of one name, `Cmp`.
fn name_path(name: &str) -> ir::Path {
    ir::Path {
        qself: None,
        segments: vec![ir::Segment::Name {
            name: name.to_string(),
            args: None,
        }],
    }
}

fn bin_op(op: pmir::BinOp) -> Option<ir::BinOp> {
    use pmir::BinOp as B;
    Some(match op {
        B::Add => ir::BinOp::Add,
        B::AddUnchecked => ir::BinOp::AddUnchecked,
        B::Sub => ir::BinOp::Sub,
        B::SubUnchecked => ir::BinOp::SubUnchecked,
        B::Mul => ir::BinOp::Mul,
        B::MulUnchecked => ir::BinOp::MulUnchecked,
        B::Div => ir::BinOp::Div,
        B::Rem => ir::BinOp::Rem,
        B::BitXor => ir::BinOp::BitXor,
        B::BitAnd => ir::BinOp::BitAnd,
        B::BitOr => ir::BinOp::BitOr,
        B::Shl => ir::BinOp::Shl,
        B::Shr => ir::BinOp::Shr,
        B::Eq => ir::BinOp::Eq,
        B::Lt => ir::BinOp::Lt,
        B::Le => ir::BinOp::Le,
        B::Ne => ir::BinOp::Ne,
        B::Ge => ir::BinOp::Ge,
        B::Gt => ir::BinOp::Gt,
        B::Offset => ir::BinOp::Offset,
        B::ShlUnchecked | B::ShrUnchecked | B::Cmp => return None,
    })
}

/// A cast's kind. A pointer coercion's source (`Implicit`, `AsCast`) is rustc's MIR's: rustc_public leaves it out.
fn cast_kind(kind: &pmir::CastKind, internal: &mmir::CastKind) -> ir::CastKind {
    use pmir::CastKind as C;
    match kind {
        C::IntToInt => ir::CastKind::IntToInt,
        C::IntToFloat => ir::CastKind::IntToFloat,
        C::FloatToInt => ir::CastKind::FloatToInt,
        C::FloatToFloat => ir::CastKind::FloatToFloat,
        C::PtrToPtr => ir::CastKind::PtrToPtr,
        C::FnPtrToPtr => ir::CastKind::FnPtrToPtr,
        C::Transmute => ir::CastKind::Transmute,
        C::Subtype => ir::CastKind::Subtype,
        C::PointerExposeAddress => ir::CastKind::PointerExposeProvenance,
        C::PointerWithExposedProvenance => ir::CastKind::PointerWithExposedProvenance,
        C::PointerCoercion(pmir::PointerCoercion::Unsize) => match internal {
            mmir::CastKind::PointerCoercion(_, mmir::CoercionSource::Implicit) => {
                ir::CastKind::Unsize(ir::CoercionSource::Implicit)
            }
            mmir::CastKind::PointerCoercion(_, mmir::CoercionSource::AsCast) => {
                ir::CastKind::Unsize(ir::CoercionSource::AsCast)
            }
            other => ir::CastKind::Other(format!("{other:?}")),
        },
        // another coercion, as rustc names it
        C::PointerCoercion(_) => ir::CastKind::Other(format!("{internal:?}")),
    }
}

/// An assert's message, as MIR prints it, and the operands it names.
fn assert_message(m: &pmir::AssertMessage) -> Option<(&'static str, Vec<&pmir::Operand>)> {
    use pmir::AssertMessage as M;
    use pmir::BinOp as B;
    Some(match m {
        M::BoundsCheck { len, index } => (
            "index out of bounds: the length is {} but the index is {}",
            vec![len, index],
        ),
        M::OverflowNeg(o) => ("attempt to negate `{}`, which would overflow", vec![o]),
        M::DivisionByZero(o) => ("attempt to divide `{}` by zero", vec![o]),
        M::RemainderByZero(o) => (
            "attempt to calculate the remainder of `{}` with a divisor of zero",
            vec![o],
        ),
        M::Overflow(B::Add, l, r) => ("attempt to compute `{} + {}`, which would overflow", vec![l, r]),
        M::Overflow(B::Sub, l, r) => ("attempt to compute `{} - {}`, which would overflow", vec![l, r]),
        M::Overflow(B::Mul, l, r) => ("attempt to compute `{} * {}`, which would overflow", vec![l, r]),
        M::Overflow(B::Div, l, r) => ("attempt to compute `{} / {}`, which would overflow", vec![l, r]),
        M::Overflow(B::Rem, l, r) => (
            "attempt to compute the remainder of `{} % {}`, which would overflow",
            vec![l, r],
        ),
        M::Overflow(B::Shr, _, r) => ("attempt to shift right by `{}`, which would overflow", vec![r]),
        M::Overflow(B::Shl, _, r) => ("attempt to shift left by `{}`, which would overflow", vec![r]),
        M::MisalignedPointerDereference { required, found } => (
            "misaligned pointer dereference: address must be a multiple of {} but is {}",
            vec![required, found],
        ),
        M::NullPointerDereference => ("null pointer dereference occurred", vec![]),
        M::InvalidEnumConstruction(o) => ("trying to construct an enum from an invalid value {}", vec![o]),
        _ => return None,
    })
}
