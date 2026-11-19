//! Cross-jumping, on the machine IR once every other rule is done. Two runs of code that end the same way and go
//! on to the same place (a `JMP` to one label, a `HRS`, or running on into the label) keep one copy of what they
//! end with: the other run jumps into it at the cell where they start to agree. Arms of a `match` that each end
//! with the same store and jump share them, and arms that each add c a few more times than the one before share one
//! row of ADDs, entered at the cell that leaves each arm its own number of them, as a multiply by a digit is written
//! by hand (#35).
//!
//! What a merge costs: the run that loses its copy now jumps into the kept one. When the kept copy runs on into
//! the place they go to and the other ended in a jump there, that jump becomes the jump in, and nothing more runs;
//! when the other's copy starts at a label nothing runs into, its labels go to the kept copy and no jump is needed
//! at all. Otherwise the jump in is one instruction more each time that way runs. Those merges are made only for a
//! program that doesn't fit without them (`SLOWER`), as `blocks::reroll_reads` is.
//!
//! Only code that is run as it stands is matched: not a cell the program writes or reads as data (a labelled `DAT`
//! in the code, or a run after a label that a constant, an offset or a load names), not the runtime routines, and
//! never across a phase or from one function into another. A jump writes cell 99, so a function that reads cell 99
//! anywhere but at its entry is left as it is.
use std::collections::{HashMap, HashSet};

use super::machine::{Arg, Kind, Line, Op};

thread_local! {
    /// Merges that put a jump on a way that had none: on only when the program doesn't fit without them.
    pub static SLOWER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Where a run goes when it ends.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum Exit {
    /// To the line at this index, by a jump to a label there or by running on into it.
    At(usize),
    /// A jump to a symbol that is not a code label (a function's exit cell) or to a numbered cell.
    To(Arg),
    /// A halt, HRS and its operand.
    Halt(Arg),
}

/// A run's end, and the lines before it that may be shared.
struct Tail {
    /// The jump or halt that ends it, or `None` when it runs on into the label its exit names.
    end: Option<usize>,
    exit: Exit,
    /// The lines before its end, latest first, up to the first line that can't be shared: instructions and labels.
    before: Vec<usize>,
    segment: usize,
}

/// A merge: `drop`'s last `n` instructions (and its end, if it has one) go, and it jumps into `keep`'s at `at`.
struct Merge {
    keep: usize,
    drop: usize,
    n: usize,
    saved: i32,
    extra: i32,
}

fn instr(l: &Line) -> Option<(Op, &Arg)> {
    l.as_instr()
}

fn code_label(l: &Line) -> Option<&str> {
    match &l.kind {
        Kind::Label(name) => Some(name),
        _ => None,
    }
}

fn label_of(arg: &Arg) -> Option<&str> {
    match arg {
        Arg::Sym(name, None) => Some(name),
        _ => None,
    }
}

/// A function's or a routine's entry, or the panic routine.
fn starts_unit(name: &str) -> bool {
    name == "rt_panic" || name.strip_suffix("_entry").is_some_and(|f| !f.is_empty())
}

fn is_call(arg: &Arg) -> bool {
    label_of(arg).is_some_and(|name| name.ends_with("_entry"))
}

/// Control never runs on past it: HRS, or a JMP that is not a call.
fn ends_run(op: Op, arg: &Arg) -> bool {
    op == Op::Hrs || (op == Op::Jmp && !is_call(arg))
}

fn names_99(arg: &Arg) -> bool {
    matches!(arg, Arg::Num(n) if n.parse::<i32>() == Ok(99))
}

/// The listing with runs that end alike sharing their ends. `keep` are the labels the loader needs (the phases'
/// starts), which stay where they are.
pub fn cross_jump(lines: &[Line], keep: &[String]) -> Vec<Line> {
    let slower = SLOWER.with(|s| s.get());
    let mut lines = lines.to_vec();
    let mut fresh = 0;
    while let Some(m) = best(&lines, keep, slower) {
        lines = apply(&lines, keep, &m, &mut fresh);
    }
    lines
}

/// What the tails are worked out from: where the code that may be merged ends, each line's segment, the lines
/// that can't be shared, and where each code label stands.
struct Shape {
    end: usize,
    segment: Vec<usize>,
    barrier: Vec<bool>,
    /// segments where nothing is merged: they read cell 99 past their entry
    reads_99: HashSet<usize>,
    /// each code label: the index of the first line after it that is not a label or a comment
    at: HashMap<String, usize>,
    /// code labels named other than by a JMP or TAC, or with an offset
    touched: HashSet<String>,
}

/// A jump table's first cell: `main_jt4`.
fn is_jump_table(name: &str) -> bool {
    name.rsplit_once("_jt")
        .is_some_and(|(_, n)| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))
}

