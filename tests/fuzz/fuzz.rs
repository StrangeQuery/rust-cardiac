//! The fuzzer: each program in tests/fuzz/targets run on generated inputs, natively and on CARDIAC, in
//! every setting the compiler's tests use (opt-level 1, 2, 3 and s, overflow checks on and off). Every run must
//! print the same cards and end the same way both ways.
//!
//! The programs are built in a package of the fuzzer's own, target/fuzz/pkg, so that nothing beside their
//! sources changes: one deck per program per setting, kept in target/fuzz/SETTING/. The inputs are numbers a card
//! holds, in each input's range, a quarter of them at an edge (the range's ends, 0, 1, -1); the same inputs go
//! to every setting. A run that differs is told apart from a miscompile when CARDIAC built with overflow checks
//! says a value didn't fit on a card, or names an overflow native i16 didn't reach: native i16 goes on to 32767,
//! a card stops at 999, and without the checks each keeps what it keeps (the README's "past a card"). Anything
//! else is a failure, shrunk to the smallest input that still differs, and printed with how to run it again.
//!
//!     sh fuzz.sh [--runs N] [--seed S] [--only NAME]...
//!
//! With --generate the programs are the fuzzer's own (stage 2, generate.rs): N small programs in the Rust the
//! compiler supports, made from the seed, built a batch at a time and each run on --runs inputs (40) in every
//! setting. The generator runs each program too, with numbers of any size, which says surely whether a value
//! passed a card. A refusal other than "too big", a compiler that crashes or hangs, and a run that differs with no
//! value past a card are findings, shrunk (input and program) and saved in target/fuzz/findings/; a program rustc
//! refuses is the generator's mistake, counted. --repro builds one file in every setting and runs it on the input
//! after `--`, to check a finding again; --tool gives another cargo-cardiac for either, such as an older build.
//!
//!     sh fuzz.sh --generate N [--seed S] [--start K] [--batch B] [--runs N] [--no-shrink] [--tool PATH]
//!     sh fuzz.sh --repro FILE.rs [--tool PATH] [-- INPUT...]
//!
//! Built by fuzz.sh with rustc alone: no crates, so it needs nothing the compiler's own build doesn't.
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

mod generate;

/// The settings: a profile name, its opt-level as Cargo writes it, and whether overflow checks are on.
const SETTINGS: [(&str, &str, bool); 8] = [
    ("opt1", "1", true),
    ("opt1-unchecked", "1", false),
    ("opt2", "2", true),
    ("opt2-unchecked", "2", false),
    ("opt3", "3", true),
    ("opt3-unchecked", "3", false),
    ("opts", "\"s\"", true),
    ("opts-unchecked", "\"s\"", false),
];

/// A run that takes longer is stopped and counted as a failure (a miscompile can loop for ever).
const TIMEOUT: Duration = Duration::from_secs(10);

/// What CARDIAC built with overflow checks says when a value passes what a card holds.
const PAST_A_CARD: &str = "doesn't fit on a card";

/// Whether CARDIAC built with overflow checks stopped for a value past a card: it says so, or (an operation whose
/// check is rustc's own) names the overflow, which native i16 arithmetic, going on to 32767, didn't reach.
fn past_a_card(why: &str) -> bool {
    why.contains(PAST_A_CARD) || why.contains("with overflow")
}

/// What native out() says when it is given a value no card holds (a value CARDIAC's cell has already cut down).
const NOT_A_CARD: &str = "a card holds a number from -999 to 999";

/// N to M numbers from lo to hi.
#[derive(Clone)]
struct Inputs {
    min: usize,
    max: usize,
    lo: i64,
    hi: i64,
}

struct Target {
    name: String,
    source: PathBuf,
    inputs: Vec<Inputs>,
    /// Marked `exact`: no value the program keeps passes a card, so without overflow checks every run must match
    /// native Rust, whatever a build with them says.
    exact: bool,
}

/// One input: its value and the range it was drawn from, which shrinking keeps it in.
#[derive(Clone, Copy)]
struct Value {
    v: i64,
    lo: i64,
    hi: i64,
}

