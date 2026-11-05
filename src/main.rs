//! `cargo cardiac`: the cargo subcommand. Cargo runs it as `cargo-cardiac cardiac ARGS`; run by hand it is
//! `cargo-cardiac ARGS`. `usage()` says what it does.
//!
//! The target is cardiac-unknown-none, the notional triple for CARDIAC. It isn't one rustc knows, so there is no
//! `--target` to give: this subcommand is what builds for it.
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI8, Ordering};

use cargo_cardiac::cargo::{self, packages, Package, Target};
use cargo_cardiac::compile::program::{self, Built, Emit, Rustc};
use cargo_cardiac::driver::{self, Dep, Mode, Outcome, Program, Settings};
use cargo_cardiac::{TARGET, TOOL};
use serde_json::{json, Value};

fn usage() -> String {
    let t = format!("cargo {TOOL}");
    format!(
        "\
{t}: Rust for CARDIAC

Usage:
  {t} build [--manifest-path PATH] [-p NAME] [--release | --profile NAME] [--bin NAME]...
      Compile the package's programs (its binary targets, or the ones named) for CARDIAC: the package here,
      or the workspace member -p names. Each gets its deck, NAME.cqb (and NAME.2.cqb, ... for a program in
      phases), and its panic map, NAME.map, next to its source. Run the deck with burn, from cardiaq; with the
      map beside it, burn says why a program panicked, as Rust does.
  {t} build --emit LIST ...
      What build writes, as rustc's --emit: any of deck, listing, map and asm (deck,map when not given).
      listing is the program in Brian Stuart's six columns, NAME.cq (NAME.2.cq, ...), which carc assembles.
  {t} build [-C opt-level=N] [-C overflow-checks=on|off] [-C debug-assertions=on|off] [--auto-overlay] FILE.rs...
      The same for single files that use no other crate, not even cardiac: such a file defines the
      built-ins it calls itself, as stubs (`#[inline(never)] pub fn out(_v: i16) {{}}`).

Resources, load_resource(\"NAME\"):
  A resource is the file resources/NAME beside the package's Cargo.toml (beside a single file, for one):
  numbers, as the input is written. A program loads it once, in main and outside any loop. build fills
  each resource's slot in the deck from it when it is there; burn fills a slot still empty from resources/
  beside the deck.

Settings, as cargo and rustc take them:
  A package is built with a Cargo profile: dev, release with --release, or the one --profile names, as
  [profile.NAME] in the workspace's Cargo.toml sets it. A single file has no Cargo.toml, so it takes
  rustc's flags, with rustc's defaults: opt-level 0, and overflow checks on at opt-level 0, off above.
  The opt-level is rustc's, and it also sets how hard cargo-cardiac's own passes work: none at 0, all of
  them at 1 and up. At 0 few programs fit in CARDIAC's memory.

Phases:
  A program too big for memory comes in phases, each loaded over the one before; the data stays. By default
  the program says where each phase ends, with overlay() in main. With auto-overlay on, the compiler does:
  when main doesn't fit, it is cut where it runs once, at the place that leaves the smallest program, and
  overlay() is an error. A package turns it on in its Cargo.toml, a single file with --auto-overlay:
      [package.metadata.cardiac]
      auto-overlay = true

How much it says:
  -q, --quiet       errors only
  -v, --verbose     also where the cells go
  --message-format=json
                    the same on standard error, each message a JSON object on a line of its own (JSON Lines)
                    with its kind in \"reason\", as cargo's: compiling, compiled, note, error.
                    --message-format=human, the default, is lines for a person

For developing cargo-cardiac:
  {t} test [-C opt-level=N] [-C overflow-checks=on|off] [-C debug-assertions=on|off] FILE.rs
          [-- INPUT...]
  {t} test [--manifest-path PATH] [-p NAME] [--release | --profile NAME] --bin NAME [-- INPUT...]
      Compile a program as the compiler's tests do, write its .mir and .asm beside it, run it from
      main_bb0 on the input, and print its cards on one line.
"
    )
}

fn bad_usage(why: &str) -> ! {
    tell(Message::Error { why, usage: true });
    std::process::exit(2)
}

/// How much the tool says besides its errors: -1 with -q, 0 by default, 1 with -v. An error is said at every
/// level.
static VERBOSITY: AtomicI8 = AtomicI8::new(0);

fn verbosity() -> i8 {
    VERBOSITY.load(Ordering::Relaxed)
}

/// Whether the tool says what it says as JSON Lines, with --message-format=json, rather than as lines for a
/// person.
static JSON: AtomicBool = AtomicBool::new(false);

fn json() -> bool {
    JSON.load(Ordering::Relaxed)
}

/// Something the tool says, in either format: a line (or two) for a person, or one JSON object whose `reason` names
/// its kind, as cargo's messages do. Each is said at a level of verbosity and above (`level`), the same in both;
/// what -v adds to one (where the cells go) is a line more in text and fields more in JSON.
enum Message<'a> {
    Compiling {
        package: &'a str,
        programs: usize,
        profile: &'a str,
    },
    Compiled(&'a Built),
    /// what a build suggests the programmer change (`Built::advice`)
    Advice {
        file: &'a str,
        text: &'a str,
    },
    /// the dev settings `cargo cardiac` chose, the programmer's Cargo.toml not setting them (`cargo::dev_defaults`)
    Defaults {
        file: &'a str,
        text: &'a str,
    },
    /// a resource a build left empty, having no file
    Note {
        deck: &'a str,
        resource: &'a str,
        file: &'a Path,
    },
    /// an error; with `usage`, one in the command line, said with the usage after it
    Error {
        why: &'a str,
        usage: bool,
    },
}

impl Message<'_> {
    /// The least verbosity the message is said at: -1 (always) for an error or a note.
    fn level(&self) -> i8 {
        match self {
            Message::Note { .. } | Message::Advice { .. } | Message::Error { .. } => -1,
            _ => 0,
        }
    }

    fn text(&self) -> String {
        let v = verbosity() >= 1;
        match self {
            Message::Compiling {
                package,
                programs: n,
                profile,
            } => format!(
                "   Compiling {package} ({n} program{}, {TARGET}, {profile} profile)",
                if *n == 1 { "" } else { "s" }
            ),
            Message::Compiled(b) if v => format!("{}\n           {}", b.line(), b.sections),
            Message::Compiled(b) => b.line(),
            Message::Note { deck, resource, file } => format!(
                "note: {deck}: resource {resource} has no file {}; its slot in the deck is empty until it is bound",
                file.display()
            ),
            Message::Advice { file, text } | Message::Defaults { file, text } => format!("note: {file}: {text}"),
            Message::Error { why, usage: false } => format!("error: {why}"),
            Message::Error { why, usage: true } => format!("error: {why}\n\n{}", usage()),
        }
    }

    fn json(&self) -> Value {
        let v = verbosity() >= 1;
        match self {
            Message::Compiling {
                package,
                programs,
                profile,
            } => {
                json!({"reason": "compiling", "package": package, "programs": programs, "target": TARGET,
                    "profile": profile})
            }
            Message::Compiled(b) => {
                let mut m = json!({"reason": "compiled", "file": b.file, "deck": b.deck_file, "phases": b.code.len(),
                    "cells": b.cells(), "data": b.data, "code": b.code});
                if v {
                    let functions: Vec<Value> = b
                        .sections
                        .code
                        .iter()
                        .map(|(name, n)| json!({"name": name, "cells": n}))
                        .collect();
                    m["functions"] = functions.into();
                    m["variables"] = b.sections.variables.into();
                    m["constants"] = b.sections.constants.into();
                }
                m
            }
            Message::Note { deck, resource, file } => {
                json!({"reason": "note", "message": self.text().strip_prefix("note: "), "deck": deck,
                    "resource": resource, "file": file.display().to_string()})
            }
            Message::Advice { file, text } | Message::Defaults { file, text } => {
                json!({"reason": "note", "message": text, "file": file})
            }
            Message::Error { why, .. } => json!({"reason": "error", "message": why}),
        }
    }
}