fn skippable(l: &Line) -> bool {
    matches!(l.kind, Kind::Blank | Kind::Note(_))
}

fn shape(lines: &[Line]) -> Shape {
    // the runtime routines and the data are left alone
    let end = lines
        .iter()
        .position(|l| match &l.kind {
            Kind::Label(name) | Kind::Dat(Some(name), _) => name.starts_with("rt_"),
            Kind::DataStart => true,
            _ => false,
        })
        .unwrap_or(lines.len());
    let mut segment = vec![0; lines.len()];
    let mut s = 0;
    for (i, l) in lines.iter().enumerate() {
        let starts = match &l.kind {
            Kind::Phase(_) => true,
            Kind::Label(name) => starts_unit(name),
            _ => false,
        };
        if starts {
            s += 1;
        }
        segment[i] = s;
    }
    let labels: HashSet<&str> = lines[..end].iter().filter_map(code_label).collect();
    let mut touched: HashSet<String> = HashSet::new();
    // in a switch's jump table (`main_jt4: DAT main_bb3+800` and the cells after it, `lower::branches`), whose
    // entries are each a JMP to their label
    let mut table = false;
    for l in lines {
        table = match &l.kind {
            Kind::Dat(Some(name), _) => is_jump_table(name),
            Kind::Dat(None, _) => table,
            _ => false,
        };
        let (named, jump) = match &l.kind {
            Kind::Instr(op, arg) => (arg, matches!(op, Op::Jmp | Op::Tac)),
            Kind::Dat(_, Arg::Sym(name, Some(800))) if table && labels.contains(name.as_str()) => continue,
            Kind::Dat(_, arg) => (arg, false),
            _ => continue,
        };
        if let Arg::Sym(name, offset) = named {
            if labels.contains(name.as_str()) && (!jump || offset.is_some()) {
                touched.insert(name.clone());
            }
        }
    }
    let mut at: HashMap<String, usize> = HashMap::new();
    for (i, l) in lines[..end].iter().enumerate() {
        if let Some(name) = code_label(l) {
            let k = (i + 1..lines.len())
                .find(|&k| code_label(&lines[k]).is_none() && !skippable(&lines[k]))
                .unwrap_or(lines.len());
            at.insert(name.to_string(), k);
        }
    }
    // a label the program reads or writes as data, and the rest of its run, are run as they stand nowhere
    let mut barrier = vec![false; lines.len()];
    let mut inside = false;
    for (i, l) in lines.iter().enumerate() {
        if i >= end {
            barrier[i] = true;
            continue;
        }
        match &l.kind {
            Kind::Label(name) if touched.contains(name) => inside = true,
            Kind::Dat(..) | Kind::Table { .. } | Kind::Phase(_) | Kind::DataStart => barrier[i] = true,
            _ => {}
        }
        if inside {
            barrier[i] = true;
            if instr(l).is_some_and(|(op, arg)| ends_run(op, arg)) {
                inside = false;
            }
        }
    }
    let mut reads_99 = HashSet::new();
    for (i, l) in lines[..end].iter().enumerate() {
        let Some((op, arg)) = instr(l) else { continue };
        if !names_99(arg) {
            continue;
        }
        // a function's `CLA 99` straight after its entry keeps its return instruction
        let entry = op == Op::Cla
            && lines[..i]
                .iter()
                .rev()
                .find(|x| !skippable(x))
                .and_then(code_label)
                .is_some_and(starts_unit);
        if !entry {
            reads_99.insert(segment[i]);
        }
    }
    Shape {
        end,
        segment,
        barrier,
        reads_99,
        at,
        touched,
    }
}

/// May the label move to the kept copy: one only jumps name, not an entry, the program's start or a phase's.
fn movable(name: &str, shape: &Shape, keep: &[String]) -> bool {
    !starts_unit(name) && name != "main_bb0" && !keep.iter().any(|k| k == name) && !shape.touched.contains(name)
}