/// How a run ended: the cards it printed, its exit status, and what it said on standard error (CARDIAC: the last
/// line, why it panicked).
#[derive(Clone, PartialEq)]
struct Run {
    cards: String,
    status: i32,
    why: String,
}

impl Run {
    fn show(&self) -> String {
        let cards: Vec<&str> = self.cards.split_whitespace().collect();
        let status = match self.status {
            -1 => "timed out".to_string(),
            s => format!("exit {s}"),
        };
        format!("{} {status}", cards.join(" "))
    }
}

/// splitmix64: a small generator, the same on every machine, so a seed always gives the same inputs.
struct Rng(u64);

impl Rng {
    fn new(seed: u64, name: &str, run: u64) -> Rng {
        // FNV-1a of the program's name, so each program gets inputs of its own
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name.bytes() {
            h = (h ^ b as u64).wrapping_mul(0x100000001b3);
        }
        Rng(h ^ seed.wrapping_mul(0x9e3779b97f4a7c15) ^ run.wrapping_mul(0xbf58476d1ce4e5b9))
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    /// A number from lo to hi, both included.
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next() % (hi - lo + 1) as u64) as i64
    }
}

fn die(m: String) -> ! {
    eprintln!("error: {m}");
    std::process::exit(2)
}

/// `LO..HI`, `NxLO..HI` or `N-MxLO..HI`.
fn parse_inputs(word: &str) -> Option<Inputs> {
    let (count, range) = match word.split_once('x') {
        Some((c, r)) => (c, r),
        None => ("1", word),
    };
    let (min, max) = match count.split_once('-') {
        Some((a, b)) => (a.parse().ok()?, b.parse().ok()?),
        None => (count.parse().ok()?, count.parse().ok()?),
    };
    let (lo, hi) = range.split_once("..")?;
    let (lo, hi): (i64, i64) = (lo.parse().ok()?, hi.parse().ok()?);
    (min <= max && lo <= hi && lo >= -999 && hi <= 999).then_some(Inputs { min, max, lo, hi })
}

fn targets(root: &Path) -> Vec<Target> {
    let file = root.join("tests/fuzz/targets");
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| die(format!("{}: {e}", file.display())));
    let mut found = vec![];
    for (n, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        let bad = || -> ! { die(format!("{}:{}: expected NAME SOURCE INPUTS...", file.display(), n + 1)) };
        if words.len() < 2 {
            bad()
        }
        let exact = words.last() == Some(&"exact");
        let ins = &words[2..words.len() - exact as usize];
        let inputs = ins.iter().map(|w| parse_inputs(w).unwrap_or_else(|| bad())).collect();
        found.push(Target {
            name: words[0].to_string(),
            source: root.join(words[1]),
            inputs,
            exact,
        });
    }
    found
}

/// One run's inputs.
fn generate(t: &Target, seed: u64, run: u64) -> Vec<Value> {
    let mut rng = Rng::new(seed, &t.name, run);
    let mut values = vec![];
    for i in &t.inputs {
        let n = rng.range(i.min as i64, i.max as i64);
        for _ in 0..n {
            let edges: Vec<i64> = [i.lo, i.hi, 0, 1, -1, i.lo + 1, i.hi - 1]
                .into_iter()
                .filter(|v| (i.lo..=i.hi).contains(v))
                .collect();
            let v = if rng.next() % 4 == 0 {
                edges[(rng.next() % edges.len() as u64) as usize]
            } else {
                rng.range(i.lo, i.hi)
            };
            values.push(Value { v, lo: i.lo, hi: i.hi });
        }
    }
    values
}

fn words(values: &[Value]) -> Vec<String> {
    values.iter().map(|x| x.v.to_string()).collect()
}

/// Run a command with this standard input, stopping it after TIMEOUT.
fn run(cmd: Command, stdin: &str) -> Run {
    run_with(cmd, stdin, TIMEOUT)
}

