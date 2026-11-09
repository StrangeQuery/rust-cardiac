//! The holes in a grid laid out by tens (`nested::arrays_of_arrays`): the cells between the end of one row and the
//! start of the next. Nothing in the program reads or writes them (rustc checks every index against its own row's
//! length, and the lowering copies and builds a grid without them), so the program's other cells can live there, as
//! a program written by hand keeps its variables at the ends of its table's lines. This works on the finished
//! listing: a cell moved into a hole is the same line, with the same label, somewhere else in the data.
//!
//! What moves: an array whole, into a run of holes as long as it is, and a cell of its own whose address is never
//! taken (named in a `DAT`) nor reached as `name+N`, into any hole left. Never a resource's slot, nor a grid.
use std::collections::{HashMap, HashSet};

use super::machine::{Arg, Kind, Line};

/// Put the data's other cells in the grids' holes, each grid by the label of its first cell and its holes counted
/// from there; the listing as it was when there is nothing to move.
pub fn fill(lines: &[Line], grids: &[(String, Vec<usize>)]) -> Vec<Line> {
    let Some(start) = lines.iter().position(|l| matches!(l.kind, Kind::DataStart)) else {
        return lines.to_vec();
    };
    let label = |l: &Line| match &l.kind {
        Kind::Dat(Some(name), _) => Some(name.clone()),
        _ => None,
    };
    let at: HashMap<String, usize> = (start..lines.len())
        .filter_map(|i| label(&lines[i]).map(|n| (n, i)))
        .collect();
    // the holes, as runs of consecutive cells
    let mut runs: Vec<Vec<usize>> = vec![];
    let mut grid_labels: HashSet<&str> = HashSet::new();
    for (name, holes) in grids {
        let Some(&first) = at.get(name) else { continue };
        let free = |k: &usize| matches!(lines.get(first + k).map(|l| &l.kind), Some(Kind::Dat(None, _)));
        if !holes.iter().all(free) {
            continue;
        }
        grid_labels.insert(name);
        for &k in holes {
            match runs.last_mut() {
                Some(run) if *run.last().unwrap() + 1 == first + k => run.push(first + k),
                _ => runs.push(vec![first + k]),
            }
        }
    }
    if runs.is_empty() {
        return lines.to_vec();
    }
    // the cells a moved cell may not be: those whose address is taken, or reached from another's
    let mut fixed: HashSet<&str> = HashSet::new();
    for l in lines {
        match &l.kind {
            Kind::Dat(_, Arg::Sym(name, _)) => {
                fixed.insert(name);
            }
            Kind::Instr(_, Arg::Sym(name, Some(k))) if *k != 0 => {
                fixed.insert(name);
            }
            _ => {}
        }
    }
    // what may move: each labelled cell of the data, and the unlabelled cells after it (an array's)
    let mut arrays: Vec<(usize, usize)> = vec![];
    let mut cells: Vec<usize> = vec![];
    let mut i = start + 1;
    while i < lines.len() {
        let Some(name) = label(&lines[i]) else {
            i += 1;
            continue;
        };
        let mut n = 1;
        while matches!(lines.get(i + n).map(|l| &l.kind), Some(Kind::Dat(None, _))) {
            n += 1;
        }
        let resource = i > 0 && matches!(lines[i - 1].kind, Kind::Table { .. });
        if !resource && !grid_labels.contains(name.as_str()) {
            if n > 1 {
                arrays.push((i, n));
            } else if !fixed.contains(name.as_str()) {
                cells.push(i);
            }
        }
        i += n;
    }
    // the hole each moved line goes into, by the line it was
    let mut moved: HashMap<usize, usize> = HashMap::new();
    arrays.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    for (i, n) in arrays {
        // the shortest run it fits in, from its start
        let Some(run) = runs.iter_mut().filter(|r| r.len() >= n).min_by_key(|r| r.len()) else {
            continue;
        };
        for (k, hole) in run.drain(..n).enumerate() {
            moved.insert(i + k, hole);
        }
    }
    let mut free = runs.into_iter().flatten();
    for i in cells {
        let Some(hole) = free.next() else { break };
        moved.insert(i, hole);
    }
    if moved.is_empty() {
        return lines.to_vec();
    }
    let into: HashMap<usize, usize> = moved.iter().map(|(from, to)| (*to, *from)).collect();
    (0..lines.len())
        .filter(|i| !moved.contains_key(i))
        .map(|i| lines[*into.get(&i).unwrap_or(&i)].clone())
        .collect()
}
