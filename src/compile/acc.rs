//! The accumulator across blocks (#36). The peephole rules see a few instructions at a time and stop at a label; this
//! pass follows the program's control flow (the compiled stack's, `stack::successors`), into and out of functions,
//! and works out for each instruction which cells the accumulator holds the value of whenever it runs: the cells
//! every way in leaves it holding. A `CLA x` where it already holds x, or a `STO x` where x already holds what it
//! does, goes. Then a store into a cell that nothing reads before it is written again goes too: the value lives in
//! the accumulator only, as a loop's count or a sum carried from one block into the next does in code written by
//! hand. Last, a block that starts `CLA x`, where some ways in leave x in the accumulator and the others come by a
//! TAC, can have those TACs load x on their own way in (`loads_on_the_way_in`).
//!
//! Only the cells nothing else reaches take part (`stack::plain_cells`): every use of one is an instruction that
//! names it. A program whose control flow the compiled stack doesn't follow (in phases, or with a jump to a number)
//! is left as it is.
use std::collections::{HashMap, HashSet};

use super::machine::{Arg, Kind, Line, Op};
use super::stack::{code, plain_cells, successors};

/// The listing with the loads and stores the accumulator makes needless taken away, and the loads moved onto TACs'
/// ways in where that pays (`loads_on_the_way_in`).
pub fn carry(lines: &[Line]) -> Vec<Line> {
    if lines.iter().any(|line| matches!(line.kind, Kind::Phase(_))) {
        return lines.to_vec();
    }
    let settled = settle(lines.to_vec());
    loads_on_the_way_in(&settled).unwrap_or(settled)
}

/// The loads and stores taken away until none is left to take.
fn settle(mut lines: Vec<Line>) -> Vec<Line> {
    loop {
        let Some(flow) = Flow::of(&lines) else {
            return lines;
        };
        let held = flow.held();
        let mut gone: Vec<usize> = (0..flow.at.len())
            .filter(|&c| flow.needless(c, &held))
            .map(|c| flow.at[c])
            .collect();
        if gone.is_empty() {
            gone = flow.dead_stores();
        }
        if gone.is_empty() {
            return lines;
        }
        remove(&mut lines, &gone);
    }
}

/// The blocks that start `CLA x`, x in the accumulator on some ways in and the others all TACs, with each of those
/// TACs going instead to a `CLA x` and a `JMP` to the block of its own, and the block's load gone: a way in from the
/// block before is then a load shorter, and a way in by a TAC one JMP longer. Once the accumulator carries x into
/// the block, the store into x that ends it often has nothing left to read it (the next block, done the same way,
/// no longer loads x): one instruction less on every way through the block. All such blocks are done at once, as
/// a row of them that fall into one another pays only together; the listing as it then settles, when it takes no
/// more cells than `settled` does. A block whose store stays is left out (a way into it by a TAC would be a JMP
/// longer for nothing), and the rest tried again, unless it is the last of a row of blocks that fall into one
/// another, the others' stores gone: the row's own store, for what reads x after it. A way into that last block by
/// a TAC is the one way made longer, by a JMP, against two instructions less for each block of the row a way runs
/// through. A block the TAC is in a loop of (the block comes before the TAC on every way to it) is never done:
/// the TAC is the loop's way round, run far more often than the way in.
fn loads_on_the_way_in(settled: &[Line]) -> Option<Vec<Line>> {
    let flow = Flow::of(settled)?;
    let held = flow.held();
    let mut blocks = flow.loaded_blocks(&held);
    while !blocks.is_empty() {
        let tried = settle(flow.with_loads_on(&blocks));
        let paid: Vec<Loaded> = blocks
            .iter()
            .filter(|b| !stores_into(&tried, b) || falls_from_paid(&tried, b, &blocks))
            .cloned()
            .collect();
        if paid.len() == blocks.len() {
            return (super::cells(&tried) <= super::cells(settled)).then_some(tried);
        }
        blocks = paid;
    }
    None
}

