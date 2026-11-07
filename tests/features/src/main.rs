//! Runs every row's program for CARDIAC and natively, and writes SUPPORT.md from what happened.
//!
//!     cargo run --release [PATTERN]
//!
//! Each row is one program, CHAPTER/NAME.rs, whose header says what it shows:
//!
//!     //! feature: `a + b` on `i16`               what the row is about (Markdown)
//!     //! section: Arithmetic operators           where it goes in its chapter (CHAPTER/SECTIONS orders them)
//!     //! disposition: easy | demand | never     for a row that isn't supported everywhere: what we'd do about it
//!     //! note: ...                              anything the reader should know (as many lines as needed)
//!     //! input: 5 -3 0                          the cards inp() reads, the same natively
//!     //! differs: ...                           only where CARDIAC is meant to print something else: why
//!     //! edition: 2024                          Rust's edition, for both builds (2021 when not given)
//!     //! file: value.txt                        a file beside the row that it reads (include!), built beside it
//!     //! debug-assertions: on                   debug assertions on, for both builds (off when not given)
//!
//! The program prints with out() and reads with inp(), as the compiler's own test programs do. It is built for
//! CARDIAC with each setting in CONFIGS and run there, and built natively with the same overflow checks and run,
//! and the two outputs compared. A row's status comes from those runs, never from its header. Each chapter's
//! results go to results/CHAPTER.json; SUPPORT.md is made from all of them.
//!
//! CARGO_CARDIAC is the cargo-cardiac to test (default: the release build of the rust-cardiac this is part of).
//! SUPPORT_OUT is where results/ and SUPPORT.md go (default: this directory), so that a branch of the compiler can
//! be tested without touching the results committed here.
//!
//!     cargo run --release -- --compare BASE OUT
//!
//! compares the results in OUT/results with those in BASE/results, row by row: each row whose outcomes changed,
//! marked better (a setting moved toward MATCH) or WORSE (a setting moved away from it), and a count of each.

mod page;
mod run;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex};

/// The settings each program is compiled with: opt-level, and overflow checks.
pub const CONFIGS: [(&str, &str); 4] = [("1", "on"), ("1", "off"), ("2", "on"), ("2", "off")];

/// The chapters, in the Rust Reference's order: the directory, and the title.
pub const CHAPTERS: [(&str, &str); 9] = [
    ("types", "Types"),
    ("expressions", "Expressions"),
    ("patterns", "Patterns"),
    ("statements", "Statements"),
    ("items", "Items"),
    ("attributes", "Attributes"),
    ("macros", "Macros"),
    ("integers", "Integer methods"),
    ("library", "The rest of the core library"),
];

/// What one build and run did.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Outcome {
    /// CARDIAC printed what native Rust printed.
    Match,
    /// CARDIAC printed something else: a compiler bug, unless the row says it differs by design.
    Differ,
    /// The build stopped with an error.
    Refused,
    /// It compiles, but not into 96 cells: the test should be smaller.
    TooBig,
    /// The compiler crashed.
    Ice,
    /// The program doesn't build natively: the test is broken.
    NativeFailed,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Run {
    pub outcome: Outcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cells: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Row {
    /// The program, relative to the repository.
    pub test: String,
    pub chapter: String,
    pub feature: String,
    pub section: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disposition: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub notes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub differs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_assertions: Option<String>,
    /// Each setting's run, by "opt-level N, overflow checks X".
    pub runs: BTreeMap<String, Run>,
}

pub fn config_name(opt: &str, checks: &str) -> String {
    format!("opt-level {opt}, overflow checks {checks}")
}

/// The row a program's header describes, with no runs yet.
fn header(test: &str, chapter: &str, src: &str) -> Row {
    let mut row = Row {
        test: test.into(),
        chapter: chapter.into(),
        ..Row::default()
    };
    for line in src.lines() {
        let Some((key, value)) = line.strip_prefix("//! ").and_then(|l| l.split_once(": ")) else {
            continue;
        };
        let value = value.to_string();
        match key {
            "feature" => row.feature = value,
            "section" => row.section = value,
            "disposition" => row.disposition = Some(value),
            "note" => row.notes.push(value),
            "input" => row.input = Some(value),
            "differs" => row.differs = Some(value),
            "edition" => row.edition = Some(value),
            "file" => row.files.push(value),
            "debug-assertions" => row.debug_assertions = Some(value),
            _ => {}
        }
    }
    if row.feature.is_empty() {
        row.feature = Path::new(test).file_stem().unwrap().to_string_lossy().into_owned();
    }
    if row.section.is_empty() {
        row.section = "Other".into();
    }
    row
}

fn programs(dir: &Path) -> Vec<PathBuf> {
    let mut found = vec![];
    for (chapter, _) in CHAPTERS {
        let Ok(entries) = std::fs::read_dir(dir.join(chapter)) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "rs") {
                found.push(p);
            }
        }
    }
    found.sort();
    found
}

