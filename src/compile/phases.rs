//! A program with overlay() calls becomes a data block and one code block per phase, each assembled after the data
//! so they agree on every address. Each phase gets main's code from its start up to the next overlay(), plus every
//! function and routine that code reaches.
use std::collections::{HashMap, HashSet};

use super::ir;
use super::machine::{Arg, Kind, Line, Op};
use super::Fail;

/// Does the block end a phase, with a call to overlay()?
fn is_overlay(b: &ir::Block) -> bool {
    matches!(&b.terminator, ir::Terminator::Call { func, args, .. } if func.name() == "overlay" && args.is_empty())
}

/// Does main end a phase anywhere, with a call to overlay() of its own?
pub fn has_overlay(blocks: &[ir::Block]) -> bool {
    blocks.iter().any(is_overlay)
}

/// A place main can be cut into phases: on the way from one block to the next, or inside a block, before its
/// statement k.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cut {
    Way(ir::BasicBlock, ir::BasicBlock),
    Within(ir::BasicBlock, usize),
}

/// Where main could be cut into phases when it doesn't fit: each place every run passes, once, on its way to main's
/// return (a run that panics may stop short of it), and that is the only way into the rest. Main reaches what comes
/// after it only through it, and never comes back, so the phase before is never loaded again. A way between blocks
/// is such a place when, without it, main's entry reaches neither the block after it nor a return, and that block
/// doesn't lead back; a place between two statements of a block, when without the block main's entry reaches no
/// return and no block after it, and the block doesn't lead back to itself.
pub fn cut_points(blocks: &[ir::Block]) -> Vec<Cut> {
    let at: HashMap<ir::BasicBlock, usize> = blocks.iter().enumerate().map(|(i, b)| (b.name, i)).collect();
    // the blocks reached from `from`, never taking the way `skip` nor passing the block `avoid`
    let reached =
        |from: &[ir::BasicBlock], skip: Option<(ir::BasicBlock, ir::BasicBlock)>, avoid: Option<ir::BasicBlock>| {
            let mut seen: HashSet<ir::BasicBlock> = HashSet::new();
            let mut todo = from.to_vec();
            while let Some(b) = todo.pop() {
                let Some(&i) = at.get(&b) else { continue };
                if Some(b) == avoid || !seen.insert(b) {
                    continue;
                }
                for s in blocks[i].terminator.successors() {
                    if skip != Some((b, s)) {
                        todo.push(s);
                    }
                }
            }
            seen
        };
    let entry = ir::BasicBlock(0);
    let all = reached(&[entry], None, None);
    let returns = |seen: &HashSet<ir::BasicBlock>| {
        blocks
            .iter()
            .any(|b| seen.contains(&b.name) && matches!(b.terminator, ir::Terminator::Return))
    };
    let mut out = vec![];
    // a main that never returns (it always panics) has no way every run takes
    if !returns(&all) {
        return out;
    }
    for b in blocks.iter().filter(|b| all.contains(&b.name)) {
        let mut targets = b.terminator.successors();
        targets.sort();
        targets.dedup();
        let after = reached(&targets, None, None);
        let around = reached(&[entry], None, Some(b.name));
        if !after.contains(&b.name) && around.is_disjoint(&after) && !returns(&around) {
            // between two statements that do something, not around a marker
            let does = |s: &ir::Statement| {
                !matches!(
                    s,
                    ir::Statement::StorageLive(_) | ir::Statement::StorageDead(_) | ir::Statement::Nop
                )
            };
            for k in 1..b.statements.len() {
                if does(&b.statements[k - 1]) && does(&b.statements[k]) {
                    out.push(Cut::Within(b.name, k));
                }
            }
        }
        for t in targets {
            let way = (b.name, t);
            let around = reached(&[entry], Some(way), None);
            if at.contains_key(&t)
                && !around.contains(&t)
                && !returns(&around)
                && !reached(&[t], None, None).contains(&b.name)
            {
                out.push(Cut::Way(b.name, t));
            }
        }
    }
    out
}