/// The lines before index `e`, latest first, back to the first that can't be shared.
fn before(lines: &[Line], shape: &Shape, e: usize) -> Vec<usize> {
    let mut out = vec![];
    let seg = shape.segment.get(e).copied().unwrap_or(usize::MAX);
    for i in (0..e).rev() {
        let l = &lines[i];
        if skippable(l) {
            continue;
        }
        if shape.barrier[i] || shape.segment[i] != seg {
            break;
        }
        match &l.kind {
            Kind::Label(name) if starts_unit(name) => break,
            Kind::Label(_) => out.push(i),
            Kind::Instr(op, arg) if ends_run(*op, arg) => break,
            Kind::Instr(..) => out.push(i),
            _ => break,
        }
    }
    out
}

fn tails(lines: &[Line], shape: &Shape) -> Vec<Tail> {
    let mut out = vec![];
    for i in 0..shape.end {
        let l = &lines[i];
        if shape.barrier[i] {
            continue;
        }
        if let Some((op, arg)) = instr(l) {
            if !ends_run(op, arg) {
                continue;
            }
            let exit = match (op, label_of(arg).and_then(|name| shape.at.get(name))) {
                (Op::Hrs, _) => Exit::Halt(arg.clone()),
                (_, Some(&at)) => Exit::At(at),
                _ => Exit::To(arg.clone()),
            };
            out.push(Tail {
                end: Some(i),
                exit,
                before: before(lines, shape, i),
                segment: shape.segment[i],
            });
        } else if let Some(name) = code_label(l) {
            // the first label of a group that code runs on into: the run before it goes there without a jump
            if starts_unit(name) || name == "main_bb0" {
                continue;
            }
            let prev = (0..i).rev().find(|&k| !skippable(&lines[k]));
            let Some(p) = prev else { continue };
            let falls = instr(&lines[p]).is_some_and(|(op, arg)| !ends_run(op, arg));
            if !falls || shape.barrier[p] || shape.segment[p] != shape.segment[i] {
                continue;
            }
            out.push(Tail {
                end: None,
                exit: Exit::At(shape.at[name]),
                before: before(lines, shape, i),
                segment: shape.segment[i],
            });
        }
    }
    out
}

/// Do two lines do the same: the same instruction, and the same check for the panic map.
fn same(a: &Line, b: &Line) -> bool {
    a.kind == b.kind && a.panic == b.panic
}

/// The merge of `drop` into `keep`, if it saves a cell.
fn merge(lines: &[Line], shape: &Shape, keep_labels: &[String], ts: &[Tail], k: usize, d: usize) -> Option<Merge> {
    let (keep, drop) = (&ts[k], &ts[d]);
    if keep.segment != drop.segment || keep.exit != drop.exit {
        return None;
    }
    if let (Some(a), Some(b)) = (keep.end, drop.end) {
        if !same(&lines[a], &lines[b]) {
            return None;
        }
    }
    // the instructions both end with, before their ends
    let ki: Vec<usize> = keep
        .before
        .iter()
        .copied()
        .filter(|&i| instr(&lines[i]).is_some())
        .collect();
    let mut n = 0;
    let mut stop = drop.before.len();
    for (pos, &i) in drop.before.iter().enumerate() {
        if let Some(name) = code_label(&lines[i]) {
            // (a label straight before the end is one the jump threading takes away)
            if n == 0 || !movable(name, shape, keep_labels) {
                stop = pos;
                break;
            }
            continue;
        }
        match ki.get(n) {
            Some(&j) if same(&lines[j], &lines[i]) => n += 1,
            _ => {
                stop = pos;
                break;
            }
        }
    }
    if n == 0 {
        return None;
    }
    // a run that loops back into the other's may reach the same lines: the copy that goes and the twins it leaves
    // must be apart
    let dropped: HashSet<usize> = drop
        .before
        .iter()
        .copied()
        .filter(|&i| instr(&lines[i]).is_some())
        .take(n)
        .collect();
    if ki[..n].iter().any(|j| dropped.contains(j)) {
        return None;
    }
    // the labels just before the drop's shared instructions, and what comes before them
    let mut seen = 0;
    let mut lead_end = drop.before.len();
    for (pos, &i) in drop.before.iter().enumerate() {
        if instr(&lines[i]).is_some() {
            seen += 1;
            if seen == n {
                lead_end = pos + 1;
                break;
            }
        }
    }
    let lead: Vec<usize> = drop.before[lead_end..stop.max(lead_end)]
        .iter()
        .copied()
        .take_while(|&i| code_label(&lines[i]).is_some())
        .collect();
    let top = lead.last().copied().unwrap_or(drop.before[lead_end - 1]);
    let falls_in = runs_into(lines, top);
    if lead.is_empty() && !falls_in {
        // nothing reaches it: leave dead code to the rules that drop it
        return None;
    }
    // (a jump writes cell 99, and the copy kept is at other cells: code that reads 99 sees the difference)
    if shape.reads_99.contains(&drop.segment) {
        return None;
    }
    let jump = falls_in;
    let (real_keep, real_drop) = (keep.end.is_some() as i32, drop.end.is_some() as i32);
    let saved = n as i32 + real_drop - jump as i32;
    let extra = jump as i32 + real_keep - real_drop;
    Some(Merge {
        keep: k,
        drop: d,
        n,
        saved,
        extra,
    })
}