/// Say a message on standard error, in the format asked for, if the verbosity is at its level.
fn tell(m: Message) {
    if verbosity() >= m.level() {
        eprintln!("{}", if json() { m.json().to_string() } else { m.text() })
    }
}

/// Say what a command made (build's sizes) on standard output, where it is the command's output; in
/// JSON on standard error with the rest, so a tool reads every message from one place.
fn say(m: Message) {
    if json() {
        tell(m)
    } else if verbosity() >= m.level() {
        println!("{}", m.text())
    }
}

fn die(e: String) -> ! {
    tell(Message::Error { why: &e, usage: false });
    std::process::exit(1)
}

/// A source path as short as it can be shown: relative to here when it is under here.
fn shown(path: &str) -> String {
    let here = std::env::current_dir().ok();
    here.and_then(|h| {
        std::path::Path::new(path)
            .strip_prefix(h)
            .ok()
            .map(|p| p.display().to_string())
    })
    .unwrap_or_else(|| path.to_string())
}

/// Numbers, each a card: -999..999.
fn cards<'a>(words: impl Iterator<Item = &'a str>) -> Vec<i32> {
    words.map(card).collect()
}

/// A number, as a card.
fn card(v: &str) -> i32 {
    match v.parse::<i32>() {
        Ok(c) if (-999..=999).contains(&c) => c,
        _ => die(format!("not a card: {v} (a card holds a number from -999 to 999)")),
    }
}

