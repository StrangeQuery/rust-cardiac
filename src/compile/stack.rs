//! A compiled stack: data cells shared by liveness. Two cells that are never live at the same time
//! become one, whichever functions they belong to: a loop counter used early and a total used late, or the
//! locals of two functions that are never active at once. No runtime stack (CARDIAC has no indexed addressing):
//! every access stays a plain direct address, so sharing costs no cell and no step.
//!
//! It works on the finished listing of a program of one phase, as register allocation does with cells for the
//! registers. The program's control flow runs across functions: a call goes to the callee's entry, and the
//! callee's return comes back after every call to it, so a cell is live wherever any path still reads it. Two
//! cells interfere when one is written while the other is live; cells that don't interfere are packed into as
//! few cells as they fit, each group's first cell standing for the rest.
//!
//! A cell takes part only when every use of it is a plain read or write of the cell itself (CLA, ADD, SUB, OUT,
//! STO, INP) and it is written somewhere: so not a constant or an address template (never written), an array or
//! a resource's slot (its other cells are reached through their address), a cell whose address is taken (named
//! in a `DAT`, or as `name+N`), or a routine's exit cell (in the code). A program whose control flow this doesn't
//! follow (a program in phases, a jump to a number) is left as it is.
use std::collections::{HashMap, HashSet};

use super::machine::{Arg, Kind, Line, Op};
use super::runtime::ROW;

/// The ops that read or write a cell as a value, the only uses a shared cell may have.
fn plain_use(op: Op) -> bool {
    matches!(op, Op::Cla | Op::Add | Op::Sub | Op::Out | Op::Sto | Op::Inp)
}

/// The data cells that can share: written somewhere, and only ever used plainly.
fn candidates(lines: &[Line], data_start: usize) -> Vec<String> {
    let mut excluded: HashSet<&str> = HashSet::new();
    let mut written: HashSet<&str> = HashSet::new();
    for line in lines {
        match &line.kind {
            Kind::Instr(op, Arg::Sym(name, offset)) => {
                if offset.is_some() || !plain_use(*op) {
                    excluded.insert(name);
                } else if matches!(op, Op::Sto | Op::Inp) {
                    written.insert(name);
                }
            }
            Kind::Dat(_, Arg::Sym(name, _)) => {
                excluded.insert(name);
            }
            _ => {}
        }
    }
    let data = &lines[data_start..];
    let mut found = vec![];
    for (index, line) in data.iter().enumerate() {
        let Kind::Dat(Some(name), initial) = &line.kind else {
            continue;
        };
        // an array: its first cell labelled, the rest after it (a resource's has a Table label before it)
        let array = data
            .get(index + 1)
            .is_some_and(|next| matches!(next.kind, Kind::Dat(None, _)));
        let resource = index > 0 && matches!(data[index - 1].kind, Kind::Table { .. });
        let zero = matches!(initial, Arg::Num(n) if n.parse::<i64>() == Ok(0));
        if !array && !resource && zero && written.contains(name.as_str()) && !excluded.contains(name.as_str()) {
            found.push(name.clone());
        }
    }
    found
}

/// The program's code as cells, one for each instruction or cell in it, and the labels on them.
struct Code<'a> {
    cells: Vec<&'a Line>,
    labels: HashMap<&'a str, usize>,
}

fn code(lines: &[Line]) -> Code<'_> {
    let mut cells = vec![];
    let mut labels = HashMap::new();
    let mut pending: Vec<&str> = vec![];
    for line in lines {
        match &line.kind {
            Kind::Label(name) | Kind::Table { label: name, .. } => pending.push(name),
            Kind::Instr(..) | Kind::Dat(..) => {
                if let Kind::Dat(Some(name), _) = &line.kind {
                    pending.push(name);
                }
                for name in pending.drain(..) {
                    labels.insert(name, cells.len());
                }
                cells.push(line);
            }
            _ => {}
        }
    }
    Code { cells, labels }
}

