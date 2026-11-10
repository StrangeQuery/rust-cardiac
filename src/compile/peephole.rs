//! Block layout and the peephole rules, on the machine IR. Each rule preserves the accumulator and memory a later
//! instruction could observe; labels block every rule, since another path may jump in.
//!
//! The rules see an unlabelled data cell (`          DAT 5`) as an instruction whose opcode is DAT: it sits in the
//! listing as one does. A rule deletes a line by leaving it blank, and blank lines are swept away after each rule.
use std::collections::{HashMap, HashSet};

use super::machine::{Arg, Kind, Line, Op};

/// An instruction as the rules see it: `op` is `None` for an unlabelled data cell.
#[derive(Clone, Copy)]
struct Instr<'a> {
    op: Option<Op>,
    arg: &'a Arg,
}

impl Instr<'_> {
    fn is(&self, op: Op) -> bool {
        self.op == Some(op)
    }

    fn is_any(&self, ops: &[Op]) -> bool {
        ops.iter().any(|&op| self.is(op))
    }
}

/// Does the program stop here for good: HRS with the bug on 00? HRS to any other cell stops for the operator to load
/// the tape it names, and the program goes on (`overlay()`).
fn ends_program(q: &Instr) -> bool {
    q.is(Op::Hrs) && matches!(q.arg, Arg::Num(n) if n.parse::<i32>() == Ok(0))
}

fn instr(l: &Line) -> Option<Instr<'_>> {
    match &l.kind {
        Kind::Instr(op, arg) => Some(Instr { op: Some(*op), arg }),
        Kind::Dat(None, arg) => Some(Instr { op: None, arg }),
        _ => None,
    }
}

/// A label on a line of its own, comment or not. A resource's table label is not one: it names data.
fn code_label(l: &Line) -> Option<&str> {
    match &l.kind {
        Kind::Label(name) => Some(name),
        _ => None,
    }
}

/// The name a line defines: a label, a table's label, or a labelled data cell.
fn defined(l: &Line) -> Option<&str> {
    match &l.kind {
        Kind::Label(name) | Kind::Table { label: name, .. } | Kind::Dat(Some(name), _) => Some(name),
        _ => None,
    }
}

/// A line with nothing in it to run: blank, a comment on its own, a phase marker, or the data's start.
fn is_blank(l: &Line) -> bool {
    matches!(l.kind, Kind::Blank | Kind::Note(_) | Kind::Phase(_) | Kind::DataStart)
}

fn is_phase(l: &Line) -> bool {
    matches!(l.kind, Kind::Phase(_))
}

fn deleted() -> Line {
    Line::new(Kind::Blank)
}

fn sweep(lines: &mut Vec<Line>) {
    lines.retain(|l| l.kind != Kind::Blank);
}

/// The symbol an operand names when it has no offset: what a label can stand for.
fn label_of(arg: &Arg) -> Option<&str> {
    match arg {
        Arg::Sym(name, None) => Some(name),
        _ => None,
    }
}

/// Does the operand name the symbol (`xs+2` names `xs`)? A number never names one.
fn arg_names(arg: &Arg, name: &str) -> bool {
    matches!(arg, Arg::Sym(s, _) if s == name)
}

/// Does the line define the symbol or name it in its operand?
fn mentions(l: &Line, name: &str) -> bool {
    match &l.kind {
        Kind::Label(label) | Kind::Table { label, .. } => label == name,
        Kind::Instr(_, arg) => arg_names(arg, name),
        Kind::Dat(label, arg) => label.as_deref() == Some(name) || arg_names(arg, name),
        Kind::Blank | Kind::Phase(_) | Kind::DataStart | Kind::Note(_) => false,
    }
}

/// `name` ends in one of `tags` and a number: `main_bb3` starts a block, `main_ret2` is a return point.
fn numbered_after(name: &str, tags: &[&str]) -> bool {
    let stem = name.trim_end_matches(|c: char| c.is_ascii_digit());
    stem.len() < name.len() && tags.iter().any(|t| stem.ends_with(t))
}

/// A scratch cell: `t0`, `t1`, ...
fn is_scratch(arg: &Arg) -> bool {
    let digits = label_of(arg).and_then(|name| name.strip_prefix('t')).unwrap_or("");
    !digits.is_empty() && digits.bytes().all(|c| c.is_ascii_digit())
}

/// A jump to a routine's or function's entry is a call, which comes back to the next cell.
fn is_call(arg: &Arg) -> bool {
    label_of(arg).is_some_and(|name| name.ends_with("_entry"))
}

/// Where a routine or function starts, or the panic routine.
fn starts_unit(name: &str) -> bool {
    name == "rt_panic" || name.strip_suffix("_entry").is_some_and(|f| !f.is_empty())
}