/// The parsed command line.
struct Args {
    command: String,
    files: Vec<String>,
    manifest: Option<String>,
    /// The workspace member named with -p.
    package: Option<String>,
    bins: Vec<String>,
    /// The Cargo profile a package is built with (`--release` is `release`); dev when none is named.
    profile: Option<String>,
    /// rustc's flags for a single file (`-C opt-level=2`), which has no profile.
    rustc: Rustc,
    /// --auto-overlay, for a single file: the compiler splits it into phases itself.
    auto_overlay: bool,
    /// --emit, for build: what it writes beside each program
    emit: Emit,
    input: Option<Vec<String>>,
}

impl Args {
    /// The profile and the flags are for different things: say so when they are given to the wrong one.
    fn settings_fit(&self) {
        let single = !self.files.is_empty();
        if single && self.profile.is_some() {
            bad_usage("--release and --profile choose a Cargo profile, and a single file has none: give it rustc's flags, -C opt-level=N, -C overflow-checks=on|off and -C debug-assertions=on|off")
        }
        if !single && !self.rustc.flags.is_empty() {
            bad_usage("-C sets rustc's flags for a single file; a package's settings come from its Cargo profile ([profile.NAME] in Cargo.toml), chosen with --release or --profile NAME")
        }
        if !single && self.auto_overlay {
            bad_usage("--auto-overlay is for a single file; a package turns it on in its Cargo.toml, with auto-overlay = true under [package.metadata.cardiac]")
        }
    }
}