/// Run a command with this standard input, stopping it after `timeout`.
fn run_with(mut cmd: Command, stdin: &str, timeout: Duration) -> Run {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| die(format!("{cmd:?}: {e}")));
    let mut input = child.stdin.take().unwrap();
    let stdin = stdin.to_string();
    let writer = std::thread::spawn(move || {
        let _ = input.write_all(stdin.as_bytes());
    });
    let read = |mut r: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut s = String::new();
            let _ = r.read_to_string(&mut s);
            s
        })
    };
    let out = read(Box::new(child.stdout.take().unwrap()));
    let err = read(Box::new(child.stderr.take().unwrap()));
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s.code().unwrap_or(-2);
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            break -1;
        }
        std::thread::sleep(Duration::from_micros(200));
    };
    let _ = writer.join();
    let cards = out.join().unwrap();
    let why = err.join().unwrap();
    Run { cards, status, why }
}

/// Where everything is.
/// burn, heartburn's runtime, which runs the decks: $BURN, or burn on the PATH.
fn burn() -> PathBuf {
    std::env::var_os("BURN").map_or_else(|| PathBuf::from("burn"), PathBuf::from)
}

struct Work {
    tool: PathBuf,
    pkg: PathBuf,
    dir: PathBuf,
}

impl Work {
    fn deck(&self, setting: &str, name: &str) -> PathBuf {
        self.dir.join(setting).join(format!("{name}.cqb"))
    }

    /// Keep what build wrote for NAME, for one setting: its decks (NAME.cqb, and NAME.2.cqb, ... for a program in
    /// phases), their slots (NAME.cqb.meta) and its panic map (NAME.map), in place of an earlier build's.
    fn keep(&self, setting: &str, name: &str) {
        let dest = self.dir.join(setting);
        let outputs = |dir: &Path| -> Vec<(PathBuf, std::ffi::OsString)> {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return vec![];
            };
            entries
                .flatten()
                .filter(|e| {
                    let file = e.file_name();
                    let Some(rest) = file.to_str().and_then(|f| f.strip_prefix(name)).and_then(|f| f.strip_prefix('.'))
                    else {
                        return false;
                    };
                    let phase = rest
                        .strip_suffix(".cqb")
                        .is_some_and(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()));
                    matches!(rest, "cqb" | "cqb.meta" | "map") || phase
                })
                .map(|e| (e.path(), e.file_name()))
                .collect()
        };
        for (old, _) in outputs(&dest) {
            let _ = std::fs::remove_file(old);
        }
        for (from, file) in outputs(&self.pkg) {
            std::fs::copy(from, dest.join(file)).unwrap();
        }
    }

    /// The program's native binary: with its `.exe` on Windows, where checking it exists or removing it needs the
    /// whole name (running it doesn't).
    fn native(&self, setting: &str, name: &str) -> PathBuf {
        self.pkg
            .join("target")
            .join(setting)
            .join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
    }

    fn run_native(&self, setting: &str, name: &str, values: &[Value]) -> Run {
        let mut cmd = Command::new(self.native(setting, name));
        // load_resource reads resources/ beside the package, which cargo names in this
        cmd.env("CARGO_MANIFEST_DIR", &self.pkg);
        run(cmd, &(words(values).join(" ") + "\n"))
    }

    /// The deck on burn, which says why it panicked from the panic map beside it, as the native run's panic does.
    fn run_cardiac(&self, setting: &str, name: &str, values: &[Value]) -> Run {
        let mut cmd = Command::new(burn());
        cmd.arg(self.deck(setting, name)).arg("--").args(words(values));
        let mut r = run(cmd, "");
        r.why = r.why.lines().last().unwrap_or("").to_string();
        r
    }
}