/// Scratch cells live within one MIR block (each block writes its scratch before reading it), so at a block's label
/// (name_bbN), or at the panic routine, none is live; at any other label or jump target it may be. (A call ends its
/// block.)
fn is_block_start(name: &str) -> bool {
    numbered_after(name, &["_bb", "_ret"]) || name == "rt_panic"
}

/// A cell named by its number, `00` or `99`.
fn is_cell_number(arg: &Arg) -> bool {
    matches!(arg, Arg::Num(n) if !n.starts_with('-'))
}

fn is_data_start(l: &Line) -> bool {
    matches!(l.kind, Kind::DataStart)
}

/// Block layout. Code falls through from one instruction to the next, so a piece of code that ends in `JMP L` can
/// have L's piece placed right after it and lose the jump. Pieces are cut where control never falls through (after
/// JMP or HRS; a JMP to a routine's entry is a call, and the routine comes back to the next cell, so it does not
/// cut), kept in order inside each phase, and chained greedily from main's entry.
pub fn layout(lines: &[Line]) -> Vec<Line> {
    // the data section stays where it is: from its start on, or the last line when it has no start marked
    let data_at = lines
        .iter()
        .position(is_data_start)
        .unwrap_or(lines.len().saturating_sub(1));
    let (code, data) = lines.split_at(data_at);
    // main's code, split at phase markers; everything from the first routine or function on stays as it is
    let unit_at = code
        .iter()
        .position(|l| defined(l).is_some_and(starts_unit))
        .unwrap_or(code.len());
    let (main, units) = code.split_at(unit_at);
    let mut phases: Vec<Vec<&Line>> = vec![vec![]];
    for l in main {
        if is_phase(l) {
            phases.push(vec![]);
        }
        phases.last_mut().unwrap().push(l);
    }
    let mut out: Vec<Line> = vec![];
    for phase in &phases {
        let pieces = cut(phase);
        for k in chain(&pieces) {
            out.extend(pieces[k].iter().map(|&l| l.clone()));
        }
    }
    out.extend(units.iter().chain(data).cloned());
    out
}

/// A phase's code in pieces, cut after each HRS and each JMP that is not a call.
fn cut<'a>(phase: &[&'a Line]) -> Vec<Vec<&'a Line>> {
    let mut pieces: Vec<Vec<&Line>> = vec![vec![]];
    for &l in phase {
        pieces.last_mut().unwrap().push(l);
        if instr(l).is_some_and(ends_run) {
            pieces.push(vec![]);
        }
    }
    pieces.retain(|p| !p.is_empty());
    pieces
}

/// Control never falls through it: HRS, or a JMP that is not a call.
fn ends_run(q: Instr) -> bool {
    q.is(Op::Hrs) || (q.is(Op::Jmp) && !is_call(q.arg))
}

/// The label a piece starts with: a code label before its first instruction or phase marker.
fn head<'a>(piece: &[&'a Line]) -> Option<&'a str> {
    for &l in piece {
        if let Some(name) = code_label(l) {
            return Some(name);
        }
        if instr(l).is_some() || is_phase(l) {
            return None;
        }
    }
    None
}

/// The label a piece's last instruction jumps to.
fn jump<'a>(piece: &[&'a Line]) -> Option<&'a str> {
    let last = piece.iter().rev().find_map(|&l| instr(l))?;
    Some(last).filter(|q| q.is(Op::Jmp)).and_then(|q| label_of(q.arg))
}

/// The order to place a phase's pieces in: the first one first, and each run follows its jumps as far as they go.
/// A last piece that ends in neither JMP nor HRS falls through into what comes after the phase's code (the first
/// peephole pass drops a JMP rt_panic just before rt_panic), so it stays last.
fn chain(pieces: &[Vec<&Line>]) -> Vec<usize> {
    let heads: Vec<Option<&str>> = pieces.iter().map(|p| head(p)).collect();
    let jumps: Vec<Option<&str>> = pieces.iter().map(|p| jump(p)).collect();
    let falls = |p: &[&Line]| !p.iter().rev().find_map(|&l| instr(l)).is_some_and(ends_run);
    let pinned = (pieces.len() > 1 && falls(&pieces[pieces.len() - 1])).then(|| pieces.len() - 1);
    // the unplaced piece k's jump lands on; the first piece, and a piece that starts a phase, stay where they are
    let target = |k: usize, placed: &HashSet<usize>| -> Option<usize> {
        let to = jumps[k]?;
        (1..pieces.len()).find(|&j| !placed.contains(&j) && heads[j] == Some(to) && !is_phase(pieces[j][0]))
    };
    // the pinned piece counts as placed from the start, and goes at the end
    let mut placed: HashSet<usize> = pinned.into_iter().collect();
    let mut order: Vec<usize> = vec![];
    while placed.len() < pieces.len() {
        // start a new run at a piece no other unplaced piece jumps to, so that the jump can become a fall-through
        let free: Vec<usize> = (0..pieces.len()).filter(|k| !placed.contains(k)).collect();
        let reached: HashSet<usize> = free.iter().filter_map(|&k| target(k, &placed)).collect();
        let start = if order.is_empty() {
            0
        } else {
            free.iter().copied().find(|k| !reached.contains(k)).unwrap_or(free[0])
        };
        let mut cur = Some(start);
        while let Some(c) = cur {
            if !placed.insert(c) {
                break;
            }
            order.push(c);
            cur = target(c, &placed);
        }
    }
    order.extend(pinned);
    order
}