fn load(path: &Path) -> Vec<Row> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Writes a file whole, so that a reader (another run of this program) never sees half of it.
pub fn write_whole(path: &Path, text: &str) {
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, text).unwrap();
    std::fs::rename(&tmp, path).unwrap();
}

/// How good an outcome is, for --compare: a match is best, a wrong answer or a crash worst.
fn rank(o: &Outcome) -> u8 {
    match o {
        Outcome::Match => 3,
        Outcome::Refused | Outcome::TooBig => 2,
        Outcome::NativeFailed => 1,
        Outcome::Differ | Outcome::Ice => 0,
    }
}

fn compare(base: &Path, out: &Path) {
    let (mut better, mut worse) = (0, 0);
    for (chapter, _) in CHAPTERS {
        let file = |d: &Path| d.join("results").join(format!("{chapter}.json"));
        let before: BTreeMap<String, Row> = load(&file(base)).into_iter().map(|r| (r.test.clone(), r)).collect();
        for row in load(&file(out)) {
            let Some(old) = before.get(&row.test) else {
                println!("new     {}", row.test);
                continue;
            };
            let (mut up, mut down) = (false, false);
            for (k, run) in &row.runs {
                let was = old.runs.get(k).map_or(1, |r| rank(&r.outcome));
                up |= rank(&run.outcome) > was;
                down |= rank(&run.outcome) < was;
            }
            let marks = |r: &Row| {
                r.runs
                    .values()
                    .map(|x| page::letter(&x.outcome))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            if down {
                worse += 1;
                println!("WORSE   {}  {} -> {}", row.test, marks(old), marks(&row));
            } else if up {
                better += 1;
                println!("better  {}  {} -> {}", row.test, marks(old), marks(&row));
            }
        }
    }
    println!("{better} better, {worse} worse");
}

fn main() {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--compare") && args.len() == 4 {
        compare(Path::new(&args[2]), Path::new(&args[3]));
        return;
    }
    let cc = std::env::var("CARGO_CARDIAC")
        .map(PathBuf::from)
        .unwrap_or_else(|_| here.join("../../target/release/cargo-cardiac"));
    let pattern = std::env::args().nth(1).unwrap_or_default();
    let paths: Vec<PathBuf> = programs(&here)
        .into_iter()
        .filter(|p| p.to_string_lossy().contains(&pattern))
        .collect();

    // the programs, run a few at a time; each prints a line as it finishes, a letter a setting
    let queue = Mutex::new(paths.into_iter());
    let (tx, rx) = mpsc::channel();
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get().min(4));
    std::thread::scope(|s| {
        for _ in 0..workers {
            let (queue, tx, cc, here) = (&queue, tx.clone(), &cc, &here);
            s.spawn(move || {
                while let Some(path) = { queue.lock().unwrap().next() } {
                    let test = path.strip_prefix(here).unwrap().to_string_lossy().into_owned();
                    let chapter = path
                        .parent()
                        .unwrap()
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned();
                    let src = std::fs::read_to_string(&path).unwrap();
                    let mut row = header(&test, &chapter, &src);
                    for (opt, checks) in CONFIGS {
                        row.runs
                            .insert(config_name(opt, checks), run::row(cc, &path, &src, &row, opt, checks));
                    }
                    let marks: Vec<&str> = row.runs.values().map(|r| page::letter(&r.outcome)).collect();
                    println!("{}  {}", marks.join(" "), row.test);
                    tx.send(row).unwrap();
                }
            });
        }
    });
    drop(tx);

    // each chapter's results: the rows just run, over the ones from before whose programs are still there
    let mut fresh: BTreeMap<String, Vec<Row>> = BTreeMap::new();
    for row in rx {
        fresh.entry(row.chapter.clone()).or_default().push(row);
    }
    let out = std::env::var("SUPPORT_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| here.clone());
    std::fs::create_dir_all(out.join("results")).unwrap();
    for (chapter, rows) in fresh {
        let path = out.join("results").join(format!("{chapter}.json"));
        let mut all: BTreeMap<String, Row> = load(&path).into_iter().map(|r| (r.test.clone(), r)).collect();
        for r in rows {
            all.insert(r.test.clone(), r);
        }
        all.retain(|test, _| here.join(test).exists());
        let rows: Vec<Row> = all.into_values().collect();
        write_whole(&path, &(serde_json::to_string_pretty(&rows).unwrap() + "\n"));
    }

    let mut rows = vec![];
    for (chapter, _) in CHAPTERS {
        rows.extend(load(&out.join("results").join(format!("{chapter}.json"))));
    }
    rows.retain(|r| here.join(&r.test).exists());
    page::write(&here, &out.join("SUPPORT.md"), &rows, &run::compiler(&cc));
}
