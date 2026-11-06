//! What a piece of IR names: its locals, in the order they are printed, and the blocks a terminator goes to.
use super::*;

/// Something that names locals.
pub trait Locals {
    /// Each local named, in printed order, as often as it is named.
    fn visit_locals(&self, f: &mut dyn FnMut(Local));

    fn locals(&self) -> Vec<Local> {
        let mut v = vec![];
        self.visit_locals(&mut |l| v.push(l));
        v
    }

    fn mentions(&self, l: Local) -> bool {
        let mut found = false;
        self.visit_locals(&mut |x| found |= x == l);
        found
    }

    /// How many times `l` is named.
    fn count(&self, l: Local) -> usize {
        let mut n = 0;
        self.visit_locals(&mut |x| n += (x == l) as usize);
        n
    }
}

impl Locals for Place {
    fn visit_locals(&self, f: &mut dyn FnMut(Local)) {
        if let PlaceBase::Local(l) = self.base {
            f(l);
        }
        for p in &self.projection {
            match p {
                Projection::Index(i) | Projection::IndexPlus(i, _) => f(*i),
                Projection::IndexPlace(q) => q.visit_locals(f),
                _ => {}
            }
        }
    }
}

impl Locals for Operand {
    fn visit_locals(&self, f: &mut dyn FnMut(Local)) {
        match self {
            Operand::Copy(p) | Operand::Move(p) | Operand::Place(p) => p.visit_locals(f),
            Operand::Constant(_) | Operand::Fn(_) => {}
            Operand::Aggregate(_, fields) => fields.iter().for_each(|(_, o)| o.visit_locals(f)),
        }
    }
}

impl Locals for Rvalue {
    fn visit_locals(&self, f: &mut dyn FnMut(Local)) {
        match self {
            Rvalue::Use(o) | Rvalue::UnaryOp(_, o) | Rvalue::Cast(o, ..) | Rvalue::Repeat(o, _) => o.visit_locals(f),
            Rvalue::BinaryOp(_, a, b) => {
                a.visit_locals(f);
                b.visit_locals(f);
            }
            Rvalue::Ref(_, p) | Rvalue::Discriminant(p) => p.visit_locals(f),
            Rvalue::Array(os) | Rvalue::Tuple(os) | Rvalue::Variant(_, os) => os.iter().for_each(|o| o.visit_locals(f)),
            Rvalue::Struct(_, fields) => fields.iter().for_each(|(_, o)| o.visit_locals(f)),
            Rvalue::Sum(terms) => terms.iter().for_each(|(_, o)| o.visit_locals(f)),
            Rvalue::Lin(terms) => terms.iter().for_each(|(_, o)| o.visit_locals(f)),
        }
    }
}

impl Locals for Statement {
    fn visit_locals(&self, f: &mut dyn FnMut(Local)) {
        match self {
            Statement::Assign(p, r) => {
                p.visit_locals(f);
                r.visit_locals(f);
            }
            Statement::Assume(o) => o.visit_locals(f),
            Statement::StorageLive(l) | Statement::StorageDead(l) => f(*l),
            Statement::Nop => {}
            Statement::Unsupported(u) => u.locals.iter().for_each(|l| f(*l)),
        }
    }
}

impl Locals for Terminator {
    fn visit_locals(&self, f: &mut dyn FnMut(Local)) {
        match self {
            Terminator::SwitchInt { discr, .. } => discr.visit_locals(f),
            Terminator::Call { dest, args, .. } => {
                dest.visit_locals(f);
                args.iter().for_each(|a| a.visit_locals(f));
            }
            Terminator::CallIndirect { dest, func, args, .. } => {
                dest.visit_locals(f);
                func.visit_locals(f);
                args.iter().for_each(|a| a.visit_locals(f));
            }
            Terminator::Assert { cond, msg_args, .. } => {
                cond.visit_locals(f);
                msg_args.iter().for_each(|a| a.visit_locals(f));
            }
            Terminator::Drop { place, .. } => place.visit_locals(f),
            Terminator::Unsupported(u) => u.locals.iter().for_each(|l| f(*l)),
            Terminator::Goto(_) | Terminator::Return | Terminator::Unreachable => {}
        }
    }
}

impl Terminator {
    /// The blocks it can go to, in the order it names them (an unwind's cleanup block too).
    pub fn successors(&self) -> Vec<BasicBlock> {
        let cleanup = |u: &Unwind| match u {
            Unwind::Cleanup(b) => Some(*b),
            _ => None,
        };
        match self {
            Terminator::Goto(b) => vec![*b],
            Terminator::SwitchInt { arms, otherwise, .. } => arms.iter().map(|(_, b)| *b).chain(*otherwise).collect(),
            Terminator::Call { target, unwind, .. } | Terminator::CallIndirect { target, unwind, .. } => {
                target.iter().copied().chain(cleanup(unwind)).collect()
            }
            Terminator::Assert { target, unwind, .. } | Terminator::Drop { target, unwind, .. } => {
                std::iter::once(*target).chain(cleanup(unwind)).collect()
            }
            Terminator::Return | Terminator::Unreachable | Terminator::Unsupported(_) => vec![],
        }
    }