/// The peephole rules on a listing, until none applies; `keep` are labels to keep whether or not anything names them.
pub fn peephole(lines: &[Line], keep: &[String]) -> Vec<Line> {
    let mut lines = lines.to_vec();
    let mut changed = true;
    while changed {
        changed = thread_jumps(&mut lines);
        changed |= drop_needless(&mut lines);
        changed |= rewrite_windows(&mut lines);
        sweep(&mut lines);
        changed |= output_loaded(&mut lines);
        sweep(&mut lines);
        changed |= input_stored(&mut lines);
        sweep(&mut lines);
        changed |= output_at_end(&mut lines);
        sweep(&mut lines);
        changed |= drop_unused_loads(&mut lines);
        sweep(&mut lines);
        changed |= drop_branch_reloads(&mut lines);
        sweep(&mut lines);
        changed |= drop_unused_labels(&mut lines, keep);
        if !changed {
            changed = drop_join_reloads(&mut lines, keep);
            sweep(&mut lines);
        }
    }
    drop_unused_cells(lines)
}

/// 1. jump threading: a label whose first instruction is JMP M stands for M (but JMP 00, the loader). A branch
///    threaded through to rt_panic takes the check of the JMP it now stands for (`Line::panic`). Not a call
///    (JMP NAME_entry): it comes back to the cell after the JMP that made it, so a jump threaded to it would come
///    back to the wrong place.
fn thread_jumps(lines: &mut [Line]) -> bool {
    let mut alias: HashMap<String, (Arg, Option<String>)> = HashMap::new();
    for (i, l) in lines.iter().enumerate() {
        let Some(m) = code_label(l) else { continue };
        let next = lines[i + 1..].iter().find(|x| !is_blank(x));
        // (not the loader, JMP 00: a hand-written phase may end with a JMP to it, which a TAC is not)
        let loader = |a: &Arg| matches!(a, Arg::Num(n) if n.parse::<i32>() == Ok(0));
        let jumps =
            |line: &&Line| instr(line).is_some_and(|ins| ins.is(Op::Jmp) && !is_call(ins.arg) && !loader(ins.arg));
        if let Some(jump) = next.filter(jumps) {
            alias.insert(m.to_string(), (instr(jump).unwrap().arg.clone(), jump.panic.clone()));
        }
    }
    let resolve = |name: &str| -> (Arg, Option<String>) {
        let (mut to, mut check) = (Arg::sym(name), None);
        let mut seen: HashSet<String> = HashSet::new();
        while let Some(name) = label_of(&to) {
            let Some((next, next_check)) = alias.get(name) else {
                break;
            };
            if !seen.insert(name.to_string()) {
                break;
            }
            (to, check) = (next.clone(), next_check.clone());
        }
        (to, check)
    };
    let mut changed = false;
    for l in lines.iter_mut() {
        let Kind::Instr(Op::Jmp | Op::Tac, arg) = &mut l.kind else {
            continue;
        };
        let Some(name) = label_of(arg)
            .filter(|name| alias.contains_key(*name))
            .map(str::to_string)
        else {
            continue;
        };
        let (to, check) = resolve(&name);
        *arg = to;
        if l.to_panic() {
            l.panic = check;
        }
        changed = true;
    }
    changed
}

/// The last instruction of these lines, in the phase the next line is in: none after a phase's start (the one
/// before it is another phase's).
fn before_in_phase(lines: &[Line]) -> Option<&Line> {
    lines
        .iter()
        .rev()
        .take_while(|l| !is_phase(l))
        .find(|l| instr(l).is_some())
}