/// The package the programs are built in: each one's source copied in, the examples' resources, and a profile for
/// each setting (no debuginfo, debug assertions or incremental builds, which change the MIR).
fn write_package(root: &Path, work: &Work, targets: &[Target]) {
    let _ = std::fs::remove_dir_all(work.pkg.join("resources"));
    std::fs::create_dir_all(work.pkg.join("resources")).unwrap();
    for e in std::fs::read_dir(root.join("examples/resources")).unwrap() {
        let e = e.unwrap();
        std::fs::copy(e.path(), work.pkg.join("resources").join(e.file_name())).unwrap();
    }
    for t in targets {
        let to = work.pkg.join(format!("{}.rs", t.name));
        let text = std::fs::read_to_string(&t.source)
            .unwrap_or_else(|e| die(format!("{}: {e}", t.source.display())));
        // unchanged files keep their times, so cargo doesn't build them again
        if std::fs::read_to_string(&to).ok().as_deref() != Some(text.as_str()) {
            std::fs::write(&to, text).unwrap();
        }
    }
    let names: Vec<String> = targets.iter().map(|t| t.name.clone()).collect();
    let toml = manifest(root, &names);
    let path = work.pkg.join("Cargo.toml");
    if std::fs::read_to_string(&path).ok().as_deref() != Some(toml.as_str()) {
        std::fs::write(&path, toml).unwrap();
    }
}

/// The fuzzer's Cargo.toml: these binaries, each NAME.rs beside it, and a profile for each setting.
fn manifest(root: &Path, names: &[String]) -> String {
    let mut toml = format!(
        "# Written by tests/fuzz/fuzz.rs; each run of the fuzzer writes it again.\n[package]\nname = \"fuzz\"\n\
         version = \"0.1.0\"\nedition = \"2021\"\npublish = false\nautobins = false\n\n[dependencies]\n\
         cardiac = {{ path = {:?} }}\n\n[workspace]\n\n",
        root.join("cardiac").display().to_string()
    );
    for name in names {
        toml += &format!("[[bin]]\nname = \"{name}\"\npath = \"{name}.rs\"\n\n");
    }
    for (name, opt, checks) in SETTINGS {
        toml += &format!(
            "[profile.{name}]\ninherits = \"dev\"\nopt-level = {opt}\noverflow-checks = {checks}\n\
             debug = false\ndebug-assertions = false\nincremental = false\n\n"
        );
    }
    toml
}

/// Build every program in every setting, natively and for CARDIAC. Returns, for each setting, the programs that
/// built (a program can be too big for CARDIAC in one setting and not in another), and says why each other didn't.
fn build(work: &Work, targets: &[Target]) -> HashMap<&'static str, Vec<String>> {
    let manifest = work.pkg.join("Cargo.toml");
    let mut built = HashMap::new();
    for (setting, _, _) in SETTINGS {
        let status = Command::new("cargo")
            .args(["build", "-q", "--bins", "--profile", setting, "--manifest-path"])
            .arg(&manifest)
            .status()
            .unwrap();
        if !status.success() {
            die(format!("the native build failed ({setting})"));
        }
        std::fs::create_dir_all(work.dir.join(setting)).unwrap();
        let mut ok = vec![];
        for t in targets {
            let out = Command::new(&work.tool)
                .args(["build", "--profile", setting, "--bin", &t.name, "--manifest-path"])
                .arg(&manifest)
                .output()
                .unwrap();
            if !out.status.success() {
                let err = String::from_utf8_lossy(&out.stderr);
                let why = err.lines().find(|l| l.starts_with("error")).unwrap_or("failed").to_string();
                println!("NOT BUILT {} {setting}: {why}", t.name);
                continue;
            }
            work.keep(setting, &t.name);
            ok.push(t.name.clone());
        }
        built.insert(setting, ok);
    }
    built
}