fn args() -> Args {
    let mut a: Vec<String> = std::env::args().skip(1).collect();
    if a.first().map(String::as_str) == Some(TOOL) {
        a.remove(0);
    }
    // the format first, so that an error in the rest of the line is said in it too
    for (i, f) in a.iter().enumerate().take_while(|(_, f)| *f != "--") {
        let format = match f.strip_prefix("--message-format") {
            Some("") => a.get(i + 1).map(String::as_str).unwrap_or(""),
            Some(joined) => joined.strip_prefix('=').unwrap_or(joined),
            None => continue,
        };
        match format {
            "json" => JSON.store(true, Ordering::Relaxed),
            "human" => JSON.store(false, Ordering::Relaxed),
            _ => bad_usage("--message-format is human (the default) or json"),
        }
    }
    let Some(command) = a.first().cloned() else {
        eprint!("{}", usage());
        std::process::exit(2)
    };
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        print!("{}", usage());
        std::process::exit(0)
    }
    let mut r = Args {
        command,
        files: vec![],
        manifest: None,
        package: None,
        bins: vec![],
        profile: None,
        rustc: Rustc::default(),
        auto_overlay: false,
        emit: Emit::default(),
        input: None,
    };
    let compiles = matches!(r.command.as_str(), "build" | "test");
    fn profile(r: &mut Args, name: &str) {
        if r.profile.as_deref().is_some_and(|p| p != name) {
            bad_usage("give one profile: --release is --profile release")
        }
        r.profile = Some(name.to_string());
    }
    let mut i = 1;
    while i < a.len() {
        match a[i].as_str() {
            "--manifest-path" => {
                i += 1;
                r.manifest = Some(
                    a.get(i)
                        .cloned()
                        .unwrap_or_else(|| bad_usage("--manifest-path needs a path")),
                );
            }
            "-p" | "--package" => {
                i += 1;
                r.package = Some(
                    a.get(i)
                        .cloned()
                        .unwrap_or_else(|| bad_usage("-p needs a package name")),
                );
            }
            "--bin" => {
                i += 1;
                r.bins
                    .push(a.get(i).cloned().unwrap_or_else(|| bad_usage("--bin needs a name")));
            }
            "--release" if compiles => profile(&mut r, "release"),
            "--auto-overlay" if compiles => r.auto_overlay = true,
            "--emit" if r.command == "build" => {
                i += 1;
                let list = a
                    .get(i)
                    .map(String::as_str)
                    .unwrap_or_else(|| bad_usage("--emit needs a list: deck,listing,map,asm"));
                r.emit = Emit::parse(list).unwrap_or_else(|e| bad_usage(&e));
            }
            s if r.command == "build" && s.starts_with("--emit=") => {
                r.emit = Emit::parse(&s["--emit=".len()..]).unwrap_or_else(|e| bad_usage(&e));
            }
            "--profile" if compiles => {
                i += 1;
                let name = a.get(i).cloned().unwrap_or_else(|| bad_usage("--profile needs a name"));
                profile(&mut r, &name)
            }
            // rustc's own spelling: -C NAME=VALUE, or -CNAME=VALUE
            c if compiles && c.starts_with("-C") => {
                let option = match &c[2..] {
                    "" => {
                        i += 1;
                        a.get(i)
                            .cloned()
                            .unwrap_or_else(|| bad_usage("-C needs an option, opt-level=N say"))
                    }
                    joined => joined.to_string(),
                };
                r.rustc.set(&option).unwrap_or_else(|e| bad_usage(&e))
            }
            // as cargo's: -q or -v, not both
            q @ ("-q" | "--quiet" | "-v" | "--verbose") => {
                let quiet = q == "-q" || q == "--quiet";
                if (quiet && verbosity() > 0) || (!quiet && verbosity() < 0) {
                    bad_usage("give -q (quiet) or -v (verbose), not both")
                }
                let level = if quiet { -1 } else { 1 };
                VERBOSITY.store(level, Ordering::Relaxed)
            }
            // read before the rest (above)
            "--message-format" => i += 1,
            f if f.starts_with("--message-format=") => {}
            "--help" | "-h" => {
                print!("{}", usage());
                std::process::exit(0)
            }
            "--" => {
                r.input = Some(a[i + 1..].to_vec());
                break;
            }
            f if !f.starts_with('-') || f.parse::<i32>().is_ok() => r.files.push(f.to_string()),
            f => bad_usage(&format!("unknown option {f}")),
        }
        i += 1;
    }
    r
}

/// The package's programs that were asked for: those named with --bin, or all of them. A package whose binaries
/// can't be CARDIAC programs (this tool's own, or one that needs a proc-macro crate) stops with where to go instead.
fn chosen<'a>(pkgs: &'a [Package], bins: &[String]) -> Vec<(&'a Package, &'a Target)> {
    for p in pkgs {
        let why = if p.name == env!("CARGO_PKG_NAME") {
            Some(format!("this is {}'s own package", p.name))
        } else {
            p.problem.clone()
        };
        if let Some(why) = why {
            die(format!("{}'s binaries can't be compiled for CARDIAC: {why}.\nRun `cargo {TOOL}` in a package whose binaries are CARDIAC programs, such as examples/ in cargo-cardiac's repository, or name one with --manifest-path.", p.name));
        }
    }
    for b in bins {
        if !pkgs.iter().any(|p| p.programs.iter().any(|t| &t.name == b)) {
            die(format!("no binary target named {b}"));
        }
    }
    pkgs.iter()
        .flat_map(|p| {
            p.programs
                .iter()
                .filter(|t| bins.is_empty() || bins.contains(&t.name))
                .map(move |t| (p, t))
        })
        .collect()
}