/// Rules 2 to 5: instructions that do nothing, each judged against the listing as rewritten up to it.
fn drop_needless(lines: &mut Vec<Line>) -> bool {
    let mut out: Vec<Line> = Vec::with_capacity(lines.len());
    let mut changed = false;
    for (i, l) in lines.iter().enumerate() {
        // a branch to rt_panic with a check stays, though rt_panic comes next, when no line before it can take the
        // check: none at all (a main that only panics, or a phase that starts with the panic), or one that
        // has a check of its own
        let carried = || {
            !(l.to_panic() && l.panic.is_some()) || before_in_phase(&out).is_some_and(|earlier| earlier.panic.is_none())
        };
        let needless = instr(l).is_some_and(|ins| {
            already_held(&out, ins)
                || already_stored(&out, ins)
                || adds_zero(ins)
                || (jumps_to_next(&lines[i + 1..], ins) && carried())
                || unreachable(&out)
                || (never_taken(&out, &ins) && !l.to_panic())
        });
        if needless {
            changed = true;
            // a branch to rt_panic dropped as rt_panic comes next: the line before now falls into it, for that check
            if l.to_panic() && l.panic.is_some() {
                if let Some(before) = out
                    .iter_mut()
                    .rev()
                    .take_while(|earlier| !is_phase(earlier))
                    .find(|earlier| instr(earlier).is_some())
                {
                    if before.panic.is_none() {
                        before.panic = l.panic.clone();
                    }
                }
            }
        } else {
            out.push(l.clone());
        }
    }
    *lines = out;
    changed
}

/// 2. STO x or CLA x, then any TACs (which leave the accumulator alone), then CLA x: it already holds x (not after
///    a store of a value that may be past a card, `Line::wide`: the cell keeps less of it)
fn already_held(before: &[Line], ins: Instr) -> bool {
    if !ins.is(Op::Cla) {
        return false;
    }
    for o in before.iter().rev() {
        // comment lines are passed over; a blank line, a label or anything else ends the search (a label would let
        // another path in with something else in the accumulator)
        let Some(q) = instr(o) else {
            if matches!(o.kind, Kind::Note(_) | Kind::Phase(_) | Kind::DataStart) {
                continue;
            }
            return false;
        };
        if q.is_any(&[Op::Tac, Op::Out]) || (q.is(Op::Sto) && q.arg != ins.arg) {
            continue;
        }
        return q.is_any(&[Op::Sto, Op::Cla]) && q.arg == ins.arg && !o.wide;
    }
    false
}

/// Rule 2b alone, after the compiled stack has shared cells; then rule 12, for the load of a copy that went (a
/// call's result, `CLA f_0` then `STO x`, once x is f_0's cell, read again by nothing before the next load).
pub fn drop_self_copies(lines: &[Line]) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::with_capacity(lines.len());
    for l in lines {
        if !instr(l).is_some_and(|ins| already_stored(&out, ins)) {
            out.push(l.clone());
        }
    }
    if drop_unused_loads(&mut out) {
        sweep(&mut out);
    }
    out
}

/// 2b. CLA x or STO x, then any TACs and OUTs, then STO x: the cell already holds what the accumulator does (the
/// compiled stack gives a local and the argument cell it is passed in one cell)
fn already_stored(before: &[Line], ins: Instr) -> bool {
    if !ins.is(Op::Sto) {
        return false;
    }
    for o in before.iter().rev() {
        let Some(q) = instr(o) else {
            if matches!(o.kind, Kind::Note(_) | Kind::Phase(_) | Kind::DataStart) {
                continue;
            }
            return false;
        };
        if q.is_any(&[Op::Tac, Op::Out]) {
            continue;
        }
        return q.is_any(&[Op::Sto, Op::Cla]) && q.arg == ins.arg;
    }
    false
}

/// 3. SUB/ADD of the constant zero leaves the accumulator alone
fn adds_zero(ins: Instr) -> bool {
    ins.is_any(&[Op::Sub, Op::Add]) && label_of(ins.arg) == Some("k0")
}

/// 4. JMP to the very next label (rt_panic's too: rule 6 keeps its label while code falls into it)
fn jumps_to_next(after: &[Line], ins: Instr) -> bool {
    if !ins.is(Op::Jmp) {
        return false;
    }
    let next = after.iter().find(|x| x.kind != Kind::Blank);
    next.and_then(code_label).is_some_and(|m| label_of(ins.arg) == Some(m))
}

/// A TAC straight after a TAC, no label between: the accumulator is 0 or more when it runs, or the first would have
/// jumped, so it never jumps (rustc's second test of a range pattern, `13 <= y` after `y < 13` failed).
fn never_taken(before: &[Line], ins: &Instr) -> bool {
    ins.is(Op::Tac) && before.last().and_then(instr).is_some_and(|p| p.is(Op::Tac))
}

/// 5. unreachable: after JMP or HRS, code up to the next label never runs
fn unreachable(before: &[Line]) -> bool {
    before
        .last()
        .and_then(instr)
        .is_some_and(|p| p.is_any(&[Op::Jmp, Op::Hrs]))
}