/// Cut main at each of `cuts` (from `cut_points`): a block that calls overlay() comes between the two sides.
pub fn cut(blocks: &mut Vec<ir::Block>, cuts: &[Cut]) {
    let mut fresh = blocks.iter().map(|b| b.name.0).max().unwrap_or(0) + 1;
    let overlay = |to: ir::BasicBlock| ir::Terminator::Call {
        dest: ir::Place::local(ir::Local::UNIT),
        func: ir::Path {
            qself: None,
            segments: vec![ir::Segment::Name {
                name: "overlay".to_string(),
                args: None,
            }],
        },
        args: vec![],
        target: Some(to),
        unwind: ir::Unwind::Continue,
    };
    for &c in cuts {
        let from = match c {
            Cut::Way(from, _) | Cut::Within(from, _) => from,
        };
        let Some(i) = blocks.iter().position(|b| b.name == from) else {
            continue;
        };
        let name = ir::BasicBlock(fresh);
        fresh += 1;
        match c {
            // a block of its own on the way
            Cut::Way(_, to) => {
                for t in blocks[i].terminator.targets_mut() {
                    if *t == to {
                        *t = name;
                    }
                }
                blocks.push(ir::Block {
                    name,
                    statements: vec![],
                    terminator: overlay(to),
                });
            }
            // the block ends at the cut, and the rest of it is a block of its own
            Cut::Within(_, k) => {
                let b = &mut blocks[i];
                if k >= b.statements.len() {
                    continue;
                }
                let rest = b.statements.split_off(k);
                let terminator = std::mem::replace(&mut b.terminator, overlay(name));
                blocks.push(ir::Block {
                    name,
                    statements: rest,
                    terminator,
                });
            }
        }
    }
}

/// Put main's blocks in phase order before they are lowered: each phase's blocks are the ones its start reaches
/// without passing an overlay(), and they come in their own order, ending at the overlay() that ends the phase.
/// rustc numbers blocks in no particular order (an inlined function's blocks come last), so a block of the first
/// phase can come after the overlay() in its numbering. A block that two phases reach is copied into the later one,
/// so each phase has its own. Blocks no phase reaches are dropped.
pub fn order_phases(blocks: &mut Vec<ir::Block>) -> Result<(), Fail> {
    if !blocks.iter().any(is_overlay) {
        return Ok(());
    }
    let at: HashMap<ir::BasicBlock, usize> = blocks.iter().enumerate().map(|(i, b)| (b.name, i)).collect();
    let mut fresh = blocks.iter().map(|b| b.name.0).max().unwrap_or(0) + 1;
    let mut owned: HashSet<usize> = HashSet::new();
    let mut out: Vec<ir::Block> = vec![];
    let mut entry = ir::BasicBlock(0);
    // the block whose overlay() ended the phase before, to point at this phase's copy of its start
    let mut ended: Option<usize> = None;
    let mut done: HashSet<usize> = HashSet::new();
    for phase in 1.. {
        // the blocks this phase reaches, in block order, and the overlay() calls it ends at
        let mut seen: HashSet<usize> = HashSet::new();
        let mut todo = vec![*at
            .get(&entry)
            .ok_or_else(|| format!("phase {phase} starts at {entry}, which is not a block"))?];
        while let Some(i) = todo.pop() {
            if !seen.insert(i) || is_overlay(&blocks[i]) {
                continue;
            }
            for s in blocks[i].terminator.successors() {
                if let Some(&j) = at.get(&s) {
                    todo.push(j);
                }
            }
        }
        // the phase's entry first: the loader starts it at its first line, and rustc can number a block the phase
        // reaches before the one overlay() returns to
        let start = at[&entry];
        let mut mine: Vec<usize> = seen.into_iter().collect();
        mine.sort_by_key(|&i| (i != start, i));
        let ends: Vec<usize> = mine.iter().copied().filter(|&i| is_overlay(&blocks[i])).collect();
        if ends.len() > 1 {
            return Err(format!("phase {phase} ends at more than one overlay()"));
        }
        // this phase's name for each block: its own, or a fresh one if an earlier phase has it already
        let mut name: HashMap<ir::BasicBlock, ir::BasicBlock> = HashMap::new();
        for &i in &mine {
            let n = if owned.insert(i) {
                blocks[i].name
            } else {
                fresh += 1;
                ir::BasicBlock(fresh - 1)
            };
            name.insert(blocks[i].name, n);
        }
        if let Some(k) = ended {
            let (from, to) = (entry, name[&entry]);
            for t in out[k].terminator.targets_mut() {
                if *t == from {
                    *t = to;
                }
            }
        }
        let first = out.len();
        for &i in mine.iter().filter(|&&i| !is_overlay(&blocks[i])).chain(ends.iter()) {
            let mut b = blocks[i].clone();
            b.name = name[&b.name];
            if !is_overlay(&b) {
                for t in b.terminator.targets_mut() {
                    if let Some(n) = name.get(t) {
                        *t = *n;
                    }
                }
            }
            out.push(b);
        }
        let Some(&e) = ends.first() else { break };
        if !done.insert(e) {
            return Err(format!(
                "phase {phase} ends at an overlay() that an earlier phase ended at: overlay() in a loop"
            ));
        }
        ended = Some(first + mine.len() - 1);
        entry = blocks[e]
            .terminator
            .successors()
            .into_iter()
            .next()
            .ok_or_else(|| format!("phase {phase}'s overlay() goes nowhere"))?;
    }
    *blocks = out;
    Ok(())
}