/// Does anything run on into line `i`: an instruction before it that is not a jump or a halt, a label that stays
/// (something may jump there), anything else, or the program's start?
fn runs_into(lines: &[Line], i: usize) -> bool {
    match (0..i).rev().find(|&p| !skippable(&lines[p])) {
        Some(p) => !instr(&lines[p]).is_some_and(|(op, arg)| ends_run(op, arg)),
        None => true,
    }
}

/// The merge that saves the most cells (and of those, the one that adds least to run), within what is allowed.
fn best(lines: &[Line], keep: &[String], slower: bool) -> Option<(Merge, Vec<Tail>)> {
    let shape = shape(lines);
    let ts = tails(lines, &shape);
    let mut groups: HashMap<&Exit, Vec<usize>> = HashMap::new();
    for (i, t) in ts.iter().enumerate() {
        groups.entry(&t.exit).or_default().push(i);
    }
    let mut found: Option<Merge> = None;
    let mut keys: Vec<&&Exit> = groups.keys().collect();
    keys.sort_by_key(|e| format!("{e:?}"));
    for e in keys {
        let g = &groups[*e];
        for &k in g {
            for &d in g {
                if k == d {
                    continue;
                }
                let Some(m) = merge(lines, &shape, keep, &ts, k, d) else {
                    continue;
                };
                if m.saved < 1 || m.extra > 1 || (m.extra == 1 && !slower) {
                    continue;
                }
                if found.as_ref().is_none_or(|f| (m.saved, -m.extra) > (f.saved, -f.extra)) {
                    found = Some(m);
                }
            }
        }
    }
    found.map(|m| (m, ts))
}

/// A label no line defines or names.
fn fresh_label(lines: &[Line], stem: &str, fresh: &mut usize) -> String {
    let used: HashSet<String> = lines
        .iter()
        .filter_map(|l| match &l.kind {
            Kind::Label(n) | Kind::Dat(Some(n), _) | Kind::Table { label: n, .. } => Some(n.clone()),
            Kind::Instr(_, a) | Kind::Dat(None, a) => a.symbol().map(str::to_string),
            _ => None,
        })
        .collect();
    loop {
        let name = format!("{stem}_tail{fresh}");
        *fresh += 1;
        if !used.contains(&name) {
            return name;
        }
    }
}

/// The function a segment's code is in, for the new label's name: `main`, or NAME for `NAME_entry`.
fn stem_at(lines: &[Line], at: usize) -> String {
    lines[..at]
        .iter()
        .rev()
        .find_map(|l| code_label(l).and_then(|n| n.strip_suffix("_entry")))
        .unwrap_or("main")
        .to_string()
}