/// What a rule over an instruction window does.
enum Edit {
    /// rule 8: the store becomes `ADD a`, and the CLA and ADD after it go
    Fold(Arg, usize, usize),
    /// these lines go
    Delete(Vec<usize>),
}

/// Rules 8 to 10 work on instruction windows; a code label is a barrier (another path may come in), a self-modifying
/// cell (`name: DAT`) is not: control only passes through it.
fn rewrite_windows(lines: &mut [Line]) -> bool {
    let mut changed = false;
    for i in 0..lines.len() {
        let Some(edit) = window_edit(lines, i) else { continue };
        match edit {
            Edit::Fold(a, cla, add) => {
                lines[i].kind = Kind::Instr(Op::Add, a);
                lines[cla] = deleted();
                lines[add] = deleted();
            }
            Edit::Delete(at) => {
                for k in at {
                    lines[k] = deleted();
                }
            }
        }
        changed = true;
    }
    changed
}

/// The edit to the window of the instruction at `i` and the (up to) four after it, before any code label.
fn window_edit(lines: &[Line], i: usize) -> Option<Edit> {
    let q0 = instr(&lines[i])?;
    let mut nx: Vec<(usize, Instr)> = vec![];
    for (j, l) in lines.iter().enumerate().skip(i + 1) {
        if nx.len() >= 4 || code_label(l).is_some() {
            break;
        }
        if let Some(q) = instr(l) {
            nx.push((j, q));
        }
    }
    let ops: Vec<Option<Op>> = nx.iter().map(|(_, q)| q.op).collect();
    // 8. STO t, CLA a, ADD t  ->  ADD a  (t a scratch cell read nowhere else)
    let wide = lines[i].wide; // the accumulator holds more than the cell stored keeps
    if q0.is(Op::Sto) && !wide && is_scratch(q0.arg) && ops.starts_with(&[Some(Op::Cla), Some(Op::Add)]) {
        let ((cla, a), (add, t)) = (nx[0], nx[1]);
        if t.arg == q0.arg && a.arg != q0.arg && !scratch_live(&lines[add + 1..], q0.arg) {
            return Some(Edit::Fold(a.arg.clone(), cla, add));
        }
    }
    // 9. a scratch store nothing reads
    if q0.is(Op::Sto) && is_scratch(q0.arg) && !scratch_live(&lines[i + 1..], q0.arg) {
        return Some(Edit::Delete(vec![i]));
    }
    // 10. after TAC (not taken) the accumulator still holds a op b: no need to work it out again
    if q0.is_any(&[Op::Sto, Op::Cla]) && !wide && nx.len() >= 4 {
        let [(_, arith), (_, tac), (cla, again), (redo, arith2)] = [nx[0], nx[1], nx[2], nx[3]];
        if arith.is_any(&[Op::Add, Op::Sub])
            && tac.is(Op::Tac)
            && again.is(Op::Cla)
            && again.arg == q0.arg
            && arith2.op == arith.op
            && arith2.arg == arith.arg
            && arith.arg != q0.arg
        {
            return Some(Edit::Delete(vec![cla, redo]));
        }
    }
    None
}

/// Does scratch cell t get read in these lines, before it is written again or a code label?
fn scratch_live(after: &[Line], t: &Arg) -> bool {
    for l in after {
        if let Some(name) = code_label(l) {
            return !is_block_start(name);
        }
        let Some(q) = instr(l) else { continue };
        if q.arg == t && q.is_any(&[Op::Cla, Op::Add, Op::Sub, Op::Out]) {
            return true;
        }
        if q.arg == t && q.is(Op::Sto) {
            return false;
        }
        if ends_program(&q) {
            return false;
        }
        // a phase's stop: the next phase may read it
        if q.is(Op::Hrs) {
            return true;
        }
        // a call: the routine never reads our scratch
        if q.is(Op::Jmp) && is_call(q.arg) {
            continue;
        }
        // (a runtime routine comes back mid-block, at an _rr label, scratch still live; a function at _ret, block
        // over)
        if q.is_any(&[Op::Jmp, Op::Tac]) && !label_of(q.arg).is_some_and(is_block_start) {
            return true;
        }
        if q.is(Op::Jmp) {
            return false;
        }
    }
    false
}