/// A program split into phases: the data block every phase shares, and each phase's code (with the cells only it
/// uses). Each prints as assembly text, every line ended by a newline.
pub struct Phases {
    pub data: Vec<Line>,
    pub code: Vec<Vec<Line>>,
}

impl Phases {
    pub fn data_text(&self) -> String {
        text(&self.data)
    }

    pub fn code_text(&self, i: usize) -> String {
        text(&self.code[i])
    }
}

fn text(lines: &[Line]) -> String {
    lines.iter().map(|l| format!("{l}\n")).collect()
}

/// The label a line defines: a label of its own, or a data cell's.
fn label_of(l: &Line) -> Option<&str> {
    match &l.kind {
        Kind::Label(name) | Kind::Table { label: name, .. } | Kind::Dat(Some(name), _) => Some(name),
        _ => None,
    }
}

/// The word an instruction's or a data cell's operand starts with: a symbol without its `+N`, or a number. A
/// negative number names nothing.
fn operand_word(l: &Line) -> Option<&str> {
    match &l.kind {
        Kind::Instr(_, arg) | Kind::Dat(_, arg) => match arg {
            Arg::Sym(name, _) => Some(name),
            Arg::Num(n) => Some(n.as_str()).filter(|n| !n.starts_with('-')),
        },
        _ => None,
    }
}

/// The symbol a data cell's value names (an address constant names the cell it points at).
fn dat_symbol(l: &Line) -> Option<&str> {
    match &l.kind {
        Kind::Dat(_, arg) => arg.symbol(),
        _ => None,
    }
}

/// A data cell with no label: the next cell of the array labelled before it.
fn is_continuation(l: &Line) -> bool {
    matches!(l.kind, Kind::Dat(None, _))
}

/// Does a unit other than main start here: a function's entry, or the first routine's label?
fn is_unit_start(l: &Line) -> bool {
    label_of(l).is_some_and(|name| name == "rt_panic" || (name.len() > "_entry".len() && name.ends_with("_entry")))
}

