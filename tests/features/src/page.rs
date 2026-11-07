//! SUPPORT.md from the results: a table for each section of each chapter, a row a program. Every status here is
//! what the runs did.

use crate::{config_name, write_whole, Outcome, Row, CHAPTERS};
use std::collections::BTreeMap;
use std::path::Path;

/// The letter a run prints as while the programs run.
pub fn letter(o: &Outcome) -> &'static str {
    match o {
        Outcome::Match => "M",
        Outcome::Differ => "D",
        Outcome::Refused => "R",
        Outcome::TooBig => "B",
        Outcome::Ice => "I",
        Outcome::NativeFailed => "N",
    }
}

/// A row's status, from its runs; the second part is the status's name in the summary.
fn status(row: &Row) -> (String, &'static str) {
    let has = |o: Outcome| row.runs.values().any(|r| r.outcome == o);
    let all = |o: Outcome| row.runs.values().all(|r| r.outcome == o);
    if has(Outcome::Differ) && row.differs.is_some() {
        return ("Differs from native Rust by design".into(), "Differs by design");
    }
    if has(Outcome::Differ) {
        return ("**Wrong answer** (a compiler bug)".into(), "Wrong answer");
    }
    if has(Outcome::Ice) {
        return ("**Compiler crash** (a bug)".into(), "Compiler crash");
    }
    if has(Outcome::TooBig) {
        return (
            "**The test is too big** (it doesn't fit in 96 cells)".into(),
            "Test too big",
        );
    }
    if has(Outcome::NativeFailed) {
        return (
            "**The test is broken** (it doesn't build natively)".into(),
            "Test broken",
        );
    }
    if all(Outcome::Match) {
        return ("Supported".into(), "Supported");
    }
    if all(Outcome::Refused) {
        return ("Refused".into(), "Refused");
    }
    let ok: Vec<&str> = row
        .runs
        .iter()
        .filter(|(_, r)| r.outcome == Outcome::Match)
        .map(|(k, _)| k.as_str())
        .collect();
    (format!("Supported only at {}", ok.join("; ")), "Supported in part")
}

/// The cells for a release build (opt-level 2, overflow checks off), then with overflow checks on.
fn cells(row: &Row) -> String {
    let parts: Vec<String> = [config_name("2", "off"), config_name("2", "on")]
        .iter()
        .filter_map(|k| row.runs.get(k).and_then(|r| r.cells))
        .map(|c| c.to_string())
        .collect();
    parts.join(" / ")
}

/// The compiler's messages, each once, with closures' paths cut out.
fn errors(row: &Row) -> String {
    let mut seen: Vec<String> = vec![];
    for r in row.runs.values() {
        let Some(e) = &r.error else { continue };
        let mut e = e.clone();
        while let Some(at) = e.find("{closure@") {
            let end = e[at..].find('}').map_or(e.len(), |n| at + n + 1);
            e.replace_range(at..end, "{closure}");
        }
        if e.chars().count() > 200 {
            e = e.chars().take(200).collect::<String>() + "...";
        }
        if !seen.contains(&e) {
            seen.push(e);
        }
    }
    seen.iter()
        .map(|e| format!("The compiler says: `{}`", e.replace('`', "'")))
        .collect::<Vec<_>>()
        .join(" ")
}

fn disposition(d: Option<&str>) -> &'static str {
    match d {
        Some("easy") => "Easy improvement",
        Some("demand") => "On demand",
        Some("never") => "Never",
        _ => "*not decided*",
    }
}

fn cell(text: &str) -> String {
    text.replace('|', "\\|")
}

pub fn write(dir: &Path, page: &Path, rows: &[Row], compiler: &str) {
    let mut out = vec![
        "# What cargo-cardiac compiles".to_string(),
        String::new(),
        "Every row is a small program here. Each is compiled for CARDIAC four ways (opt-level 1 and 2, with"
            .into(),
        "overflow checks on and off), run, and compared with the same program run natively on the same input. The"
            .into(),
        "status is what those runs did: **Supported** means CARDIAC printed what native Rust printed every time,"
            .into(),
        "**Refused** that the build stopped with an error every time. Cells are for a release build (opt-level 2,"
            .into(),
        "overflow checks off), then with overflow checks on. `./test_features.sh` remakes this page.".into(),
        String::new(),
        format!("Made with {compiler}."),
        String::new(),
    ];
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for r in rows {
        *counts.entry(status(r).1).or_default() += 1;
    }
    out.push("| Status | Rows |".into());
    out.push("|---|---|".into());
    for (k, v) in &counts {
        out.push(format!("| {k} | {v} |"));
    }
    out.push(String::new());

    for (chapter, title) in CHAPTERS {
        let mine: Vec<&Row> = rows.iter().filter(|r| r.chapter == chapter).collect();
        if mine.is_empty() {
            continue;
        }
        out.push(format!("## {title}"));
        out.push(String::new());
        // the sections in the order CHAPTER/SECTIONS gives, then any it doesn't name
        let order: Vec<String> = std::fs::read_to_string(dir.join(chapter).join("SECTIONS"))
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect();
        let mut sections: Vec<&str> = vec![];
        for r in &mine {
            if !sections.contains(&r.section.as_str()) {
                sections.push(&r.section);
            }
        }
        sections.sort_by_key(|s| order.iter().position(|o| o == s).unwrap_or(order.len()));
        for section in sections {
            out.push(format!("### {section}"));
            out.push(String::new());
            out.push("| Feature | Status | Cells | If not supported | Notes | Test |".into());
            out.push("|---|---|---|---|---|---|".into());
            for r in mine.iter().filter(|r| r.section == section) {
                let (st, _) = status(r);
                let disp = if st == "Supported" {
                    ""
                } else {
                    disposition(r.disposition.as_deref())
                };
                let mut notes = r.notes.clone();
                notes.extend(r.differs.clone());
                let e = errors(r);
                if !e.is_empty() {
                    notes.push(e);
                }
                let name = Path::new(&r.test).file_name().unwrap().to_string_lossy();
                out.push(format!(
                    "| {} | {} | {} | {} | {} | [{}]({}) |",
                    cell(&r.feature),
                    st,
                    cells(r),
                    disp,
                    cell(&notes.join(" ")),
                    name,
                    r.test
                ));
            }
            out.push(String::new());
        }
    }
    write_whole(page, &out.join("\n"));
}