/// Does the block, in the listing as tried, fall in from another of `blocks` whose store went: the last of a row
/// of them, whose own store stays for what reads x after the row?
fn falls_from_paid(lines: &[Line], block: &Loaded, blocks: &[Loaded]) -> bool {
    let Some(at) = lines
        .iter()
        .position(|l| matches!(&l.kind, Kind::Label(n) if *n == block.label))
    else {
        return false;
    };
    let named = named(lines);
    let mut falls = false;
    for line in lines[..at].iter().rev() {
        match &line.kind {
            Kind::Instr(op, arg) if !falls => {
                let ends =
                    matches!(op, Op::Hrs) || (*op == Op::Jmp && !arg.symbol().is_some_and(|n| n.ends_with("_entry")));
                if ends {
                    return false;
                }
                falls = true;
            }
            Kind::Dat(..) => return false,
            Kind::Label(name) if falls && blocks.iter().any(|b| b.label == *name) => {
                return !stores_into(lines, blocks.iter().find(|b| b.label == *name).unwrap());
            }
            Kind::Label(name) if falls && (named.contains(name.as_str()) || is_return(name)) => return false,
            _ => {}
        }
    }
    false
}

/// Does the block, in the listing as tried, still store into its x before it ends (at a jump, or a label something
/// names: another way in)?
fn stores_into(lines: &[Line], block: &Loaded) -> bool {
    let Some(at) = lines
        .iter()
        .position(|l| matches!(&l.kind, Kind::Label(n) if *n == block.label))
    else {
        return true;
    };
    let named = named(lines);
    for line in &lines[at + 1..] {
        match &line.kind {
            Kind::Label(name) if named.contains(name.as_str()) || is_return(name) => return false,
            Kind::DataStart => return false,
            Kind::Instr(Op::Sto, Arg::Sym(name, None)) if *name == block.x => return true,
            Kind::Instr(Op::Jmp | Op::Tac | Op::Hrs, _) | Kind::Dat(..) => return false,
            _ => {}
        }
    }
    false
}

/// The symbols the listing's instructions and constants name.
fn named(lines: &[Line]) -> HashSet<&str> {
    lines
        .iter()
        .filter_map(|l| match &l.kind {
            Kind::Instr(_, arg) | Kind::Dat(_, arg) => arg.symbol(),
            _ => None,
        })
        .collect()
}

/// A call's way back, come to through cell 99 and not by name.
fn is_return(name: &str) -> bool {
    let stem = name.trim_end_matches(|c: char| c.is_ascii_digit());
    stem.len() < name.len() && (stem.ends_with("_ret") || stem.ends_with("_rr"))
}

/// A block whose load can go onto its TACs' ways in: its first code cell, a label of it, the cell it loads, and the
/// TACs.
#[derive(Clone)]
struct Loaded {
    cell: usize,
    label: String,
    x: String,
    tacs: Vec<usize>,
}

/// Take these lines away, each one's comment passing to the instruction after it when that has none.
fn remove(lines: &mut Vec<Line>, gone: &[usize]) {
    let gone: HashSet<usize> = gone.iter().copied().collect();
    let mut comment: Option<String> = None;
    let mut out = Vec::with_capacity(lines.len());
    for (i, line) in lines.drain(..).enumerate() {
        if gone.contains(&i) {
            comment = comment.or(line.comment);
            continue;
        }
        let mut line = line;
        if matches!(line.kind, Kind::Instr(..)) {
            if line.comment.is_none() {
                line.comment = comment.take();
            }
            comment = None;
        } else if matches!(line.kind, Kind::Label(_) | Kind::Dat(..) | Kind::DataStart) {
            comment = None;
        }
        out.push(line);
    }
    *lines = out;
}

/// The program's code cells, where they are in the listing, what can run after each, and the cells taking part.
struct Flow<'a> {
    lines: &'a [Line],
    /// for each code cell, its line
    at: Vec<usize>,
    after: Vec<Vec<usize>>,
    before: Vec<Vec<usize>>,
    labels: HashMap<&'a str, usize>,
    /// the cells taking part, by name, each with its number
    number: HashMap<&'a str, usize>,
    /// the code cells the program stores into: what runs there is not what the listing shows
    built: HashSet<usize>,
    entry: usize,
}

/// What the accumulator holds the value of, at a cell: None where no way in has been seen yet.
type Held = Option<HashSet<usize>>;