fn apply(lines: &[Line], keep_labels: &[String], (m, ts): &(Merge, Vec<Tail>), fresh: &mut usize) -> Vec<Line> {
    let shape = shape(lines);
    let (keep, drop) = (&ts[m.keep], &ts[m.drop]);
    let ki: Vec<usize> = keep
        .before
        .iter()
        .copied()
        .filter(|&i| instr(&lines[i]).is_some())
        .collect();
    let mut out_lines = lines.to_vec();
    let mut gone: HashSet<usize> = HashSet::new();
    let mut before_line: HashMap<usize, Vec<Line>> = HashMap::new();
    // walk the drop's shared lines, latest first: each instruction's twin in the kept copy, each label moved there
    let mut n = 0;
    let mut pos = 0;
    while n < m.n {
        let i = drop.before[pos];
        pos += 1;
        if code_label(&lines[i]).is_some() {
            // it stands before the drop's instruction just walked, so before the twin of that one
            before_line.entry(ki[n - 1]).or_default().insert(0, lines[i].clone());
            gone.insert(i);
            continue;
        }
        let j = ki[n];
        out_lines[j].wide |= lines[i].wide;
        gone.insert(i);
        n += 1;
    }
    let at = ki[m.n - 1];
    // the labels just before them go to the kept copy too (all can: `merge` stopped at one that can't)
    let mut lead = vec![];
    while let Some(&i) = drop.before.get(pos) {
        match code_label(&lines[i]) {
            Some(label) if movable(label, &shape, keep_labels) => lead.push(i),
            _ => break,
        }
        pos += 1;
    }
    let top = drop.before[..pos].last().copied().unwrap();
    let jump = runs_into(lines, top);
    let mut moved: Vec<Line> = lead.iter().rev().map(|&i| lines[i].clone()).collect();
    gone.extend(lead.iter().copied());
    // the jump in goes to a label there already, where there is one; else to a new one
    let above = (0..at)
        .rev()
        .find(|&p| !skippable(&lines[p]))
        .and_then(|p| code_label(&lines[p]));
    let name = match above.filter(|l| movable(l, &shape, keep_labels)) {
        Some(label) => label.to_string(),
        None => match moved.first().and_then(code_label) {
            Some(label) => label.to_string(),
            None => {
                let name = fresh_label(lines, &stem_at(lines, at), fresh);
                moved.push(Line::label(&name));
                name
            }
        },
    };
    before_line.entry(at).or_default().splice(0..0, moved);
    if let Some(e) = drop.end {
        gone.insert(e);
    }
    let first = drop.before[..pos]
        .iter()
        .rev()
        .find(|&&i| instr(&lines[i]).is_some())
        .copied()
        .unwrap();
    let mut out = Vec::with_capacity(lines.len());
    for (i, l) in out_lines.into_iter().enumerate() {
        if let Some(extra) = before_line.remove(&i) {
            out.extend(extra);
        }
        if i == first && jump {
            out.push(Line::commented(Op::Jmp, Arg::sym(&name), "the rest is shared"));
        }
        if !gone.contains(&i) {
            out.push(l);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::machine::{parse, print};

    fn run(text: &str, slower: bool) -> String {
        SLOWER.with(|s| s.set(slower));
        let out = print(&cross_jump(&parse(text).unwrap(), &[]));
        SLOWER.with(|s| s.set(false));
        out
    }

    fn cells(text: &str) -> usize {
        parse(text)
            .unwrap()
            .iter()
            .filter(|l| matches!(l.kind, Kind::Instr(..) | Kind::Dat(..)))
            .count()
    }

    /// Two arms that store and jump to one join, and a third that runs on into it: the stores are shared with the
    /// one that runs on, and the jumps become the jumps in.
    #[test]
    fn arms_share_their_store_with_the_one_that_runs_on() {
        let text =
            "main_bb0:\n        CLA a\n        TAC main_bb2\n        ADD b\n        STO c\n        JMP main_bb3\n\
            main_bb2:\n        SUB b\n        STO c\nmain_bb3:\n        OUT c\n        HRS 0\n";
        let out = run(text, false);
        assert_eq!(cells(&out), cells(text) - 1, "{out}");
        assert!(out.contains("JMP main_tail0"), "{out}");
    }

    /// Rows of ADDs one longer than the next are one row, entered at each arm's cell, only when slower is allowed.
    #[test]
    fn rows_of_adds_are_one_row() {
        let text = "main_bb0:\n        CLA d\n        TAC main_bb2\n        CLA c\n        ADD c\n        ADD c\n        ADD c\n\
            \x20       STO s\n        JMP main_bb3\nmain_bb2:\n        CLA c\n        ADD c\n        ADD c\n        STO s\n\
            \x20       JMP main_bb3\nmain_bb3:\n        OUT s\n        HRS 0\n";
        assert_eq!(run(text, false), print(&parse(text).unwrap()));
        let out = run(text, true);
        assert_eq!(cells(&out), cells(text) - 3, "{out}");
    }

    /// A run that ends at a label nothing runs into loses its copy without a jump: its label moves.
    #[test]
    fn a_label_moves_to_the_kept_copy() {
        let text =
            "main_bb0:\n        CLA d\n        TAC main_bb2\n        CLA c\n        ADD c\n        JMP main_bb3\n\
            main_bb2:\n        CLA c\n        ADD c\n        JMP main_bb3\nmain_bb3:\n        OUT s\n        HRS 0\n";
        let out = run(text, false);
        assert_eq!(cells(&out), cells(text) - 3, "{out}");
        assert!(!out.contains("JMP main_tail"), "{out}");
    }

    /// Code a constant names, a cell in the code, and code that reads cell 99 are left as they are.
    #[test]
    fn data_in_the_code_is_left_alone() {
        let named =
            "main_bb0:\n        CLA d\n        TAC main_bb2\n        CLA c\n        ADD c\n        JMP main_bb3\n\
            main_bb2:\n        CLA c\n        ADD c\n        JMP main_bb3\nmain_bb3:\n        OUT s\n        HRS 0\n\
            jmp_row:  DAT main_bb2+800\n";
        let out = run(named, true);
        assert_eq!(cells(&out), cells(named), "{out}");
        let cell =
            "main_bb0:\n        CLA d\n        TAC main_bb2\nmain_sm0: DAT 0\n        ADD c\n        JMP main_bb3\n\
            main_bb2:\nmain_sm1: DAT 0\n        ADD c\n        JMP main_bb3\nmain_bb3:\n        OUT s\n        HRS 0\n";
        // (the ADD after each cell may be shared; the cells themselves are not)
        let out = run(cell, true);
        assert!(
            out.contains("main_sm0: DAT 0\n") && out.contains("main_sm1: DAT 0\n"),
            "{out}"
        );
        let reads = "main_bb0:\n        CLA d\n        TAC main_bb2\n        CLA 99\n        ADD c\n        STO s\n\
            \x20       JMP main_bb3\nmain_bb2:\n        CLA c\n        ADD c\n        STO s\nmain_bb3:\n        OUT s\n\
            \x20       HRS 0\n";
        assert_eq!(run(reads, true), print(&parse(reads).unwrap()));
    }

    /// A jump table's entries are jumps to their labels: arms entered from one still share their ends.
    #[test]
    fn jump_table_entries_are_jumps() {
        let text = "main_bb0:\n        CLA d\n        ADD jmp_main_jt0\n        STO main_jt0_go\nmain_jt0_go: DAT 0\n\
            main_jt0: DAT main_bb1+800\n          DAT main_bb2+800\njmp_main_jt0: DAT main_jt0+800\nmain_bb1:\n\
            \x20       CLA c\n        ADD c\n        STO s\n        JMP main_bb3\nmain_bb2:\n        CLA c\n\
            \x20       ADD c\n        STO s\nmain_bb3:\n        OUT s\n        HRS 0\n";
        let out = run(text, true);
        assert!(cells(&out) < cells(text), "{out}");
    }

    /// A check's reason for the panic map is part of what a line does: two branches to rt_panic for different
    /// reasons are not one.
    #[test]
    fn checks_keep_their_reasons() {
        let text =
            "main_bb0:\n        CLA d\n        TAC main_bb2\n        CLA c\n        TAC rt_panic\n        STO s\n\
            \x20       JMP main_bb3\nmain_bb2:\n        CLA c\n        TAC rt_panic\n        STO s\nmain_bb3:\n\
            \x20       OUT s\n        HRS 0\nrt_panic:\n        OUT kn999\n        HRS rt_panic\n";
        let mut lines = parse(text).unwrap();
        for (k, l) in lines.iter_mut().filter(|l| l.to_panic()).enumerate() {
            l.panic = Some(format!("reason {k}"));
        }
        let out = cross_jump(&lines, &[]);
        let count = |ls: &[Line]| ls.iter().filter(|l| matches!(l.kind, Kind::Instr(..))).count();
        // only the store is shared
        assert_eq!(count(&out), count(&lines) - 1, "{}", print(&out));
    }

    /// A jump back to a label the run before it runs into: the two ends are the same lines, and nothing merges.
    #[test]
    fn a_loop_is_not_merged_with_itself() {
        let text = "main_bb0:\n        CLA c\n        ADD c\nmain_bb1:\n        JMP main_bb1\n";
        assert_eq!(run(text, true), print(&parse(text).unwrap()));
    }
}