/// A run that differs, and whether it passed what a card holds: native out() was given a value no card holds, or
/// CARDIAC built with overflow checks (at this opt-level if it built, else at any) says a value didn't fit. For a
/// program marked `exact`, only a build with overflow checks can pass a card: one without must match.
fn differs(
    work: &Work,
    built: &HashMap<&'static str, Vec<String>>,
    setting: &str,
    t: &Target,
    values: &[Value],
) -> Option<(Run, Run, bool)> {
    let name = t.name.as_str();
    let native = work.run_native(setting, name, values);
    let cardiac = work.run_cardiac(setting, name, values);
    if (native.cards.as_str(), native.status) == (cardiac.cards.as_str(), cardiac.status) {
        return None;
    }
    let opt = &setting[..4];
    let mut checked: Vec<&str> = SETTINGS
        .iter()
        .filter(|(s, _, c)| *c && built[s].iter().any(|n| n == name))
        .map(|(s, _, _)| *s)
        .collect();
    checked.sort_by_key(|s| !s.starts_with(opt));
    if t.exact && !SETTINGS.iter().any(|(s, _, c)| *s == setting && *c) {
        checked.clear();
    }
    let past = native.why.contains(NOT_A_CARD)
        || match checked.first() {
        Some(&s) if s == setting => past_a_card(&cardiac.why),
        Some(&s) => past_a_card(&work.run_cardiac(s, name, values).why),
            None => false,
        };
    Some((native, cardiac, past))
}

/// The smallest input, as far as a greedy search finds, that still differs other than by passing a card: one
/// value fewer, or a value nearer 0, kept in its range.
fn shrink(
    work: &Work,
    built: &HashMap<&'static str, Vec<String>>,
    setting: &str,
    t: &Target,
    mut values: Vec<Value>,
) -> Vec<Value> {
    let fails = |vs: &[Value]| matches!(differs(work, built, setting, t, vs), Some((_, _, false)));
    let mut tries = 0;
    'better: loop {
        let mut candidates: Vec<Vec<Value>> = vec![];
        for i in 0..values.len() {
            let mut fewer = values.clone();
            fewer.remove(i);
            candidates.push(fewer);
        }
        for i in 0..values.len() {
            let x = values[i];
            for v in [0, x.v / 2, x.v - x.v.signum()] {
                let v = v.clamp(x.lo, x.hi);
                if v.abs() < x.v.abs() {
                    let mut nearer = values.clone();
                    nearer[i].v = v;
                    candidates.push(nearer);
                }
            }
        }
        for c in candidates {
            tries += 1;
            if tries > 400 {
                break 'better;
            }
            if fails(&c) {
                values = c;
                continue 'better;
            }
        }
        break;
    }
    values
}

/// How one program went in one setting.
#[derive(Default)]
struct Tally {
    same: usize,
    past: usize,
    failed: usize,
}