/// Compile these programs of a package with this profile, as `mode` asks (the driver does it, in cargo's build):
/// what the driver made of each, in order, up to the first that doesn't compile, which stops `cargo cardiac` with
/// its error (as rustc's errors stop the build, said as rustc says them).
fn compile(p: &Package, programs: &[&Target], profile: Option<&str>, mode: Mode, emit: Emit) -> Vec<Outcome> {
    let settings = Settings {
        mode,
        programs: programs
            .iter()
            .map(|t| Program {
                source: PathBuf::from(&t.path),
                name: t.name.clone(),
                file: shown(&t.path),
                resources: resources_beside(&p.manifest),
                auto_overlay: p.auto_overlay,
                emit,
            })
            .collect(),
        crates: p
            .deps
            .iter()
            .map(|x| Dep {
                name: x.name.clone(),
                krate: x.krate.clone(),
            })
            .collect(),
        single: false,
    };
    // the dev settings CARDIAC needs that the programmer hasn't set, said each time they're used
    let defaults = cargo::dev_defaults(p, profile);
    if !defaults.is_empty() {
        let list: Vec<String> = defaults.iter().map(|(k, v)| format!("{k} = {v}")).collect();
        tell(Message::Defaults {
            file: &shown(&p.workspace_manifest.to_string_lossy()),
            text: &format!(
                "building with {} in the dev profile, which [profile.dev] doesn't set: CARDIAC needs rustc's optimized \
                 MIR, the CARDIAC driver can't build incrementally, debuginfo changes the listing, and the checks \
                 rarely fit in 96 cells (so an overflow `cargo run` panics on goes unchecked here). Set any of them \
                 in [profile.dev] to choose your own",
                list.join(", ")
            ),
        });
    }
    let (reports, ok) = cargo::build(p, programs, profile, &settings).unwrap_or_else(|e| die(e));
    let mut made = vec![];
    for (t, r) in programs.iter().zip(reports) {
        match r.map(|r| r.outcome) {
            Some(Outcome::Failed(e)) => {
                // what came before is said, as each program was compiled in turn
                say_made(&made, p);
                die(e)
            }
            Some(o) => made.push(o),
            None if ok => die(format!("the CARDIAC driver said nothing of {}", t.name)),
            None => {}
        }
    }
    if !ok {
        say_made(&made, p);
        die(format!("cargo check failed on {}", p.name));
    }
    made
}

/// What a build made before it stopped, said as `build` says it.
fn say_made(made: &[Outcome], p: &Package) {
    for o in made {
        if let Outcome::Built(b) = o {
            say(Message::Compiled(b));
            note_empty(b, &resources_beside(&p.manifest));
        }
    }
}

/// Build the programs asked for, each package checked by cargo once, and report each one's size.
fn build(pkgs: &[Package], bins: &[String], profile: Option<&str>, emit: Emit) {
    for p in pkgs {
        let mine: Vec<&Target> = chosen(std::slice::from_ref(p), bins)
            .into_iter()
            .map(|(_, t)| t)
            .collect();
        if mine.is_empty() {
            continue;
        }
        tell(Message::Compiling {
            package: &p.name,
            programs: mine.len(),
            profile: profile.unwrap_or("dev"),
        });
        for o in compile(p, &mine, profile, Mode::Build, emit) {
            let Outcome::Built(b) = o else { continue };
            say(Message::Compiled(&b));
            note_empty(&b, &resources_beside(&p.manifest));
        }
    }
}

/// Compile a single file with these flags, as `mode` asks: what the driver made of it.
fn compile_single(f: &str, rustc: &Rustc, auto_overlay: bool, mode: Mode, emit: Emit) -> Outcome {
    let name = Path::new(f).file_stem().and_then(|s| s.to_str()).unwrap_or("program");
    let source = std::fs::canonicalize(f).unwrap_or_else(|e| die(format!("{f}: {e}")));
    let settings = Settings {
        mode,
        programs: vec![Program {
            source,
            name: name.to_string(),
            file: f.to_string(),
            resources: resources_beside(f),
            auto_overlay,
            emit,
        }],
        crates: vec![],
        single: true,
    };
    match driver::single(f, rustc, &settings).unwrap_or_else(|e| die(e)) {
        Some(r) => match r.outcome {
            Outcome::Failed(e) => die(e),
            o => o,
        },
        None => die(format!("rustc failed on {f}")),
    }
}

