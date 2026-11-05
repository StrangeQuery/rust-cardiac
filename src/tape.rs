//! Tapes, as HEARTBURN keeps them (`heartburn::tape`, re-exported here), and the tape and the six-column listings of
//! a compiled program: built from what `split_phases` made of it, which only the compiler knows.
pub use heartburn::tape::*;

use crate::asm;
use crate::compile::machine::{Kind, Line};
use crate::compile::Phases;

/// The slots a piece of assembly has: a resource's table label, then the array it labels, its cells the DAT lines
/// that follow until the next label.
fn slots(data: &[Line], labels: &asm::Program) -> Vec<Slot> {
    let mut found = vec![];
    for (i, line) in data.iter().enumerate() {
        let Kind::Table { label, resource } = &line.kind else {
            continue;
        };
        let first = data.get(i + 1).is_some_and(|l| matches!(l.kind, Kind::Dat(Some(_), _)));
        let rest = data
            .iter()
            .skip(i + 2)
            .take_while(|l| matches!(l.kind, Kind::Dat(None, _)))
            .count();
        let len = if first { 1 + rest } else { 0 };
        if let Some(&at) = labels.labels.get(label) {
            found.push(Slot {
                name: resource.clone(),
                at,
                len,
                bound: false,
            });
        }
    }
    found.sort_by_key(|t| t.at);
    found
}

/// One phase of a compiled program, assembled: its text, what the assembler made of it, the cells its tape loads
/// (`of`), and where it starts.
struct Assembled {
    text: String,
    program: asm::Program,
    cells: Vec<(i32, i32)>,
    start: i32,
}

/// Each phase of a compiled program, from what `split_phases` made of it, and the slots of the first. The first
/// phase loads onto cleared memory, so its cells that hold 0 are left off its tape (but not a slot's: those cards
/// are filled in when it is bound); a later phase loads over the one before, so all of its cells are punched.
fn assembled(r: &Phases) -> Result<(Vec<Assembled>, Vec<Slot>), String> {
    let mut phases = vec![];
    let mut found = vec![];
    let data = r.data_text();
    let data_end = asm::assemble(3, &data)?.end;
    for i in 0..r.code.len() {
        let c = r.code_text(i);
        let text = if r.code.len() == 1 {
            format!("{c}{data}")
        } else {
            format!("{data}\n{c}")
        };
        let p = asm::assemble(3, &text)?;
        if i == 0 {
            found = slots(&r.data, &p);
        }
        let in_slot = |a: i32| found.iter().any(|t| a >= t.at && a < t.at + t.len as i32);
        let cells: Vec<(i32, i32)> = p
            .cells
            .iter()
            .copied()
            .filter(|&(a, v)| {
                if i == 0 {
                    v != 0 || in_slot(a)
                } else {
                    r.code.len() == 1 || a >= data_end
                }
            })
            .collect();
        let start = if i == 0 {
            "start".to_string()
        } else {
            format!("p{}", i + 1)
        };
        let start = *p.labels.get(&start).ok_or(format!("no label {start}"))?;
        phases.push(Assembled {
            text,
            program: p,
            cells,
            start,
        });
    }
    Ok((phases, found))
}

/// The tape of a compiled program, its slots empty: a deck for each phase (see `assembled`).
pub fn of(r: &Phases) -> Result<Tape, String> {
    let (phases, slots) = assembled(r)?;
    Ok(Tape {
        phases: phases.iter().map(|p| asm::tape(p.start, &p.cells)).collect(),
        slots,
    })
}

/// A compiled program in Brian Stuart's six columns (`listing::six_columns`), one listing for each phase: the cells
/// its tape loads, and the first phase's that hold 0 too (they are the program's variables), each with its label,
/// its instruction or DATA (what the compiler wrote: DAT, and CUT and LDC, whose words are negative, are DATA), and
/// the comment the compiler gave its line.
pub fn listings(r: &Phases) -> Result<Vec<String>, String> {
    let (phases, _) = assembled(r)?;
    let mut out = vec![];
    for (i, ph) in phases.iter().enumerate() {
        let p = &ph.program;
        let cells: Vec<(usize, i32)> = if i == 0 {
            p.cells.iter().map(|&(a, v)| (a as usize, v)).collect()
        } else {
            ph.cells.iter().map(|&(a, v)| (a as usize, v)).collect()
        };
        let mut notes = crate::listing::Notes::default();
        let mut labels: Vec<(String, usize)> = p.labels.iter().map(|(l, &a)| (l.clone(), a as usize)).collect();
        labels.sort_by(|x, y| (x.1, &x.0).cmp(&(y.1, &y.0)));
        notes.labels = labels;
        for (line, at) in ph.text.split('\n').zip(&p.lines) {
            let Some(at) = at.map(|a| a as usize) else {
                continue;
            };
            let (code, comment) = line.split_once(';').unwrap_or((line, ""));
            let comment = comment.trim();
            if !comment.is_empty() {
                notes.comments.push((at, comment.to_string()));
            }
            let body = code.rsplit(':').next().unwrap_or(code).trim();
            if ["DAT", "CUT", "LDC"]
                .iter()
                .any(|d| body.split_whitespace().next() == Some(d))
            {
                notes.data.push(at);
            } else {
                notes.code.push(at);
            }
        }
        out.push(crate::listing::six_columns(&cells, Some(ph.start as usize), &notes));
    }
    Ok(out)
}