fn main() {
    let mut runs: Option<u64> = None;
    let mut seed: u64 = 1;
    let mut only: Vec<String> = vec![];
    let mut count: Option<u64> = None;
    let mut start: u64 = 0;
    let mut batch: usize = 60;
    let mut shrinking = true;
    let mut enums: u64 = 30;
    let mut repro: Option<PathBuf> = None;
    let mut input: Vec<String> = vec![];
    let mut tool: Option<PathBuf> = None;
    let mut a = std::env::args().skip(1);
    let usage = "usage: fuzz ROOT [--runs N] [--seed S] [--only NAME]... | --generate N [--start K] [--batch B] \
                 [--no-shrink] [--enums PCT] | --repro FILE.rs [-- INPUT...]; and --tool CARGO-CARDIAC";
    let root = PathBuf::from(a.next().unwrap_or_else(|| die(usage.into())));
    while let Some(arg) = a.next() {
        let mut value = || a.next().unwrap_or_else(|| die(format!("{arg} needs a value")));
        let number = |v: String| v.parse::<u64>().unwrap_or_else(|_| die(format!("{arg} takes a number")));
        match arg.as_str() {
            "--runs" => runs = Some(number(value())),
            "--seed" => seed = number(value()),
            "--only" => only.push(value()),
            "--generate" => count = Some(number(value())),
            "--start" => start = number(value()),
            "--batch" => batch = number(value()).max(1) as usize,
            "--no-shrink" => shrinking = false,
            "--enums" => enums = number(value()).min(100),
            "--repro" => repro = Some(PathBuf::from(value())),
            "--tool" => tool = Some(PathBuf::from(value())),
            "--" => input = a.by_ref().collect(),
            _ => die(format!("unknown argument {arg}\n{usage}")),
        }
    }
    // absolute, not canonical: on Windows a canonical path is a verbatim \\?\ one, which MinGW ld cannot open
    let root = std::path::absolute(&root).unwrap();
    let tool = tool.unwrap_or_else(|| root.join("target/release/cargo-cardiac"));
    if let Some(file) = repro {
        generate::repro(&root, &tool, &file, &input);
        return;
    }
    if let Some(count) = count {
        let runs = runs.unwrap_or(40);
        generate::campaign(&root, &tool, generate::Options { count, start, seed, runs, batch, shrink: shrinking, enums });
        return;
    }
    let runs = runs.unwrap_or(300);
    let mut targets = targets(&root);
    if !only.is_empty() {
        targets.retain(|t| only.contains(&t.name));
        if targets.is_empty() {
            die(format!("no program named {}", only.join(", ")));
        }
    }
    let dir = root.join("target/fuzz");
    let work = Work {
        tool,
        pkg: dir.join("pkg"),
        dir,
    };
    write_package(&root, &work, &targets);
    let built = build(&work, &targets);
    println!("-- {} programs, {runs} runs each in {} settings, seed {seed}", targets.len(), SETTINGS.len());
    // one thread a program; each run's inputs go to every setting
    let results: Vec<(String, Vec<Tally>, Vec<String>)> = std::thread::scope(|s| {
        let handles: Vec<_> = targets
            .iter()
            .map(|t| {
                let (work, built) = (&work, &built);
                s.spawn(move || {
                    let mut tallies: Vec<Tally> = SETTINGS.iter().map(|_| Tally::default()).collect();
                    let mut reports = vec![];
                    for r in 0..runs {
                        let values = generate(t, seed, r);
                        for (k, (setting, _, _)) in SETTINGS.iter().enumerate() {
                            if !built[setting].contains(&t.name) {
                                continue;
                            }
                            match differs(work, built, setting, t, &values) {
                                None => tallies[k].same += 1,
                                Some((_, _, true)) => tallies[k].past += 1,
                                Some(_) => {
                                    tallies[k].failed += 1;
                                    // the first failure in each setting is shrunk and reported
                                    if tallies[k].failed > 1 {
                                        continue;
                                    }
                                    let small = shrink(work, built, setting, t, values.clone());
                                    let (native, cardiac, _) =
                                        differs(work, built, setting, t, &small).unwrap();
                                    let input = words(&small).join(" ");
                                    let mut m = format!(
                                        "DIFFER {} {setting}, seed {seed} run {r}: input {input} (from {})\n",
                                        t.name,
                                        words(&values).join(" ")
                                    );
                                    m += &format!("  native:  {}\n", native.show());
                                    m += &format!("  cardiac: {}", cardiac.show());
                                    if cardiac.status == 101 {
                                        m += &format!(" ({})", cardiac.why);
                                    }
                                    m += &format!(
                                        "\n  again:   echo \"{input}\" | {}\n           {} run {} -- {input}\n",
                                        work.native(setting, &t.name).display(),
                                        work.tool.display(),
                                        work.deck(setting, &t.name).display()
                                    );
                                    reports.push(m);
                                }
                            }
                        }
                    }
                    (t.name.clone(), tallies, reports)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut failed = false;
    for (name, tallies, _) in &results {
        let cols: Vec<String> = SETTINGS
            .iter()
            .zip(tallies)
            .map(|((setting, _, _), t)| {
                if !built[setting].contains(name) {
                    "-".to_string()
                } else if t.failed > 0 {
                    format!("FAIL {}", t.failed)
                } else if t.past > 0 {
                    format!("{} +{} past", t.same, t.past)
                } else {
                    t.same.to_string()
                }
            })
            .collect();
        let line: String = cols.iter().map(|c| format!("{c:>16}")).collect();
        println!("{name:<13}{line}");
        failed |= tallies.iter().any(|t| t.failed > 0);
    }
    let header: String = SETTINGS.iter().map(|(s, _, _)| format!("{s:>16}")).collect();
    println!("{:<13}{header}", "");
    for (_, _, reports) in &results {
        for r in reports {
            print!("{r}");
        }
    }
    if failed {
        std::process::exit(1);
    }
}