impl<'a> Flow<'a> {
    fn of(lines: &'a [Line]) -> Option<Flow<'a>> {
        let data_start = lines.iter().position(|line| matches!(line.kind, Kind::DataStart))?;
        let code = code(&lines[..data_start]);
        let after = successors(&code)?;
        let at: Vec<usize> = lines[..data_start]
            .iter()
            .enumerate()
            .filter(|(_, l)| matches!(l.kind, Kind::Instr(..) | Kind::Dat(..)))
            .map(|(i, _)| i)
            .collect();
        let mut before = vec![vec![]; at.len()];
        for (c, next) in after.iter().enumerate() {
            for &n in next {
                before[n].push(c);
            }
        }
        let names: Vec<&str> = plain_cells(lines, data_start)
            .iter()
            .filter_map(|name| {
                lines[data_start..].iter().find_map(|l| match &l.kind {
                    Kind::Dat(Some(n), _) if n == name => Some(n.as_str()),
                    _ => None,
                })
            })
            .collect();
        let number = names.iter().enumerate().map(|(i, &name)| (name, i)).collect();
        let entry = ["start", "main_bb0"]
            .iter()
            .find_map(|label| code.labels.get(label).copied())
            .unwrap_or(0);
        let built = lines
            .iter()
            .filter_map(|l| match &l.kind {
                Kind::Instr(Op::Sto | Op::Inp, Arg::Sym(name, offset)) => {
                    let at = *code.labels.get(name.as_str())?;
                    Some(at + offset.unwrap_or(0) as usize)
                }
                _ => None,
            })
            .collect();
        Some(Flow {
            lines,
            at,
            after,
            before,
            labels: code.labels,
            number,
            built,
            entry,
        })
    }

    fn line(&self, c: usize) -> &'a Line {
        &self.lines[self.at[c]]
    }

    /// The cell taking part that an instruction names plainly.
    fn plain(&self, arg: &Arg) -> Option<usize> {
        match arg {
            Arg::Sym(name, None) => self.number.get(name.as_str()).copied(),
            _ => None,
        }
    }

    /// What the accumulator holds the value of after the code cell c runs, from what it held before.
    fn step(&self, c: usize, held: &mut HashSet<usize>) {
        let line = self.line(c);
        if self.built.contains(&c) {
            held.clear();
            return;
        }
        match &line.kind {
            Kind::Instr(Op::Cla, arg) => {
                held.clear();
                held.extend(self.plain(arg));
            }
            Kind::Instr(Op::Sto, arg) => match (self.plain(arg), arg) {
                (Some(x), _) if line.wide => {
                    held.remove(&x);
                }
                (Some(x), _) => {
                    held.insert(x);
                }
                // a cell of an array: one past its end may be another's
                (None, Arg::Sym(_, Some(_))) => held.clear(),
                (None, _) => {}
            },
            Kind::Instr(Op::Inp, arg) => match (self.plain(arg), arg) {
                (Some(x), _) => {
                    held.remove(&x);
                }
                (None, Arg::Sym(_, Some(_))) => held.clear(),
                (None, _) => {}
            },
            Kind::Instr(Op::Out | Op::Tac | Op::Jmp | Op::Hrs, _) => {}
            // an arithmetic op, or a cell in the code run as whatever instruction was stored there
            _ => held.clear(),
        }
    }

    /// What the accumulator holds the value of before each code cell runs, on every way there.
    fn held(&self) -> Vec<Held> {
        let count = self.at.len();
        let mut into: Vec<Held> = vec![None; count];
        let mut work: Vec<usize> = vec![];
        for c in 0..count {
            if c == self.entry || self.before[c].is_empty() {
                into[c] = Some(HashSet::new());
                work.push(c);
            }
        }
        while let Some(c) = work.pop() {
            let Some(mut held) = into[c].clone() else { continue };
            self.step(c, &mut held);
            for &n in &self.after[c] {
                let met = match &into[n] {
                    None => held.clone(),
                    Some(old) => old.intersection(&held).copied().collect(),
                };
                if into[n].as_ref() != Some(&met) {
                    into[n] = Some(met);
                    work.push(n);
                }
            }
        }
        into
    }

    /// A load of what the accumulator holds already, or a store of it into the cell that holds it.
    fn needless(&self, c: usize, held: &[Held]) -> bool {
        let Some(held) = &held[c] else { return false };
        if self.built.contains(&c) {
            return false;
        }
        let line = self.line(c);
        match &line.kind {
            Kind::Instr(Op::Cla, arg) => self.plain(arg).is_some_and(|x| held.contains(&x)),
            Kind::Instr(Op::Sto, arg) => !line.wide && self.plain(arg).is_some_and(|x| held.contains(&x)),
            _ => false,
        }
    }