/// 11. CLA x, STO y, OUT y, where y is read and written nowhere else: CLA x, OUT x (the accumulator still holds x).
///     An address or instruction constant (`a_y: DAT y`, `rd_y: DAT y+100`) reaches y too, so it counts as a use,
///     unless the program halts right after the OUT without using it.
fn output_loaded(lines: &mut [Line]) -> bool {
    let mut refs: HashMap<Arg, usize> = HashMap::new();
    for q in lines.iter().filter_map(instr) {
        *refs.entry(q.arg.clone()).or_default() += 1;
    }
    // the cells address and instruction constants reach, each with the constants that reach it (a number reaches
    // none)
    let mut pointers: HashMap<String, Vec<String>> = HashMap::new();
    for l in lines.iter() {
        if let Kind::Dat(Some(name), Arg::Sym(target, _)) = &l.kind {
            pointers.entry(target.clone()).or_default().push(name.clone());
        }
    }
    let mut changed = false;
    for i in 2..lines.len() {
        let x = match (instr(&lines[i - 2]), instr(&lines[i - 1]), instr(&lines[i])) {
            (Some(a), Some(b), Some(c))
                if a.is(Op::Cla)
                    && b.is(Op::Sto)
                    && c.is(Op::Out)
                    && b.arg == c.arg
                    && refs.get(b.arg) == Some(&2)
                    && b.arg
                        .symbol()
                        .and_then(|y| pointers.get(y))
                        .is_none_or(|names| halts_unread(&lines[i + 1..], names))
                    && !is_cell_number(a.arg)
                    && !label_of(b.arg).is_some_and(|y| y.ends_with("_exit") || y.ends_with("_entry")) =>
            {
                a.arg.clone()
            }
            _ => continue,
        };
        lines[i - 1] = deleted();
        lines[i].kind = Kind::Instr(Op::Out, x);
        changed = true;
    }
    changed
}

/// 13. INP a, CLA a, STO b, where a is named nowhere else: INP b, CLA b. The card goes straight into the cell it
///     is copied to, as `let r = inp() as usize` reads it (the cast is the same cell value on CARDIAC). The load
///     stays for whatever reads the accumulator next; rule 6 drops it where nothing does.
fn input_stored(lines: &mut [Line]) -> bool {
    let mut changed = false;
    for i in 2..lines.len() {
        let (Some(a), Some(b), Some(c)) = (instr(&lines[i - 2]), instr(&lines[i - 1]), instr(&lines[i])) else {
            continue;
        };
        if !(a.is(Op::Inp) && b.is(Op::Cla) && c.is(Op::Sto) && a.arg == b.arg && b.arg != c.arg) {
            continue;
        }
        let (Some(t), Some(_)) = (a.arg.symbol(), c.arg.symbol()) else {
            continue;
        };
        if is_cell_number(a.arg) || matches!(a.arg, Arg::Sym(_, Some(_))) || matches!(c.arg, Arg::Sym(_, Some(_))) {
            continue;
        }
        let named = lines.iter().filter(|l| mentions(l, t) && defined(l) != Some(t)).count();
        if named != 2 || label_of(c.arg).is_some_and(|y| y.ends_with("_exit") || y.ends_with("_entry")) {
            continue;
        }
        let into = c.arg.clone();
        lines[i - 2].kind = Kind::Instr(Op::Inp, into.clone());
        lines[i - 1].kind = Kind::Instr(Op::Cla, into);
        lines[i] = deleted();
        changed = true;
    }
    changed
}

/// The program's last two instructions after the label `name`: `OUT y`, then HRS 00 (its end), with y.
fn output_then_end<'a>(lines: &'a [Line], name: &str) -> Option<&'a Arg> {
    let at = lines.iter().position(|l| code_label(l) == Some(name))?;
    let mut next = lines[at + 1..].iter().filter(|l| !is_blank(l));
    let (o, h) = (instr(next.next()?)?, instr(next.next()?)?);
    (o.is(Op::Out) && ends_program(&h)).then_some(o.arg)
}

/// 14. CLA x, STO y, JMP L, where L is OUT y and the program's end (HRS 00): OUT x, HRS 00. Nothing runs after the
///     HRS, so y's value is read only by that OUT. `out(if full { 1 } else { 0 })` compiles to two arms that each
///     store their value and join to output it; each arm now outputs its own and stops, as a program written by hand
///     does.
/// 15. CLA x, STO y, OUT y, HRS 00, one after another: OUT x, HRS 00, for the same reason (the other arm, once
///     nothing jumps to the label between them).
fn output_at_end(lines: &mut Vec<Line>) -> bool {
    let not_linkage = |a: &Arg| !label_of(a).is_some_and(|y| y.ends_with("_exit") || y.ends_with("_entry"));
    for i in 0..lines.len() {
        if i + 2 >= lines.len() {
            break;
        }
        let (Some(c), Some(st), Some(third)) = (instr(&lines[i]), instr(&lines[i + 1]), instr(&lines[i + 2])) else {
            continue;
        };
        if !(c.is(Op::Cla) && st.is(Op::Sto) && not_linkage(st.arg) && c.arg != st.arg) {
            continue;
        }
        let x = c.arg.clone();
        // 14
        if third.is(Op::Jmp) {
            let Some(target) = label_of(third.arg) else { continue };
            if output_then_end(lines, target) != Some(st.arg) {
                continue;
            }
            let comment = lines[i].comment.clone();
            let mut out = Line::new(Kind::Instr(Op::Out, x));
            out.comment = comment;
            lines[i] = out;
            lines[i + 1] = Line::new(Kind::Instr(Op::Hrs, Arg::Num("0".into())));
            lines[i + 2] = deleted();
            return true;
        }
        // 15
        let Some(Some(end)) = lines.get(i + 3).map(instr) else {
            continue;
        };
        if third.is(Op::Out) && third.arg == st.arg && ends_program(&end) {
            let comment = lines[i].comment.clone();
            let mut out = Line::new(Kind::Instr(Op::Out, x));
            out.comment = comment;
            lines[i] = out;
            lines[i + 1] = deleted();
            lines[i + 2] = deleted();
            return true;
        }
    }
    false
}