/// The cell a slot's built jump goes into the row before: `rt_x_end`, for a slot `rt_x_go`, `rt_x_go1`, ...
fn row_end(slot: &str, code: &Code) -> Option<usize> {
    let (routine, _) = slot.strip_prefix("rt_")?.rsplit_once("_go")?;
    let routine = format!("rt_{routine}");
    code.labels.get(format!("{routine}_end").as_str()).copied()
}

/// For each code cell, the cells that can run after it; None when the control flow is one this doesn't follow.
fn successors(code: &Code) -> Option<Vec<Vec<usize>>> {
    // the function each cell belongs to: from its NAME_entry label to the next (before the first, main)
    let mut function_of = vec![String::new(); code.cells.len()];
    let mut entries: Vec<(usize, &str)> = code
        .labels
        .iter()
        .filter_map(|(label, &at)| label.strip_suffix("_entry").map(|function| (at, function)))
        .collect();
    entries.sort();
    let mut current = "main";
    for (at, function) in function_of.iter_mut().enumerate() {
        if let Some(&(_, name)) = entries.iter().rev().find(|(start, _)| *start == at) {
            current = name;
        }
        *function = current.to_string();
    }
    // where each function's calls come back to: the cell after each JMP NAME_entry
    let mut returns: HashMap<&str, Vec<usize>> = HashMap::new();
    for (at, cell) in code.cells.iter().enumerate() {
        if let Kind::Instr(Op::Jmp, Arg::Sym(target, None)) = &cell.kind {
            if let Some(function) = target.strip_suffix("_entry") {
                returns.entry(function).or_default().push(at + 1);
            }
        }
    }
    let count = code.cells.len();
    let mut after = Vec::with_capacity(count);
    for (at, cell) in code.cells.iter().enumerate() {
        let next: Vec<usize> = if at + 1 < count { vec![at + 1] } else { vec![] };
        let back = returns.get(function_of[at].as_str()).cloned().unwrap_or_default();
        let target = |name: &str| code.labels.get(name).copied();
        let followed = match &cell.kind {
            Kind::Instr(Op::Hrs, _) => vec![],
            Kind::Instr(Op::Jmp, Arg::Sym(name, None)) if name.ends_with("_exit") => back,
            Kind::Instr(Op::Jmp, Arg::Sym(name, None)) => vec![target(name)?],
            Kind::Instr(Op::Jmp, Arg::Num(n)) if n == "99" => back,
            Kind::Instr(Op::Tac, Arg::Sym(name, None)) => next.into_iter().chain(target(name)).collect(),
            Kind::Instr(Op::Tac, Arg::Num(n)) if n == "99" => next.into_iter().chain(back).collect(),
            Kind::Instr(Op::Jmp | Op::Tac, _) => return None,
            // a routine's exit cell holds the jump back to its caller; any other cell in the code is a
            // self-modifying slot, an instruction built and stored there, which runs and carries on
            Kind::Dat(Some(name), _) if name.ends_with("_exit") => back,
            // or a jump built into a routine's row (`rt_skip_go` into the row ending at `rt_skip_end`): to any
            // cell of it, and on (the row comes back after it)
            Kind::Dat(Some(name), _) if row_end(name, code).is_some() => {
                let end = row_end(name, code).unwrap();
                next.into_iter().chain(end.saturating_sub(ROW)..=end).collect()
            }
            _ => next,
        };
        after.push(followed);
    }
    Some(after)
}