    /// For each code cell, the cells taking part that some way on from it reads before it writes them.
    fn live(&self) -> Vec<HashSet<usize>> {
        let count = self.at.len();
        let mut live_in: Vec<HashSet<usize>> = vec![HashSet::new(); count];
        let mut changed = true;
        while changed {
            changed = false;
            for c in (0..count).rev() {
                let mut live: HashSet<usize> = self.after[c].iter().flat_map(|&n| live_in[n].iter().copied()).collect();
                if let Kind::Instr(op, arg) = &self.line(c).kind {
                    if let Some(x) = self.plain(arg) {
                        if matches!(op, Op::Sto | Op::Inp) {
                            live.remove(&x);
                        } else {
                            live.insert(x);
                        }
                    }
                }
                if live != live_in[c] {
                    live_in[c] = live;
                    changed = true;
                }
            }
        }
        live_in
    }

    /// The lines of the stores into a cell nothing reads before it is written again.
    fn dead_stores(&self) -> Vec<usize> {
        let live_in = self.live();
        (0..self.at.len())
            .filter(|&c| match &self.line(c).kind {
                Kind::Instr(Op::Sto, arg) => self
                    .plain(arg)
                    .is_some_and(|x| !self.after[c].iter().any(|&n| live_in[n].contains(&x))),
                _ => false,
            })
            .map(|c| self.at[c])
            .collect()
    }

    /// The blocks whose load can go onto their TACs' ways in (`loads_on_the_way_in`).
    fn loaded_blocks(&self, held: &[Held]) -> Vec<Loaded> {
        let dominators = self.dominators();
        let mut blocks = vec![];
        for c in 0..self.at.len() {
            let x = match &self.line(c).kind {
                Kind::Instr(Op::Cla, arg) if !self.built.contains(&c) => self.plain(arg).map(|x| (x, arg)),
                _ => None,
            };
            // (not a scratch cell: the peephole rules take one to be read only in the block that writes it)
            let Some((x, Arg::Sym(name, None))) = x else { continue };
            if name
                .strip_prefix('t')
                .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
            {
                continue;
            }
            let has = |p: usize| {
                held[p].as_ref().is_some_and(|h| {
                    let mut h = h.clone();
                    self.step(p, &mut h);
                    h.contains(&x)
                })
            };
            let (with, without): (Vec<usize>, Vec<usize>) = self.before[c].iter().partition(|&&p| has(p));
            if with.is_empty() || without.is_empty() {
                continue;
            }
            // each way without x a TAC that goes to the block, not one that falls into it
            let tacs = without.iter().all(|&p| {
                p + 1 != c
                    && !self.built.contains(&p)
                    && matches!(&self.line(p).kind, Kind::Instr(Op::Tac, Arg::Sym(to, None))
                        if self.labels.get(to.as_str()) == Some(&c))
            });
            let Some(label) = self.label_at(c) else { continue };
            if tacs && !without.iter().any(|&p| dominators[p].contains(&c)) && self.stores_after(c, x) {
                blocks.push(Loaded {
                    cell: c,
                    label: label.to_string(),
                    x: name.clone(),
                    tacs: without,
                });
            }
        }
        blocks
    }