/// Does control run off the end of the lines into whatever comes next: is their last instruction other than HRS or
/// a JMP that isn't a call (a call comes back)? The peephole drops a JMP to the next label, the panic routine's too.
/// Lines with no instruction at all run straight on (a main that only panics is no code before rt_panic).
fn falls_off<'a>(lines: impl DoubleEndedIterator<Item = &'a Line>) -> bool {
    lines
        .rev()
        .find_map(|l| match &l.kind {
            Kind::Instr(op, arg) => Some((*op, arg)),
            _ => None,
        })
        .is_none_or(|(op, arg)| match op {
            Op::Hrs => false,
            Op::Jmp => arg
                .symbol()
                .is_some_and(|name| name.len() > "_entry".len() && name.ends_with("_entry")),
            _ => true,
        })
}

/// The line the data block starts at.
fn is_data_start(l: &Line) -> bool {
    matches!(l.kind, Kind::DataStart)
}

/// Where main's next phase starts.
fn is_phase_marker(l: &Line) -> bool {
    matches!(l.kind, Kind::Phase(_))
}

/// The labels the lines define.
fn defs<'a>(lines: impl IntoIterator<Item = &'a Line>) -> HashSet<&'a str> {
    lines.into_iter().filter_map(label_of).collect()
}

fn is_number(word: &str) -> bool {
    word.bytes().all(|c| c.is_ascii_digit())
}