    /// Does it name block `b`?
    pub fn names(&self, b: BasicBlock) -> bool {
        self.successors().contains(&b)
    }
}

impl Block {
    /// Each line's locals: the statements', then the terminator's.
    pub fn lines(&self) -> impl Iterator<Item = &dyn Locals> {
        self.statements
            .iter()
            .map(|s| s as &dyn Locals)
            .chain(std::iter::once(&self.terminator as &dyn Locals))
    }

    /// How many of its lines name `l`.
    pub fn lines_naming(&self, l: Local) -> usize {
        self.lines().filter(|x| x.mentions(l)).count()
    }
}

impl Rvalue {
    /// Each operand, in printed order.
    pub fn operands(&self) -> Vec<&Operand> {
        match self {
            Rvalue::Use(o) | Rvalue::UnaryOp(_, o) | Rvalue::Cast(o, ..) | Rvalue::Repeat(o, _) => vec![o],
            Rvalue::BinaryOp(_, a, b) => vec![a, b],
            Rvalue::Ref(..) | Rvalue::Discriminant(_) => vec![],
            Rvalue::Array(os) | Rvalue::Tuple(os) | Rvalue::Variant(_, os) => os.iter().collect(),
            Rvalue::Struct(_, fields) => fields.iter().map(|(_, o)| o).collect(),
            Rvalue::Sum(terms) => terms.iter().map(|(_, o)| o).collect(),
            Rvalue::Lin(terms) => terms.iter().map(|(_, o)| o).collect(),
        }
    }
}

impl Statement {
    /// Each operand, in printed order.
    pub fn operands(&self) -> Vec<&Operand> {
        match self {
            Statement::Assign(_, r) => r.operands(),
            Statement::Assume(o) => vec![o],
            _ => vec![],
        }
    }
}

impl Terminator {
    /// Each operand, in printed order.
    pub fn operands(&self) -> Vec<&Operand> {
        match self {
            Terminator::SwitchInt { discr, .. } => vec![discr],
            Terminator::Call { args, .. } => args.iter().collect(),
            Terminator::CallIndirect { func, args, .. } => std::iter::once(func).chain(args).collect(),
            Terminator::Assert { cond, msg_args, .. } => std::iter::once(cond).chain(msg_args).collect(),
            _ => vec![],
        }
    }
}

impl Operand {
    /// The place read, whether copied, moved or named.
    pub fn place(&self) -> Option<&Place> {
        match self {
            Operand::Copy(p) | Operand::Move(p) | Operand::Place(p) => Some(p),
            Operand::Constant(_) | Operand::Aggregate(..) | Operand::Fn(_) => None,
        }
    }

    /// The local read, when the operand reads a whole local.
    pub fn local(&self) -> Option<Local> {
        self.place().and_then(Place::as_local)
    }

    /// An integer constant's value.
    pub fn int(&self) -> Option<(i128, IntTy)> {
        match self {
            Operand::Constant(Const::Int { value, ty }) => Some((*value, *ty)),
            _ => None,
        }
    }
}

impl Rvalue {
    fn places_mut(&mut self, f: &mut dyn FnMut(&mut Place)) {
        match self {
            Rvalue::Ref(_, p) | Rvalue::Discriminant(p) => f(p),
            r => {
                for o in r.operands_mut() {
                    if let Some(p) = o.place_mut() {
                        f(p);
                    }
                }
            }
        }
    }

    fn operands_mut(&mut self) -> Vec<&mut Operand> {
        match self {
            Rvalue::Use(o) | Rvalue::UnaryOp(_, o) | Rvalue::Cast(o, ..) | Rvalue::Repeat(o, _) => vec![o],
            Rvalue::BinaryOp(_, a, b) => vec![a, b],
            Rvalue::Ref(..) | Rvalue::Discriminant(_) => vec![],
            Rvalue::Array(os) | Rvalue::Tuple(os) | Rvalue::Variant(_, os) => os.iter_mut().collect(),
            Rvalue::Struct(_, fields) => fields.iter_mut().map(|(_, o)| o).collect(),
            Rvalue::Sum(terms) => terms.iter_mut().map(|(_, o)| o).collect(),
            Rvalue::Lin(terms) => terms.iter_mut().map(|(_, o)| o).collect(),
        }
    }