/// Share the data cells that are never live at the same time; the listing as it was when there are none to share,
/// or the program is one this doesn't follow.
pub fn share_cells(lines: &[Line]) -> Vec<Line> {
    let unchanged = || lines.to_vec();
    if lines.iter().any(|line| matches!(line.kind, Kind::Phase(_))) {
        return unchanged();
    }
    let Some(data_start) = lines.iter().position(|line| matches!(line.kind, Kind::DataStart)) else {
        return unchanged();
    };
    let names = candidates(lines, data_start);
    if names.len() < 2 {
        return unchanged();
    }
    let number: HashMap<&str, usize> = names.iter().enumerate().map(|(i, name)| (name.as_str(), i)).collect();
    let code = code(&lines[..data_start]);
    let Some(after) = successors(&code) else {
        return unchanged();
    };
    // what each code cell reads and writes, of the candidates
    let access = |cell: &Line| -> (Option<usize>, Option<usize>) {
        match &cell.kind {
            Kind::Instr(op, Arg::Sym(name, None)) => {
                let Some(&n) = number.get(name.as_str()) else {
                    return (None, None);
                };
                if matches!(op, Op::Sto | Op::Inp) {
                    (None, Some(n))
                } else {
                    (Some(n), None)
                }
            }
            _ => (None, None),
        }
    };
    // liveness, backwards to a fixed point
    let count = code.cells.len();
    let mut live_in: Vec<HashSet<usize>> = vec![HashSet::new(); count];
    let mut live_out: Vec<HashSet<usize>> = vec![HashSet::new(); count];
    let mut changed = true;
    while changed {
        changed = false;
        for at in (0..count).rev() {
            let out: HashSet<usize> = after[at].iter().flat_map(|&s| live_in[s].iter().copied()).collect();
            let (read, write) = access(code.cells[at]);
            let mut into: HashSet<usize> = out.iter().copied().filter(|&n| Some(n) != write).collect();
            into.extend(read);
            if into != live_in[at] || out != live_out[at] {
                live_in[at] = into;
                live_out[at] = out;
                changed = true;
            }
        }
    }
    // a cell is written while another is live: they interfere. Cells read before any write rely on starting at 0,
    // all together at the start.
    let mut interferes: Vec<HashSet<usize>> = vec![HashSet::new(); names.len()];
    for (cell, live_after) in code.cells.iter().zip(&live_out) {
        if let (_, Some(written)) = access(cell) {
            for &other in live_after.iter().filter(|&&other| other != written) {
                interferes[written].insert(other);
                interferes[other].insert(written);
            }
        }
    }
    let entry = ["start", "main_bb0"]
        .iter()
        .find_map(|label| code.labels.get(label).copied())
        .unwrap_or(0);
    if let Some(at_start) = live_in.get(entry) {
        for &first in at_start {
            for &second in at_start.iter().filter(|&&second| second != first) {
                interferes[first].insert(second);
            }
        }
    }
    // pack: the most constrained first (ties in listing order), each into the first group it interferes with none of
    let mut order: Vec<usize> = (0..names.len()).collect();
    order.sort_by_key(|&n| (std::cmp::Reverse(interferes[n].len()), n));
    let mut groups: Vec<Vec<usize>> = vec![];
    for n in order {
        match groups
            .iter_mut()
            .find(|group| group.iter().all(|member| !interferes[n].contains(member)))
        {
            Some(group) => group.push(n),
            None => groups.push(vec![n]),
        }
    }
    if groups.len() == names.len() {
        return unchanged();
    }
    // each group's first cell in the listing stands for the rest
    let mut stands_for: HashMap<&str, &str> = HashMap::new();
    let mut shared_by: HashMap<&str, Vec<&str>> = HashMap::new();
    for group in &mut groups {
        group.sort();
        let kept = names[group[0]].as_str();
        for &other in &group[1..] {
            stands_for.insert(names[other].as_str(), kept);
            shared_by.entry(kept).or_default().push(names[other].as_str());
        }
    }
    let mut shared = Vec::with_capacity(lines.len());
    for line in lines {
        let mut line = line.clone();
        match &mut line.kind {
            Kind::Dat(Some(name), _) if stands_for.contains_key(name.as_str()) => continue,
            Kind::Dat(Some(name), _) => {
                if let Some(others) = shared_by.get(name.as_str()) {
                    line.note(&format!("also {} (never live at once)", others.join(", ")));
                }
            }
            Kind::Instr(_, Arg::Sym(name, None)) => {
                if let Some(kept) = stands_for.get(name.as_str()) {
                    *name = kept.to_string();
                }
            }
            _ => {}
        }
        shared.push(line);
    }
    shared
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::machine::{parse, print};

    /// The listing after sharing, as text, from a listing given as text.
    fn shared(listing: &str) -> String {
        print(&share_cells(&parse(listing).unwrap()))
    }

    /// The data cells a listing ends with.
    fn data_cells(listing: &str) -> Vec<String> {
        listing
            .lines()
            .skip_while(|line| !line.starts_with("; ---- locals"))
            .filter_map(|line| {
                line.split(':')
                    .next()
                    .filter(|name| !name.starts_with(' ') && !name.starts_with(';'))
            })
            .map(str::to_string)
            .collect()
    }

    const TWO_LOOPS: &str = "\
start:
        INP main_1
        CLA main_1
        OUT main_1
        INP main_2
        CLA main_2
        OUT main_2
        HRS 0
; ---- locals and constants ----
main_1:   DAT 0
main_2:   DAT 0
";

    #[test]
    fn cells_never_live_at_once_share() {
        let after = shared(TWO_LOOPS);
        assert_eq!(data_cells(&after), vec!["main_1"]);
        assert!(after.contains("INP main_1\n        CLA main_1\n        OUT main_1\n        INP main_1"));
        assert!(after.contains("also main_2 (never live at once)"));
    }

    #[test]
    fn cells_live_at_once_keep_their_own() {
        // main_1 is read again after main_2 is written
        let both = TWO_LOOPS.replace("        HRS 0\n", "        OUT main_1\n        HRS 0\n");
        assert_eq!(data_cells(&shared(&both)), vec!["main_1", "main_2"]);
        // across a branch: main_1 is still needed where the TAC goes
        let branch = "\
start:
        INP main_1
        CLA main_1
        TAC later
        INP main_2
        OUT main_2
        HRS 0
later:
        OUT main_1
        HRS 0
; ---- locals and constants ----
main_1:   DAT 0
main_2:   DAT 0
";
        let after = shared(branch);
        // main_2 is written only where main_1 is no longer needed: they share
        assert_eq!(data_cells(&after), vec!["main_1"]);
        let crossing = branch.replace(
            "        INP main_2\n        OUT main_2\n",
            "        INP main_2\n        OUT main_2\n        CLA main_1\n        TAC later\n",
        );
        assert_eq!(data_cells(&shared(&crossing)), vec!["main_1", "main_2"]);
    }

    #[test]
    fn a_callee_s_cells_and_a_caller_s_live_across_the_call_dont_share() {
        let call = "\
start:
        INP main_1
        JMP f_entry
        OUT main_1
        HRS 0
f_entry:
        CLA 99
        STO f_exit
        INP f_1
        OUT f_1
        JMP f_exit
f_exit:   DAT 0
; ---- locals and constants ----
main_1:   DAT 0
f_1:      DAT 0
";
        assert_eq!(data_cells(&shared(call)), vec!["main_1", "f_1"]);
        // with main_1 dead before the call, they share: f's cells and main's are one block when they can be
        let dead = call.replace(
            "        JMP f_entry\n        OUT main_1\n",
            "        OUT main_1\n        JMP f_entry\n",
        );
        assert_eq!(data_cells(&shared(&dead)), vec!["main_1"]);
    }

    #[test]
    fn cells_reached_through_their_address_or_never_written_keep_their_own() {
        // main_2's address is in a DAT: a pointer to it, or a template for reading it
        let taken = TWO_LOOPS.replace("main_2:   DAT 0\n", "main_2:   DAT 0\np:        DAT main_2\n");
        assert_eq!(data_cells(&shared(&taken)), vec!["main_1", "main_2", "p"]);
        let offset = TWO_LOOPS.replace("        HRS 0\n", "        OUT main_1+1\n        HRS 0\n");
        assert_eq!(data_cells(&shared(&offset)), vec!["main_1", "main_2"]);
        // a program in phases is left as it is
        let phased = TWO_LOOPS.replace("        HRS 0\n", "        HRS 02\n;@@PHASE p2\np2:\n        HRS 0\n");
        assert_eq!(shared(&phased), print(&parse(&phased).unwrap()));
    }
}
