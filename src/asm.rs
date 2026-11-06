//! The compiler's assembler, and the tape punch: the text the compiler writes (`compile`) made into cells. It was
//! HEARTBURN's until HEARTBURN took up the listings CARDIAC's own sources use (Brian Stuart's six columns, which
//! `cargo cardiac build --emit listing` writes); the compiler keeps it as the form it works in.
//!
//! One instruction or `DAT` per line, `label:` in front if wanted, `;` comments. An operand is a number, a label
//! or `label+N`. A tape is the two-card loader (002, 800), then an address card and a value card for every cell,
//! then Brian Stuart's start pair, 002 8xx, which puts a jump to the start in the loader's cell 02.
use std::collections::HashMap;

use crate::codes::{self, Diag};

pub struct Program {
    pub cells: Vec<(i32, i32)>,
    pub labels: HashMap<String, i32>,
    pub end: i32,
    /// for each line of the source, the cell it fills, if it fills one
    pub lines: Vec<Option<i32>>,
}

fn op(mn: &str) -> Option<i32> {
    Some(match mn {
        "INP" => 0,
        "CLA" => 1,
        "ADD" => 2,
        "TAC" => 3,
        "SFT" => 4,
        "OUT" => 5,
        "STO" => 6,
        "SUB" => 7,
        "JMP" => 8,
        "HRS" => 9,
        _ => return None,
    })
}

fn is_word(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// JavaScript's Number() for the operands that occur: optional sign, decimal digits, surrounding blanks.
fn number(t: &str) -> Option<i32> {
    let t = t.trim();
    if t.is_empty() {
        return Some(0);
    }
    t.parse::<i32>()
        .ok()
        .or_else(|| t.strip_prefix('+').and_then(|u| u.parse().ok()))
}

pub fn assemble(origin: i32, src: &str) -> Result<Program, Diag> {
    let mut labels = HashMap::new();
    let mut items: Vec<(i32, String)> = vec![];
    let mut at = origin;
    let mut lines = vec![];
    for raw in src.split('\n') {
        let line = raw.split(';').next().unwrap().trim();
        lines.push(None);
        if line.is_empty() {
            continue;
        }
        let mut body = line;
        if let Some(c) = line.find(':') {
            let (l, rest) = (&line[..c], &line[c + 1..]);
            if is_word(l) {
                labels.insert(l.to_string(), at);
                body = rest.trim();
            }
        }
        if !body.is_empty() {
            items.push((at, body.to_string()));
            *lines.last_mut().unwrap() = Some(at);
            at += 1;
        }
    }
    let arg = |t: &str| -> Result<i32, Diag> {
        if let Some(p) = t.find('+') {
            let (l, n) = (&t[..p], &t[p + 1..]);
            if is_word(l) && !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) {
                return labels
                    .get(l)
                    .map(|v| v + n.parse::<i32>().unwrap())
                    .ok_or_else(|| codes::internal(format!("unknown label {t}")));
            }
        }
        if let Some(v) = labels.get(t) {
            return Ok(*v);
        }
        number(t).ok_or_else(|| codes::internal(format!("unknown label {t}")))
    };
    let mut cells = vec![];
    for (a, body) in &items {
        let mut it = body.split_whitespace();
        let mn = it.next().unwrap();
        let x = it.next().unwrap_or("0");
        let v = match mn {
            // a cell holds -999..999: a constant past that is refused before here, and this says so if one isn't
            "DAT" => match arg(x)? {
                v @ -999..=999 => v,
                _ => return Err(codes::dat_past_card(x)),
            },
            "CUT" => -(600 + arg(x)?),
            "LDC" => -(100 + arg(x)?),
            _ => op(mn).ok_or_else(|| codes::internal(format!("unknown instruction {mn}")))? * 100 + arg(x)?,
        };
        cells.push((*a, v));
    }
    if at > 99 {
        return Err(codes::runs_past_98());
    }
    Ok(Program {
        cells,
        labels,
        end: at,
        lines,
    })
}

impl Program {
    pub fn label(&self, l: &str) -> i32 {
        *self.labels.get(l).unwrap_or_else(|| panic!("no label {l}"))
    }
}

/// Punch a self-loading tape: loader, (address, value) for each cell, then the pair 002 8xx: JMP start into cell 02.
pub fn tape(start: i32, cells: &[(i32, i32)]) -> Vec<i32> {
    let mut t = vec![2, 800];
    for &(a, v) in cells {
        t.push(a);
        t.push(v);
    }
    t.extend([2, 800 + start]);
    t
}