    fn places(&self) -> Vec<&Place> {
        match self {
            Rvalue::Ref(_, p) | Rvalue::Discriminant(p) => vec![p],
            r => r.operands().into_iter().filter_map(Operand::place).collect(),
        }
    }
}

impl Operand {
    pub fn place_mut(&mut self) -> Option<&mut Place> {
        match self {
            Operand::Copy(p) | Operand::Move(p) | Operand::Place(p) => Some(p),
            Operand::Constant(_) | Operand::Aggregate(..) | Operand::Fn(_) => None,
        }
    }
}

impl Statement {
    /// Every place it names: what it assigns, then what it reads.
    pub fn places(&self) -> Vec<&Place> {
        match self {
            Statement::Assign(p, r) => std::iter::once(p).chain(r.places()).collect(),
            Statement::Assume(o) => o.place().into_iter().collect(),
            _ => vec![],
        }
    }

    pub fn places_mut(&mut self, f: &mut dyn FnMut(&mut Place)) {
        match self {
            Statement::Assign(p, r) => {
                f(p);
                r.places_mut(f);
            }
            Statement::Assume(o) => {
                if let Some(p) = o.place_mut() {
                    f(p);
                }
            }
            _ => {}
        }
    }
}

impl Terminator {
    /// Every place it names: a call's destination, then what it reads.
    pub fn places(&self) -> Vec<&Place> {
        let dest = match self {
            Terminator::Call { dest, .. } | Terminator::CallIndirect { dest, .. } => Some(dest),
            Terminator::Drop { place, .. } => Some(place),
            _ => None,
        };
        dest.into_iter()
            .chain(self.operands().into_iter().filter_map(Operand::place))
            .collect()
    }

    pub fn places_mut(&mut self, f: &mut dyn FnMut(&mut Place)) {
        match self {
            Terminator::Call { dest, args, .. } => {
                f(dest);
                args.iter_mut().filter_map(Operand::place_mut).for_each(f);
            }
            Terminator::CallIndirect { dest, func, args, .. } => {
                f(dest);
                func.place_mut().into_iter().for_each(&mut *f);
                args.iter_mut().filter_map(Operand::place_mut).for_each(f);
            }
            Terminator::SwitchInt { discr, .. } => discr.place_mut().into_iter().for_each(f),
            Terminator::Drop { place, .. } => f(place),
            Terminator::Assert { cond, msg_args, .. } => {
                cond.place_mut().into_iter().for_each(&mut *f);
                msg_args.iter_mut().filter_map(Operand::place_mut).for_each(f);
            }
            _ => {}
        }
    }

    /// A call's destination, function and arguments.
    pub fn as_call(&self) -> Option<(&Place, &Path, &[Operand])> {
        match self {
            Terminator::Call { dest, func, args, .. } => Some((dest, func, args)),
            _ => None,
        }
    }
}

impl Path {
    /// Is it the one-word path `name`, with no generic arguments?
    pub fn is(&self, name: &str) -> bool {
        self.is_path(&[name])
    }
}

impl Statement {
    pub fn operands_mut(&mut self) -> Vec<&mut Operand> {
        match self {
            Statement::Assign(_, r) => r.operands_mut(),
            Statement::Assume(o) => vec![o],
            _ => vec![],
        }
    }
}

impl Terminator {
    pub fn operands_mut(&mut self) -> Vec<&mut Operand> {
        match self {
            Terminator::SwitchInt { discr, .. } => vec![discr],
            Terminator::Call { args, .. } => args.iter_mut().collect(),
            Terminator::CallIndirect { func, args, .. } => std::iter::once(func).chain(args.iter_mut()).collect(),
            Terminator::Assert { cond, msg_args, .. } => std::iter::once(cond).chain(msg_args.iter_mut()).collect(),
            _ => vec![],
        }
    }
}

impl Terminator {
    /// Each block it can go to, mutably (an unwind's cleanup block too).
    pub fn targets_mut(&mut self) -> Vec<&mut BasicBlock> {
        fn cleanup(u: &mut Unwind) -> Option<&mut BasicBlock> {
            match u {
                Unwind::Cleanup(b) => Some(b),
                _ => None,
            }
        }
        match self {
            Terminator::Goto(b) => vec![b],
            Terminator::SwitchInt { arms, otherwise, .. } => {
                arms.iter_mut().map(|(_, b)| b).chain(otherwise.as_mut()).collect()
            }
            Terminator::Call { target, unwind, .. } | Terminator::CallIndirect { target, unwind, .. } => {
                target.as_mut().into_iter().chain(cleanup(unwind)).collect()
            }
            Terminator::Assert { target, unwind, .. } | Terminator::Drop { target, unwind, .. } => {
                std::iter::once(target).chain(cleanup(unwind)).collect()
            }
            Terminator::Return | Terminator::Unreachable | Terminator::Unsupported(_) => vec![],
        }
    }
}