/// Does straight-line code reach HRS before any label, jump or use of one of these constants? Then nothing can read
/// through them again.
fn halts_unread(after: &[Line], names: &[String]) -> bool {
    for l in after {
        if matches!(l.kind, Kind::Label(_) | Kind::Table { .. }) {
            return false;
        }
        // a self-modifying cell (`name: DAT 0`) is run through, as rule 8 to 10's windows treat it
        let arg = match &l.kind {
            Kind::Instr(_, arg) | Kind::Dat(_, arg) => arg,
            _ => continue,
        };
        if names.iter().any(|n| arg.symbol() == Some(n.as_str())) {
            return false;
        }
        match instr(l) {
            Some(q) if ends_program(&q) => return true,
            Some(q) if q.is_any(&[Op::Jmp, Op::Tac, Op::Hrs]) => return false,
            _ => {}
        }
    }
    false
}

/// 12. a load nobody uses: CLA x, then only OUTs, then another CLA (or INP-only, or HRS)
fn drop_unused_loads(lines: &mut [Line]) -> bool {
    let mut changed = false;
    for i in 0..lines.len() {
        if instr(&lines[i]).is_some_and(|a| a.is(Op::Cla)) && load_unused(&lines[i + 1..]) {
            lines[i] = deleted();
            changed = true;
        }
    }
    changed
}

fn load_unused(after: &[Line]) -> bool {
    for l in after.iter().filter(|l| !is_blank(l)) {
        let Some(q) = instr(l) else { return false };
        if !q.is_any(&[Op::Out, Op::Inp]) {
            return q.is(Op::Cla) || ends_program(&q);
        }
    }
    false
}

/// 13. a block entered only by one TAC, right after STO x or CLA x, that starts CLA x: the accumulator holds x
///     already (nothing falls into the block: what comes before it is a JMP or HRS; and the STO is not `wide`)
fn drop_branch_reloads(lines: &mut [Line]) -> bool {
    let mut changed = false;
    for i in 0..lines.len() {
        let Some(label) = code_label(&lines[i]).map(str::to_string) else {
            continue;
        };
        if let Some(k) = branch_reload(lines, i, &label) {
            lines[k] = deleted();
            changed = true;
        }
    }
    changed
}

/// The CLA that starts the block labelled `label` at line `i`, when rule 13 drops it.
fn branch_reload(lines: &[Line], i: usize, label: &str) -> Option<usize> {
    // the one other line that defines or names the label
    let mut refs = (0..lines.len()).filter(|&k| k != i && mentions(&lines[k], label));
    let (Some(r), None) = (refs.next(), refs.next()) else {
        return None;
    };
    if !instr(&lines[r]).is_some_and(|t| t.is(Op::Tac) && label_of(t.arg) == Some(label)) {
        return None;
    }
    let before_tac = &lines[r.checked_sub(1)?];
    let held = instr(before_tac).filter(|b| b.is_any(&[Op::Sto, Op::Cla]) && !before_tac.wide)?;
    let before = lines[..i].iter().rev().find(|l| !is_blank(l));
    if !before.and_then(instr).is_some_and(|p| p.is_any(&[Op::Jmp, Op::Hrs])) {
        return None;
    }
    let (k, first) = lines.iter().enumerate().skip(i + 1).find(|(_, l)| !is_blank(l))?;
    instr(first)
        .is_some_and(|n| n.is(Op::Cla) && n.arg == held.arg)
        .then_some(k)
}

