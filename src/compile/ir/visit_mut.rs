//! A visitor that can change what it visits, in the manner of rustc's `MutVisitor`: each `visit_*` method does its
//! own work and then calls the matching `walk_*` function, which visits the parts. A transformation overrides the
//! methods for the nodes it changes and leaves the rest to the walk.
//!
//! An `Unsupported` line is not visited: it is kept as rustc wrote it, for the error message that names it.
use super::*;

pub trait VisitMut {
    fn visit_body(&mut self, b: &mut Body) {
        walk_body(self, b)
    }

    fn visit_block(&mut self, b: &mut Block) {
        walk_block(self, b)
    }

    fn visit_statement(&mut self, s: &mut Statement) {
        walk_statement(self, s)
    }

    fn visit_terminator(&mut self, t: &mut Terminator) {
        walk_terminator(self, t)
    }

    fn visit_rvalue(&mut self, r: &mut Rvalue) {
        walk_rvalue(self, r)
    }

    fn visit_operand(&mut self, o: &mut Operand) {
        walk_operand(self, o)
    }

    fn visit_place(&mut self, p: &mut Place) {
        walk_place(self, p)
    }

    fn visit_const(&mut self, c: &mut Const) {
        walk_const(self, c)
    }

    fn visit_ty(&mut self, t: &mut Ty) {
        walk_ty(self, t)
    }

    fn visit_path(&mut self, p: &mut Path) {
        walk_path(self, p)
    }
}

pub fn walk_body<V: VisitMut + ?Sized>(v: &mut V, b: &mut Body) {
    for d in &mut b.decls {
        v.visit_ty(&mut d.ty);
    }
    for block in &mut b.blocks {
        v.visit_block(block);
    }
}

pub fn walk_block<V: VisitMut + ?Sized>(v: &mut V, b: &mut Block) {
    for s in &mut b.statements {
        v.visit_statement(s);
    }
    v.visit_terminator(&mut b.terminator);
}

pub fn walk_statement<V: VisitMut + ?Sized>(v: &mut V, s: &mut Statement) {
    match s {
        Statement::Assign(p, r) => {
            v.visit_place(p);
            v.visit_rvalue(r);
        }
        Statement::Assume(o) => v.visit_operand(o),
        Statement::StorageLive(_) | Statement::StorageDead(_) | Statement::Nop | Statement::Unsupported(_) => {}
    }
}

pub fn walk_terminator<V: VisitMut + ?Sized>(v: &mut V, t: &mut Terminator) {
    match t {
        Terminator::SwitchInt { discr, .. } => v.visit_operand(discr),
        Terminator::Call { dest, func, args, .. } => {
            v.visit_place(dest);
            v.visit_path(func);
            for a in args {
                v.visit_operand(a);
            }
        }
        Terminator::Assert { cond, msg_args, .. } => {
            v.visit_operand(cond);
            for a in msg_args {
                v.visit_operand(a);
            }
        }
        Terminator::Drop { place, .. } => v.visit_place(place),
        Terminator::Goto(_) | Terminator::Return | Terminator::Unreachable | Terminator::Unsupported(_) => {}
    }
}

pub fn walk_rvalue<V: VisitMut + ?Sized>(v: &mut V, r: &mut Rvalue) {
    match r {
        Rvalue::Use(o) | Rvalue::UnaryOp(_, o) | Rvalue::Repeat(o, _) => v.visit_operand(o),
        Rvalue::BinaryOp(_, a, b) => {
            v.visit_operand(a);
            v.visit_operand(b);
        }
        Rvalue::Cast(o, t, _) => {
            v.visit_operand(o);
            v.visit_ty(t);
        }
        Rvalue::Ref(_, p) | Rvalue::Discriminant(p) => v.visit_place(p),
        Rvalue::Array(os) | Rvalue::Tuple(os) => {
            for o in os {
                v.visit_operand(o);
            }
        }
        Rvalue::Struct(p, fields) => {
            v.visit_path(p);
            for (_, o) in fields {
                v.visit_operand(o);
            }
        }
        Rvalue::Variant(p, os) => {
            v.visit_path(p);
            for o in os {
                v.visit_operand(o);
            }
        }
        Rvalue::Sum(terms) => {
            for (_, o) in terms {
                v.visit_operand(o);
            }
        }
        Rvalue::Lin(terms) => {
            for (_, o) in terms {
                v.visit_operand(o);
            }
        }
    }
}

pub fn walk_operand<V: VisitMut + ?Sized>(v: &mut V, o: &mut Operand) {
    match o {
        Operand::Copy(p) | Operand::Move(p) | Operand::Place(p) => v.visit_place(p),
        Operand::Constant(c) => v.visit_const(c),
        Operand::Aggregate(p, fields) => {
            v.visit_path(p);
            for (_, o) in fields {
                v.visit_operand(o);
            }
        }
    }
}

pub fn walk_place<V: VisitMut + ?Sized>(v: &mut V, p: &mut Place) {
    for proj in &mut p.projection {
        match proj {
            Projection::Field(_, t) => v.visit_ty(t),
            Projection::IndexPlace(index) => v.visit_place(index),
            Projection::Deref
            | Projection::Index(_)
            | Projection::IndexPlus(..)
            | Projection::ConstantIndex { .. }
            | Projection::ConstantIndexFromEnd { .. }
            | Projection::Subslice { .. }
            | Projection::Downcast(_) => {}
        }
    }
}

pub fn walk_const<V: VisitMut + ?Sized>(v: &mut V, c: &mut Const) {
    match c {
        Const::Path(p) => v.visit_path(p),
        Const::Adt(p, fields) => {
            v.visit_path(p);
            for (_, c) in fields {
                v.visit_const(c);
            }
        }
        Const::Variant(p, cs) => {
            v.visit_path(p);
            for c in cs {
                v.visit_const(c);
            }
        }
        // (a promoted's path names the function it was promoted from, as the item's does: not a value's path)
        Const::Int { .. }
        | Const::Bool(_)
        | Const::Char(_)
        | Const::Unit
        | Const::Str(_)
        | Const::Bytes(_)
        | Const::Promoted(..) => {}
        Const::Alloc(_, t) => v.visit_ty(t),
    }
}

pub fn walk_ty<V: VisitMut + ?Sized>(v: &mut V, t: &mut Ty) {
    match t {
        Ty::Tuple(ts) => {
            for t in ts {
                v.visit_ty(t);
            }
        }
        Ty::Array(t, _) | Ty::Slice(t) | Ty::Ref { to: t, .. } | Ty::Ptr { to: t, .. } => v.visit_ty(t),
        Ty::Path(p) | Ty::Dyn(p) => v.visit_path(p),
        Ty::Int(_) | Ty::Bool | Ty::Char | Ty::Str | Ty::Unit | Ty::Never | Ty::Other(_) => {}
    }
}

pub fn walk_path<V: VisitMut + ?Sized>(v: &mut V, p: &mut Path) {
    if let Some(q) = &mut p.qself {
        v.visit_ty(&mut q.ty);
        if let Some(t) = &mut q.as_trait {
            v.visit_path(t);
        }
    }
    for s in &mut p.segments {
        match s {
            Segment::Name { args: Some(args), .. } => {
                for a in &mut args.args {
                    if let GenericArg::Ty(t) = a {
                        v.visit_ty(t);
                    }
                }
            }
            Segment::Impl(t) => v.visit_ty(t),
            Segment::Name { args: None, .. } => {}
        }
    }
}