/// A file a build wrote, read back.
fn read(file: &str) -> String {
    std::fs::read_to_string(file).unwrap_or_else(|e| die(format!("{file}: {e}")))
}

/// Run a test program's listing, which the driver wrote, as the compiler's tests do, and print its cards.
fn run_tested(file: &str, o: Outcome, input: &[i32]) {
    let Outcome::Tested { asm, opt_level } = o else {
        die(format!("the CARDIAC driver didn't test {file}"))
    };
    let punched = program::run_test(file, &read(&asm), input).map_err(|e| program::hint(e, opt_level));
    println!("{}", punched.unwrap_or_else(|e| die(e)));
}

/// The `resources/` directory beside a file: a package's Cargo.toml or a single source file.
fn resources_beside(file: &str) -> PathBuf {
    Path::new(file).parent().unwrap_or(Path::new("")).join("resources")
}

/// Say what a build suggests the programmer change, and which resources it left empty, having no file (burn binds
/// the deck's slot when it runs it).
fn note_empty(b: &Built, dir: &Path) {
    for text in &b.advice {
        tell(Message::Advice { file: &b.file, text });
    }
    for name in &b.empty {
        tell(Message::Note {
            deck: &b.deck_file,
            resource: name,
            file: &dir.join(name),
        });
    }
}

fn main() {
    let a = args();
    let single = !a.files.is_empty();
    let pkgs = || packages(a.manifest.as_deref(), a.package.as_deref()).unwrap_or_else(|e| die(e));
    let profile = a.profile.as_deref();
    if matches!(a.command.as_str(), "build" | "test") {
        a.settings_fit()
    }
    match a.command.as_str() {
        "build" if single => {
            for f in &a.files {
                let src = std::fs::read_to_string(f).unwrap_or_else(|e| die(format!("{f}: {e}")));
                if src.contains("cardiac::") {
                    die(format!("{f}: a single file can't use the cardiac crate (no Cargo package says where it is). Build it in a \
                        package with `cargo cardiac build`, or define the built-ins it calls in the file itself, such as \
                        `#[inline(never)] pub fn out(_v: i16) {{}}`"));
                }
                let Outcome::Built(b) = compile_single(f, &a.rustc, a.auto_overlay, Mode::Build, a.emit) else {
                    die(format!("the CARDIAC driver didn't build {f}"))
                };
                say(Message::Compiled(&b));
                note_empty(&b, &resources_beside(f));
            }
        }
        "build" => {
            build(&pkgs(), &a.bins, profile, a.emit);
        }
        "test" => {
            if a.files.len() > 1 {
                bad_usage("test takes one FILE.rs; its input goes after --")
            }
            // the program's own IR, printed as MIR, is written beside it before anything else (by the driver), so it
            // is there when linking fails
            let input = cards(a.input.iter().flatten().map(String::as_str));
            if single {
                let f = &a.files[0];
                run_tested(
                    f,
                    compile_single(f, &a.rustc, a.auto_overlay, Mode::Test, Emit::default()),
                    &input,
                );
            } else {
                if a.bins.len() != 1 {
                    bad_usage("test takes one FILE.rs or one --bin NAME")
                }
                let pkgs = pkgs();
                let (p, t) = chosen(&pkgs, &a.bins)[0];
                let o = compile(p, &[t], profile, Mode::Test, Emit::default()).pop();
                run_tested(
                    &shown(&t.path),
                    o.unwrap_or_else(|| die(format!("{} was not tested", t.name))),
                    &input,
                );
            }
        }
        // commands that are gone, and where to go instead
        "run" => die(format!(
            "cargo {TOOL} run is gone: build writes the deck, NAME.cqb, and burn runs it (burn NAME.cqb -- INPUT...), \
             saying why it panicked from NAME.map beside it"
        )),
        "asm" => die(format!(
            "cargo {TOOL} asm is gone: build writes a program's deck, and its listing in Brian Stuart's six columns \
             with --emit listing; carc assembles a listing edited by hand"
        )),
        "cardiasm" => die(format!("cargo {TOOL} cardiasm is gone")),
        c => bad_usage(&format!("unknown command {c}")),
    }
}