/// 14. a block that starts CLA x, where every way in leaves x in the accumulator already: each line that names its
///     label is a TAC or a JMP after STO x or CLA x (with only TACs, OUTs and other stores between), and so is what
///     falls into it, if anything does (two ways that each store x and go on to code that reads it). Not a call's
///     return point, a unit's entry or a label kept for the loader, which are come to with something else in the
///     accumulator. Tried only once the other rules are done, as it takes away the CLA that rule 10 starts from.
fn drop_join_reloads(lines: &mut [Line], keep: &[String]) -> bool {
    let mut changed = false;
    for i in 0..lines.len() {
        let Some(label) = code_label(&lines[i]).map(str::to_string) else {
            continue;
        };
        let entered = label == "main_bb0" || starts_unit(&label) || numbered_after(&label, &["_ret", "_rr"]);
        if entered || keep.contains(&label) {
            continue;
        }
        if let Some(k) = join_reload(lines, i, &label) {
            lines[k] = deleted();
            changed = true;
        }
    }
    changed
}

/// The CLA that starts the block labelled `label` at line `i`, when rule 14 drops it.
fn join_reload(lines: &[Line], i: usize, label: &str) -> Option<usize> {
    let (k, first) = lines.iter().enumerate().skip(i + 1).find(|(_, l)| !is_blank(l))?;
    let x = instr(first).filter(|n| n.is(Op::Cla))?.arg;
    for r in (0..lines.len()).filter(|&r| r != i && mentions(&lines[r], label)) {
        let jumps = instr(&lines[r]).is_some_and(|t| t.is_any(&[Op::Tac, Op::Jmp]) && label_of(t.arg) == Some(label));
        if !jumps || !holds(&lines[..r], x) {
            return None;
        }
    }
    let before = lines[..i]
        .iter()
        .rev()
        .find(|l| !matches!(l.kind, Kind::Blank | Kind::Note(_)))?;
    let falls = !instr(before).is_some_and(|p| p.is(Op::Hrs) || (p.is(Op::Jmp) && !is_call(p.arg)));
    (!falls || holds(&lines[..i], x)).then_some(k)
}

/// Does the accumulator hold x after these lines: the last STO x or CLA x, with only TACs, OUTs, stores elsewhere
/// and comments after it? A label or anything else between says no, and so does a store of a value that may be past
/// a card (`Line::wide`).
fn holds(before: &[Line], x: &Arg) -> bool {
    for o in before.iter().rev() {
        let Some(q) = instr(o) else {
            if matches!(o.kind, Kind::Note(_) | Kind::Blank) {
                continue;
            }
            return false;
        };
        if q.is_any(&[Op::Tac, Op::Out]) || (q.is(Op::Sto) && q.arg != x) {
            continue;
        }
        return q.is_any(&[Op::Sto, Op::Cla]) && q.arg == x && !o.wide;
    }
    false
}

/// 6. labels nothing refers to (code labels only: data cells are removed below); a call's return point is reached
///    through cell 99, not by name, so _ret labels always stay. rt_panic stays while the line before falls into it:
///    the panic map finds the routine by its label, and a run that reaches it unlabelled halts as if it ended
fn drop_unused_labels(lines: &mut Vec<Line>, keep: &[String]) -> bool {
    // named by an instruction, or by a constant (a jump built on `rt_times_end` is on `DAT rt_times_end+800`)
    let constants = lines.iter().filter_map(|l| match &l.kind {
        Kind::Dat(Some(_), arg) => arg.symbol(),
        _ => None,
    });
    let used: HashSet<String> = lines
        .iter()
        .filter_map(instr)
        .filter_map(|q| q.arg.symbol())
        .chain(constants)
        .map(str::to_string)
        .collect();
    // no instruction before it at all: the program starts there, and runs into it
    let entered = |i: usize| lines[..i].iter().rev().find_map(instr).is_none_or(|q| !ends_run(q));
    let unused = |i: usize, m: &str| {
        !used.contains(m)
            && m != "main_bb0"
            && !keep.iter().any(|k| k == m)
            && !numbered_after(m, &["_ret", "_rr"])
            && !(m == "rt_panic" && entered(i))
    };
    let before = lines.len();
    let gone: HashSet<usize> = (0..lines.len())
        .filter(|&i| code_label(&lines[i]).is_some_and(|m| unused(i, m)))
        .collect();
    let mut i = 0;
    lines.retain(|_| {
        i += 1;
        !gone.contains(&(i - 1))
    });
    lines.len() < before
}

/// 7. data cells nothing reads or writes: a cell is used if an instruction names it, or if an address constant
///    (DAT label) points at it
fn drop_unused_cells(lines: Vec<Line>) -> Vec<Line> {
    let mut used: HashSet<String> = HashSet::new();
    for l in &lines {
        if let Kind::Instr(_, arg) | Kind::Dat(_, arg) = &l.kind {
            if let Some(y) = arg.symbol() {
                used.insert(y.to_string());
            }
        }
    }
    let wanted = |l: &Line| match &l.kind {
        Kind::Dat(Some(name), _) => used.contains(name) || name.ends_with("_exit"),
        _ => true,
    };
    lines.into_iter().filter(wanted).collect()
}
