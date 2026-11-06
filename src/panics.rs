//! Panic maps: where a compiled program's checks are, so whatever runs its deck can say why it panicked.
//!
//! The map, its file format and the reading of it (`Map::parse`, `Map::panic`) are HEARTBURN's (`heartburn::panics`),
//! which burn uses: with NAME.map beside NAME.cqb, a run that panics says why, as Rust does. What is the compiler's
//! is making one (`map_of`), from where the checks went in a compiled program.
use crate::asm;
use crate::codes::Diag;
use crate::compile::machine::Line;
use crate::compile::Phases;
pub use heartburn::panics::{Check, Map, Panic};

/// The function a label starts: `NAME_entry` is NAME (a crate's `kit__arith__twice`, `kit::arith::twice`), and the
/// start of the program or of a later phase is main.
fn function_of(label: &str, phase: usize) -> Option<String> {
    if label == "start" || label == "main_bb0" || label == format!("p{phase}") {
        return Some("main".into());
    }
    let name = label.strip_suffix("_entry").filter(|name| !name.is_empty())?;
    Some(name.replace("__", "::"))
}

/// The map of a compiled program, from what `split_phases` made of it: each phase's lines are assembled as its deck
/// is (`tape::of`), so a line's cell here is its cell there.
pub fn map_of(program: &Phases) -> Result<Map, Diag> {
    let mut map = Map::default();
    for (index, code) in program.code.iter().enumerate() {
        let phase = index + 1;
        let lines: Vec<&Line> = if program.code.len() == 1 {
            code.iter().chain(&program.data).collect()
        } else {
            program.data.iter().chain(code).collect()
        };
        let text: String = lines.iter().map(|line| format!("{line}\n")).collect();
        let assembled = asm::assemble(3, &text)?;
        let mut starts: Vec<(i32, String)> = assembled
            .labels
            .iter()
            .filter_map(|(label, &at)| function_of(label, phase).map(|function| (at, function)))
            .collect();
        starts.sort();
        starts.dedup_by(|later, earlier| later.0 == earlier.0);
        if let Some(&at) = assembled.labels.get("rt_panic") {
            map.stops.push((phase, at));
        }
        for (line_number, line) in lines.iter().enumerate() {
            let cell = assembled.lines.get(line_number).copied().flatten();
            let (Some(message), Some(at)) = (&line.panic, cell) else {
                continue;
            };
            // a runtime routine's check is its caller's: rt_chk is left by cell 99, the others by their exit cell
            let routine = starts
                .iter()
                .rev()
                .find(|(start, _)| *start <= at)
                .map(|(_, function)| function.as_str());
            let caller = match routine {
                Some("rt_chk") => Some(99),
                Some(name) if name.starts_with("rt_") => assembled.labels.get(&format!("{name}_exit")).copied(),
                _ => None,
            };
            map.checks.push(Check {
                phase,
                at,
                caller,
                message: message.clone(),
            });
        }
        map.functions
            .extend(starts.into_iter().map(|(at, function)| (phase, at, function)));
    }
    Ok(map)
}