/// Split a listing into phases. Main's code is cut at each phase marker; each phase then takes every function and
/// routine its code names, and the ones those name, until it names nothing it lacks.
pub fn split_phases(listing: &[Line]) -> Result<Phases, Fail> {
    // with no data heading, the last line is taken for the data block
    let di = listing
        .iter()
        .position(is_data_start)
        .unwrap_or(listing.len().saturating_sub(1));
    let (code, data) = listing.split_at(di);
    // main's code runs until the first unit that is not main's; each unit runs to the next one's start
    let first = code.iter().position(is_unit_start).unwrap_or(code.len());
    let (main_code, rest) = code.split_at(first);
    // a unit that runs off its end into the next (its JMP to the next label dropped) is one unit with it, so the two
    // stay together wherever the unit goes
    let mut bounds: Vec<usize> = vec![];
    for i in (0..rest.len()).filter(|&i| is_unit_start(&rest[i])) {
        if bounds.last().is_none_or(|&b| !falls_off(rest[b..i].iter())) {
            bounds.push(i);
        }
    }
    bounds.push(rest.len());
    let units: Vec<&[Line]> = bounds.windows(2).map(|w| &rest[w[0]..w[1]]).collect();
    let unit_defs: Vec<HashSet<&str>> = units.iter().map(|u| defs(u.iter())).collect();
    let data_labels = defs(data);
    let mut bodies: Vec<Vec<&Line>> = vec![];
    let last = main_code.split(is_phase_marker).count() - 1;
    for (i, phase) in main_code.split(is_phase_marker).enumerate() {
        let mut have = defs(phase);
        let mut body: Vec<&Line> = phase.iter().collect();
        // main's last phase that runs off its end runs into the first unit (the panic routine, its JMP rt_panic
        // dropped as rt_panic came next): that unit comes next, whether or not anything names it
        if i == last && falls_off(phase.iter()) {
            if let (Some(unit), Some(d)) = (units.first(), unit_defs.first()) {
                body.extend(unit.iter());
                have.extend(d.iter().copied());
            }
        }
        loop {
            // what the phase names and does not define, each once, in the order it first names them (the order
            // an error lists them in)
            let mut need: Vec<&str> = vec![];
            for r in body.iter().filter_map(|l| operand_word(l)) {
                if !have.contains(r) && !data_labels.contains(r) && !is_number(r) && !need.contains(&r) {
                    need.push(r);
                }
            }
            // a unit comes in once, when the phase names it and has none of it yet
            let mut changed = false;
            for (unit, d) in units.iter().zip(&unit_defs) {
                if need.iter().any(|x| d.contains(x)) && !d.iter().any(|x| have.contains(x)) {
                    body.extend(unit.iter());
                    have.extend(d.iter().copied());
                    changed = true;
                }
            }
            if changed {
                continue;
            }
            if !need.is_empty() {
                return Err(format!(
                    "phase {} jumps to {}, in another phase",
                    i + 1,
                    need.join(", ")
                ));
            }
            break;
        }
        bodies.push(body);
    }
    // Data only one phase uses lives in that phase's own code block, after its code: the next phase's code loads over
    // it, and it is dead by then. Only what several phases share stays in the common data block.
    // Each data cell or array: its label, and its lines. An unlabelled cell before any label belongs to none.
    let mut items: Vec<(&str, Vec<&Line>)> = vec![];
    for l in data {
        if let Some(name) = label_of(l) {
            items.push((name, vec![l]));
        } else if is_continuation(l) {
            if let Some((_, lines)) = items.last_mut() {
                lines.push(l);
            }
        }
    }
    let used_by: Vec<HashSet<&str>> = bodies
        .iter()
        .map(|body| body.iter().filter_map(|l| operand_word(l)).collect())
        .collect();
    let dat_refs: Vec<(&str, Vec<&str>)> = items
        .iter()
        .map(|(name, lines)| (*name, lines.iter().filter_map(|l| dat_symbol(l)).collect()))
        .collect();
    // the phases that use a cell, or use a cell whose value names it
    let phases_of = |name: &str| -> Vec<usize> {
        let mut p: Vec<usize> = vec![];
        let mut add = |k: usize| {
            if !p.contains(&k) {
                p.push(k)
            }
        };
        for (k, u) in used_by.iter().enumerate() {
            if u.contains(name) {
                add(k);
            }
        }
        for (other, refs) in &dat_refs {
            if refs.contains(&name) {
                for (k, u) in used_by.iter().enumerate() {
                    if u.contains(other) {
                        add(k);
                    }
                }
            }
        }
        p
    };
    let mut common: Vec<&Line> = vec![];
    let mut own: Vec<Vec<&Line>> = vec![vec![]; bodies.len()];
    for (name, lines) in &items {
        if bodies.len() == 1 {
            common.extend(lines);
            continue;
        }
        let p = phases_of(name);
        // keep it common if a common cell's value names it, or several phases (or none: _exit cells) use it
        let named = dat_refs
            .iter()
            .any(|(other, refs)| refs.contains(name) && phases_of(other).len() != 1);
        if p.len() == 1 && !named && !name.ends_with("_exit") {
            own[p[0]].extend(lines);
        } else {
            common.extend(lines);
        }
    }
    // the data block's headings and comments come first, then its common cells
    let head = data.iter().filter(|l| label_of(l).is_none() && !is_continuation(l));
    let mut shared: Vec<Line> = head.chain(common).cloned().collect();
    // a data block with nothing in it is still one line
    if shared.is_empty() {
        shared.push(Line::new(Kind::Blank));
    }
    // each phase opens with its own start label for the loader: start, then p2, p3, ...
    let mut phases: Vec<Vec<Line>> = vec![];
    for (i, (body, own)) in bodies.iter().zip(&own).enumerate() {
        let start = if i == 0 {
            "start".to_string()
        } else {
            format!("p{}", i + 1)
        };
        let mut lines = vec![Line::label(&start)];
        lines.extend(body.iter().copied().cloned());
        // a phase with no code still has a line after its label
        if body.is_empty() {
            lines.push(Line::new(Kind::Blank));
        }
        if !own.is_empty() {
            lines.push(Line::new(Kind::Note(" ---- this phase's own cells ----".to_string())));
            lines.extend(own.iter().copied().cloned());
        }
        phases.push(lines);
    }
    Ok(Phases {
        data: shared,
        code: phases,
    })
}
