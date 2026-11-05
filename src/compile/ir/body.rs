//! Questions about a function's blocks as a whole.
use std::collections::HashMap;

use super::*;

/// The block named `b`.
pub fn find(blocks: &[Block], b: BasicBlock) -> Option<usize> {
    blocks.iter().position(|x| x.name == b)
}

/// How many lines, in all the blocks, name `l`.
pub fn lines_naming(blocks: &[Block], l: Local) -> usize {
    blocks.iter().map(|b| b.lines_naming(l)).sum()
}

/// How many times `l` is named, in all the blocks.
pub fn occurrences(blocks: &[Block], l: Local) -> usize {
    blocks.iter().flat_map(|b| b.lines()).map(|x| x.count(l)).sum()
}

/// How many blocks go to `b`.
pub fn blocks_naming(blocks: &[Block], b: BasicBlock) -> usize {
    blocks.iter().filter(|x| x.terminator.names(b)).count()
}

/// For each block, the blocks that go to it (each counted once per block), in block order.
pub fn predecessors(blocks: &[Block]) -> HashMap<BasicBlock, Vec<BasicBlock>> {
    let mut preds: HashMap<BasicBlock, Vec<BasicBlock>> = HashMap::new();
    for b in blocks {
        let mut seen = vec![];
        for s in b.terminator.successors() {
            if !seen.contains(&s) {
                seen.push(s);
                preds.entry(s).or_default().push(b.name);
            }
        }
    }
    preds
}

impl Statement {
    /// The whole local it assigns, `x = ...`.
    pub fn def(&self) -> Option<Local> {
        match self {
            Statement::Assign(p, _) => p.as_local().filter(|l| *l != Local::UNIT),
            _ => None,
        }
    }
}

impl Terminator {
    /// The whole local a call assigns, `x = f(..) -> ..`.
    pub fn def(&self) -> Option<Local> {
        match self {
            Terminator::Call { dest, .. } => dest.as_local().filter(|l| *l != Local::UNIT),
            _ => None,
        }
    }
}

impl Block {
    /// For each line, the whole local it assigns.
    pub fn defs(&self) -> Vec<Option<Local>> {
        self.statements
            .iter()
            .map(Statement::def)
            .chain(std::iter::once(self.terminator.def()))
            .collect()
    }
}

impl Place {
    /// How many projections wrap the place in parentheses when printed (a deref, a field, a downcast).
    pub fn wrapped(&self) -> usize {
        self.projection
            .iter()
            .filter(|p| matches!(p, Projection::Deref | Projection::Field(..) | Projection::Downcast(_)))
            .count()
    }

    /// The place `(l.k: ty)`.
    pub fn field(l: Local, k: u32, ty: Ty) -> Place {
        Place {
            base: PlaceBase::Local(l),
            projection: vec![Projection::Field(k, ty)],
        }
    }

    /// Is it `(l.0: _)`, or that field projected further?
    pub fn is_value_of(&self, l: Local) -> bool {
        self.base == PlaceBase::Local(l) && matches!(self.projection.first(), Some(Projection::Field(0, _)))
    }
}