    /// A code label on code cell c.
    fn label_at(&self, c: usize) -> Option<&'a str> {
        self.lines[..self.at[c]]
            .iter()
            .rev()
            .take_while(|l| matches!(l.kind, Kind::Label(_) | Kind::Note(_) | Kind::Blank))
            .find_map(|l| match &l.kind {
                Kind::Label(name) => Some(name.as_str()),
                _ => None,
            })
    }

    /// The listing with each block's TACs going to a `CLA x`, `JMP` to the block of their own, put after the first
    /// instruction from the block on that control never falls through (or the block left as it is, when there is
    /// none before the data), and the block's load gone.
    fn with_loads_on(&self, blocks: &[Loaded]) -> Vec<Line> {
        let mut names: HashSet<String> = self.labels.keys().map(|n| n.to_string()).collect();
        let mut stubs: HashMap<usize, Vec<Line>> = HashMap::new();
        let mut tac_to: HashMap<usize, String> = HashMap::new();
        let mut gone: Vec<usize> = vec![];
        for block in blocks {
            let ends = |l: &Line| match &l.kind {
                Kind::Instr(Op::Hrs, _) => true,
                Kind::Instr(Op::Jmp, Arg::Sym(name, None)) => !name.ends_with("_entry"),
                Kind::Instr(Op::Jmp, _) => true,
                _ => false,
            };
            let Some(place) = (self.at[block.cell]..self.lines.len())
                .take_while(|&i| !matches!(self.lines[i].kind, Kind::DataStart))
                .find(|&i| ends(&self.lines[i]))
            else {
                continue;
            };
            for &p in &block.tacs {
                let mut k = 0;
                let name = loop {
                    let name = format!("{}_in{k}", block.label);
                    if names.insert(name.clone()) {
                        break name;
                    }
                    k += 1;
                };
                tac_to.insert(self.at[p], name.clone());
                let at = stubs.entry(place).or_default();
                at.push(Line::label(&name));
                at.push(Line::commented(
                    Op::Cla,
                    Arg::sym(&block.x),
                    &format!("into {}", block.label),
                ));
                at.push(Line::instr(Op::Jmp, Arg::sym(&block.label)));
            }
            gone.push(self.at[block.cell]);
        }
        let gone: HashSet<usize> = gone.into_iter().collect();
        let mut out: Vec<Line> = Vec::with_capacity(self.lines.len());
        let mut loads: Vec<usize> = vec![];
        for (i, line) in self.lines.iter().enumerate() {
            let mut line = line.clone();
            if let Some(name) = tac_to.get(&i) {
                line.kind = Kind::Instr(Op::Tac, Arg::sym(name));
            }
            if gone.contains(&i) {
                loads.push(out.len());
            }
            out.push(line);
            if let Some(more) = stubs.remove(&i) {
                out.extend(more);
            }
        }
        remove(&mut out, &loads);
        out
    }

    /// For each code cell, the cells every way to it from the start runs through.
    fn dominators(&self) -> Vec<HashSet<usize>> {
        let count = self.at.len();
        let all: HashSet<usize> = (0..count).collect();
        let starts = |c: usize| c == self.entry || self.before[c].is_empty();
        let mut dom: Vec<HashSet<usize>> = (0..count)
            .map(|c| if starts(c) { HashSet::from([c]) } else { all.clone() })
            .collect();
        let mut changed = true;
        while changed {
            changed = false;
            for c in (0..count).filter(|&c| !starts(c)) {
                let mut met: Option<HashSet<usize>> = None;
                for &p in &self.before[c] {
                    met = Some(match met {
                        None => dom[p].clone(),
                        Some(m) => m.intersection(&dom[p]).copied().collect(),
                    });
                }
                let mut next = met.unwrap_or_default();
                next.insert(c);
                if next != dom[c] {
                    dom[c] = next;
                    changed = true;
                }
            }
        }
        dom
    }

    /// Does the block that starts at c store into x, in the straight run of code from c on, before anything else
    /// reads x or control goes elsewhere? That store is what a way in by a TAC is to save.
    fn stores_after(&self, c: usize, x: usize) -> bool {
        let mut n = c + 1;
        while n < self.at.len() && self.before[n] == [n - 1] && !self.built.contains(&n) {
            match &self.line(n).kind {
                Kind::Instr(Op::Sto, arg) if self.plain(arg) == Some(x) => return true,
                Kind::Instr(_, arg) if self.plain(arg) == Some(x) => return false,
                Kind::Instr(Op::Jmp | Op::Tac | Op::Hrs, _) | Kind::Dat(..) => return false,
                _ => {}
            }
            n += 1;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::machine::{parse, print};

    /// The listing carried, as text, from a listing given as text.
    fn carried(listing: &str) -> String {
        print(&carry(&parse(listing).unwrap()))
    }

    /// The instructions of a listing, each as `OP arg`, in order.
    fn instructions(listing: &str) -> Vec<String> {
        parse(listing)
            .unwrap()
            .iter()
            .filter_map(|l| match &l.kind {
                Kind::Instr(op, arg) => Some(format!("{} {}", op.mnemonic(), print_arg(arg))),
                _ => None,
            })
            .collect()
    }

    fn print_arg(arg: &Arg) -> String {
        match arg {
            Arg::Num(n) => n.clone(),
            Arg::Sym(s, None) => s.clone(),
            Arg::Sym(s, Some(k)) => format!("{s}+{k}"),
        }
    }

    /// A count down in a loop, kept in its cell, then output: the loop's way round leaves it in the accumulator, and
    /// so does the way in, so the load at the top goes; and then nothing reads the first store, while the loop's is
    /// read after the loop.
    const LOOP: &str = "\
main_bb0:
        INP main_1
        CLA main_1
        STO main_2
main_bb1:
        CLA main_2
        SUB k1
        STO main_2
        TAC main_bb2
        JMP main_bb1
main_bb2:
        OUT main_2
        HRS 0
; ---- locals and constants ----
main_1:   DAT 0
main_2:   DAT 0
k1:       DAT 1
";

    #[test]
    fn a_load_every_way_in_leaves_in_the_accumulator_goes() {
        let after = instructions(&carried(LOOP));
        assert_eq!(
            after,
            [
                "INP main_1",
                "CLA main_1",
                "SUB k1",
                "STO main_2",
                "TAC main_bb2",
                "JMP main_bb1",
                "OUT main_2",
                "HRS 0"
            ]
        );
    }

    #[test]
    fn a_load_one_way_in_doesnt_leave_stays() {
        // the way round loads another cell first
        let other = LOOP.replace(
            "        TAC main_bb2\n        JMP main_bb1\n",
            "        TAC main_bb2\n        CLA main_1\n        JMP main_bb1\n",
        );
        assert!(instructions(&carried(&other)).contains(&"CLA main_2".to_string()));
    }

    #[test]
    fn a_store_nothing_reads_goes_and_one_read_later_stays() {
        // without the OUT nothing reads main_2
        let unread = LOOP.replace("        OUT main_2\n", "");
        let after = instructions(&carried(&unread));
        assert!(!after.contains(&"STO main_2".to_string()), "{after:?}");
        assert!(instructions(&carried(LOOP)).contains(&"STO main_2".to_string()));
    }

    /// Two blocks in a row, each also come to by a TAC: each TAC loads the sum on its own way in, and the sum stays in
    /// the accumulator from the first block into the second.
    const ROW: &str = "\
main_bb0:
        INP main_1
        INP main_2
        CLA k0
        STO main_3
        CLA main_1
        TAC main_bb3
        SUB 00
        TAC main_bb2
        SUB 00
        TAC main_bb1
        CLA main_3
        ADD main_2
        STO main_3
main_bb1:
        CLA main_3
        ADD main_2
        STO main_3
main_bb2:
        CLA main_3
        ADD main_2
        STO main_3
main_bb3:
        OUT main_3
        HRS 0
; ---- locals and constants ----
main_1:   DAT 0
main_2:   DAT 0
main_3:   DAT 0
k0:       DAT 0
";

    #[test]
    fn a_row_entered_by_tacs_carries_its_sum() {
        let text = carried(ROW);
        let after = instructions(&text);
        assert_eq!(after.iter().filter(|i| *i == "ADD main_2").count(), 3);
        // one store, at the end of the row; a load on each TAC's way in, and the row's first load
        assert_eq!(after.iter().filter(|i| *i == "STO main_3").count(), 2, "{text}");
        assert_eq!(after.iter().filter(|i| *i == "CLA main_3").count(), 3, "{text}");
        assert!(text.contains("main_bb1_in0:\n        CLA main_3"), "{text}");
        assert!(super::super::cells(&parse(&text).unwrap()) <= super::super::cells(&parse(ROW).unwrap()));
    }

    #[test]
    fn a_loop_s_way_round_is_never_made_longer() {
        // the TAC is the loop's way back to its top: it stays a TAC straight there
        let round = "\
main_bb0:
        INP main_1
        CLA k0
        STO main_2
main_bb1:
        CLA main_2
        OUT main_2
        ADD k1
        STO main_2
        SUB main_1
        TAC main_bb1
        HRS 0
; ---- locals and constants ----
main_1:   DAT 0
main_2:   DAT 0
k0:       DAT 0
k1:       DAT 1
";
        let text = carried(round);
        assert!(!text.contains("_in0"), "{text}");
        assert!(text.contains("TAC main_bb1"), "{text}");
    }

    #[test]
    fn a_call_s_cells_are_followed_into_the_callee() {
        // f reads its argument cell: the store into it stays, though nothing in main reads it
        let call = "\
main_bb0:
        INP main_1
        CLA main_1
        STO f_0
        JMP f_entry
main_ret0:
        HRS 0
f_entry:
        CLA 99
        STO f_exit
        OUT f_0
        JMP f_exit
f_exit:   DAT 0
; ---- locals and constants ----
main_1:   DAT 0
f_0:      DAT 0
";
        assert!(instructions(&carried(call)).contains(&"STO f_0".to_string()));
    }
}
