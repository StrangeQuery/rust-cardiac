//! `cargo cardiac`: the cargo subcommand. Cargo runs it as `cargo-cardiac cardiac ARGS`; run by hand it is
//! `cargo-cardiac ARGS`. `usage()` says what it does.
//!
//! The target is cardiac-unknown-none, the notional triple for CARDIAC. It isn't one rustc knows, so there is no
//! `--target` to give: this subcommand is what builds for it.
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicI8, Ordering};
use std::sync::Mutex;

use cargo_cardiac::cargo::{self, packages, Package, Target};
use cargo_cardiac::codes::{self, Diag};
use cargo_cardiac::compile::program::{self, Built, Emit, Rustc, Sections};
use cargo_cardiac::driver::{self, Dep, Mode, Outcome, Program, Settings};
use cargo_cardiac::{CARDIAC, TARGET, TOOL};
use serde_json::{json, Value};

fn usage() -> String {
    let t = format!("cargo {TOOL}");
    let part = |name| command_usage(name).unwrap_or_default();
    let commands: String = COMMANDS.iter().filter(|c| **c != "test").map(|c| part(c)).collect();
    format!(
        "\
{t}: Rust for CARDIAC

Usage:
{commands}  {t} --version
      Which cargo-cardiac this is.

Messages:
  What build says about a program has a code, as rustc's errors do: error[RCE....] for an error,
  warning[RCW....] for a warning, note[RCI....] for a note. `{t} help CODE` says what it means.

Resources, load_resource(\"NAME\"):
  A resource is the file resources/NAME beside the package's Cargo.toml (beside a single file, for one):
  numbers, as the input is written. A program loads it once, in main and outside any loop. build fills
  each resource's slot in the deck from it when it is there; run fills a slot still empty from resources/
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

How much build and run say:
  -q, --quiet       errors only
  -v, --verbose     also where the cells go
  --message-format=json
                    the same on standard error, each message a JSON object on a line of its own (JSON Lines)
                    with its kind in \"reason\", as cargo's: compiling, compiled, note, warning, error,
                    and the message's code in \"code\".
                    --message-format=human, the default, is lines for a person

New versions:
  At most once a day, build, run and view look up cargo-cardiac's newest version in crates.io's public index,
  in the background, and say so after the build when there is a newer one. Nothing is sent but the request.
  CARGO_CARDIAC_NO_UPDATE_CHECK=1 turns it off; it is off with -q, --message-format=json and when standard
  error isn't a terminal.

For developing cargo-cardiac:
{test}",
        test = part("test"),
    )
}

/// The commands, in the usage's order: `help COMMAND` says one's part of it.
const COMMANDS: [&str; 9] = ["new", "setup", "build", "run", "view", "debug", "asm", "help", "test"];

/// A command's part of the usage: none for a word that isn't one.
fn command_usage(name: &str) -> Option<String> {
    let t = format!("cargo {TOOL}");
    let nightly = driver::TOOLCHAIN;
    let codes_page = codes::PAGE;
    Some(match name {
        "new" => format!(
            "  {t} new NAME [OPTION]...
      Make a CARDIAC program, as `cargo new NAME` makes a Rust one (its options are cargo new's), with cardiac
      {CARDIAC} as a dependency and src/main.rs a countdown from the number it reads.
"
        ),
        "setup" => format!(
            "  {t} setup
      Fetch what the first build needs, now rather than then: the CARDIAC driver for this platform (about
      1 MB) and the nightly it runs on, {nightly}, in rustup's minimal profile (about 550 MB on disk).
      When both are here already, it says so.
"
        ),
        "build" => format!(
            "  {t} build [--manifest-path PATH] [-p NAME] [--release | --profile NAME] [--bin NAME]...
      Compile the package's programs (its binary targets, or the ones named) for CARDIAC: the package here,
      or the workspace member -p names. Each gets its deck, NAME.cqb (and NAME.2.cqb, ... for a program in
      phases), and its panic map, NAME.map, next to its source.
  {t} build --emit LIST ...
      What build writes, as rustc's --emit: any of deck, listing, map and asm (deck,map when not given).
      listing is the program in Brian Stuart's six columns, NAME.cq (NAME.2.cq, ...), which asm assembles.
  {t} build [-C opt-level=N] [-C overflow-checks=on|off] [-C debug-assertions=on|off] [--auto-overlay]
            [--edition YEAR] FILE.rs...
      The same for single files that use no other crate, not even cardiac: such a file defines the
      built-ins it calls itself, as stubs (`#[inline(never)] pub fn out(_v: i16) {{}}`). It is Rust's
      2021 edition unless --edition names another, as rustc's does.
"
        ),
        "run" => format!(
            "  {t} run [build's options] [--bin NAME] [-- INPUT...]
      Build the program as build does, then run its deck, each phase's deck in turn, with its panic map
      beside it: a program that panics says why, and exits 101, as `cargo run` does. The input is the
      numbers after --, or standard input. A package with more than one program runs the one --bin names.
      Its build's messages go to standard error, so standard output is the program's alone.
"
        ),
        "view" => format!(
            "  {t} view [build's options] [--bin NAME] [-- INPUT...]
      Build the program as run does, and open it in Cardigan, the CARDIAC IDE, in your browser: stopped at the
      start, the input in its reader, to step through. The program goes in the address, which stays on this machine.
"
        ),
        "debug" => format!(
            "  {t} debug [OPTION]... FILE [INPUT]...
      Debug a listing (NAME.cq) or a deck, or profile it with --profile: ekg, with ekg's options.
      `{t} help debug` says them.
"
        ),
        "asm" => format!(
            "  {t} asm [OPTION]... FILE
      Assemble a listing into a deck: carc, with carc's options. `{t} help asm` says them.
"
        ),
        "help" => format!(
            "  {t} help [COMMAND | CODE]
      This, or a command's part of it, or what a code in a message means, as `rustc --explain` says it:
      error[RCE0002] is explained by `{t} help RCE0002`. Each code is on the web too, at
      {codes_page}CODE/.
      `{t} help --json` gives every code, its kind, title, messages and explanation, as JSON.
"
        ),
        "test" => format!(
            "  {t} test [-C opt-level=N] [-C overflow-checks=on|off] [-C debug-assertions=on|off] [--edition YEAR]
          FILE.rs [-- INPUT...]
  {t} test [--manifest-path PATH] [-p NAME] [--release | --profile NAME] --bin NAME [-- INPUT...]
      Compile a program as the compiler's tests do, write its .mir and .asm beside it, run it from
      main_bb0 on the input, and print its cards on one line.
"
        ),
        _ => return None,
    })
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
    /// a note or a warning about a file: what a build suggests the programmer change (`Built::advice`), or the dev
    /// settings `cargo cardiac` chose, the programmer's Cargo.toml not setting them (`cargo::dev_defaults`)
    Advice {
        file: &'a str,
        said: &'a Diag,
    },
    /// a resource a build left empty, having no file
    Note {
        deck: &'a str,
        resource: &'a str,
        file: &'a Path,
    },
    /// why the program can't be built, with its code (`codes`); and, of a program too big for CARDIAC's memory,
    /// where its cells go, which -v says as it says them of a program built
    Failed(&'a Diag, Option<&'a Sections>),
    /// an error of the tool's or the command line's, with no code; with `usage`, one in the command line, said with
    /// the usage after it
    Error {
        why: &'a str,
        usage: bool,
    },
}

impl Message<'_> {
    /// The least verbosity the message is said at: -1 (always) for an error; 0 for the rest, notes and warnings
    /// among them, which -q leaves out.
    fn level(&self) -> i8 {
        match self {
            Message::Failed(..) | Message::Error { .. } => -1,
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
            Message::Note { deck, resource, file } => codes::resource_empty(deck, resource, file.display()).to_string(),
            Message::Advice { file, said } => (*said).clone().at(file).to_string(),
            // an internal error asks for a bug report, as rustc's do
            Message::Failed(e, _) if e.is_bug() => format!("{e}\n{}", codes::BUG_REPORT),
            Message::Failed(e, Some(cells)) if v => format!("{e}\nnote: where the cells go: {cells}"),
            Message::Failed(e, _) => e.to_string(),
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
                    where_the_cells_go(&mut m, &b.sections);
                }
                m
            }
            Message::Note { deck, resource, file } => {
                let mut m = codes::resource_empty(deck, resource, file.display()).json();
                m["deck"] = (*deck).into();
                m["resource"] = (*resource).into();
                m["file"] = file.display().to_string().into();
                m
            }
            Message::Advice { file, said } => {
                let mut m = said.json();
                m["file"] = (*file).into();
                m
            }
            Message::Failed(e, cells) => {
                let mut m = e.json();
                if e.is_bug() {
                    let note = codes::BUG_REPORT.trim_start_matches("note: ");
                    m["bug_report"] = note.into();
                }
                if let Some(cells) = cells.filter(|_| v) {
                    where_the_cells_go(&mut m, cells);
                }
                m
            }
            Message::Error { why, .. } => json!({"reason": "error", "message": why}),
        }
    }
}

/// What -v adds to a message in JSON: where the cells go, as `functions` (each with its `name` and `cells`),
/// `variables` and `constants`.
fn where_the_cells_go(m: &mut Value, s: &Sections) {
    let functions: Vec<Value> = s.code.iter().map(|(name, n)| json!({"name": name, "cells": n})).collect();
    m["functions"] = functions.into();
    m["variables"] = s.variables.into();
    m["constants"] = s.constants.into();
}

/// The codes of the errors and warnings said for a person, for `more_information` to say where they are explained.
static SAID: Mutex<Vec<&'static codes::Code>> = Mutex::new(vec![]);

/// Say a message on standard error, in the format asked for, if the verbosity is at its level.
fn tell(m: Message) {
    if verbosity() < m.level() {
        return;
    }
    if json() {
        eprintln!("{}", m.json());
        return;
    }
    eprintln!("{}", m.text());
    if let Message::Failed(said, _) | Message::Advice { said, .. } = m {
        SAID.lock().unwrap().extend(said.codes());
    }
}

/// After a build, run, view or test: where the codes of its errors and warnings are explained, as rustc's last
/// lines say, for a person (not in JSON, and not with -q).
fn more_information() {
    let said = std::mem::take(&mut *SAID.lock().unwrap());
    if let Some(more) = codes::more_information(&said).filter(|_| verbosity() >= 0) {
        eprintln!("{more}");
    }
}

/// Whether what build says of the programs it made goes to standard error, as for `run`, whose standard output is
/// the program's.
static QUIET_STDOUT: AtomicBool = AtomicBool::new(false);

/// Say what a command made (build's sizes) on standard output, where it is the command's output; in
/// JSON on standard error with the rest, so a tool reads every message from one place.
fn say(m: Message) {
    if json() || QUIET_STDOUT.load(Ordering::Relaxed) {
        tell(m)
    } else if verbosity() >= m.level() {
        println!("{}", m.text())
    }
}

fn die(e: String) -> ! {
    tell(Message::Error { why: &e, usage: false });
    more_information();
    std::process::exit(1)
}

/// Stop, saying why the program can't be built.
fn fail(e: Diag) -> ! {
    fail_in(e, None)
}

/// Stop, saying why the program can't be built, and, when it is too big, where its cells go (with -v).
fn fail_in(e: Diag, cells: Option<Sections>) -> ! {
    tell(Message::Failed(&e, cells.as_ref()));
    more_information();
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
    /// The arguments of a command whose options are another program's (`new`, `debug`, `asm`), as given.
    rest: Vec<String>,
}

/// An edition, as rustc names them.
fn edition(year: String) -> String {
    if !["2015", "2018", "2021", "2024"].contains(&year.as_str()) {
        bad_usage(&format!("--edition {year}: Rust's editions are 2015, 2018, 2021 and 2024"))
    }
    year
}

impl Args {
    /// The profile and the flags are for different things: say so when they are given to the wrong one.
    fn settings_fit(&self) {
        let single = !self.files.is_empty();
        if single && self.profile.is_some() {
            bad_usage("--release and --profile choose a Cargo profile, and a single file has none: give it rustc's flags, -C opt-level=N, -C overflow-checks=on|off and -C debug-assertions=on|off")
        }
        if !single && self.rustc.edition.is_some() {
            bad_usage("--edition is for a single file; a package's edition is the one its Cargo.toml says")
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
    if matches!(command.as_str(), "--help" | "-h") {
        print!("{}", usage());
        std::process::exit(0)
    }
    if matches!(command.as_str(), "--version" | "-V" | "version") {
        println!("cargo-cardiac {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0)
    }
    // the commands whose options are another program's: everything after the command is theirs
    let passed = matches!(command.as_str(), "new" | "debug" | "asm" | "help" | "explain");
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
        rest: vec![],
    };
    if passed {
        r.rest = a[1..].to_vec();
        return r;
    }
    let compiles = matches!(r.command.as_str(), "build" | "test" | "run");
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
            "--edition" if compiles => {
                i += 1;
                let year = a.get(i).cloned().unwrap_or_else(|| bad_usage("--edition needs a year, 2024 say"));
                r.rustc.edition = Some(edition(year));
            }
            s if compiles && s.starts_with("--edition=") => {
                r.rustc.edition = Some(edition(s["--edition=".len()..].into()))
            }
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
        if p.name == env!("CARGO_PKG_NAME") {
            die(format!("{}'s binaries can't be compiled for CARDIAC: this is {}'s own package.\nRun `cargo {TOOL}` in a package whose binaries are CARDIAC programs, such as examples/ in rust-cardiac's repository, or name one with --manifest-path.", p.name, p.name));
        }
        if let Some(e) = &p.problem {
            fail(e.clone());
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
        tell(Message::Advice {
            file: &shown(&p.workspace_manifest.to_string_lossy()),
            said: &codes::dev_defaults(list.join(", ")),
        });
    }
    let (reports, ok) = cargo::build(p, programs, profile, &settings, verbosity() < 0).unwrap_or_else(|e| die(e));
    let mut made = vec![];
    for (t, r) in programs.iter().zip(reports) {
        match r.map(|r| r.outcome) {
            Some(Outcome::Failed(e, cells)) => {
                // what came before is said, as each program was compiled in turn
                say_made(&made, p);
                fail_in(e, cells)
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

/// Build the programs asked for, each package checked by cargo once, and report each one's size: what was built.
fn build(pkgs: &[Package], bins: &[String], profile: Option<&str>, emit: Emit) -> Vec<Built> {
    let mut built = vec![];
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
            built.push(b);
        }
    }
    built
}

/// Build one single file, as build does: what was built.
fn build_single(f: &str, rustc: &Rustc, auto_overlay: bool, emit: Emit) -> Built {
    let src = std::fs::read_to_string(f).unwrap_or_else(|e| die(format!("{f}: {e}")));
    if src.contains("cardiac::") {
        fail(codes::single_file_cardiac(f));
    }
    let Outcome::Built(b) = compile_single(f, rustc, auto_overlay, Mode::Build, emit) else {
        die(format!("the CARDIAC driver didn't build {f}"))
    };
    say(Message::Compiled(&b));
    note_empty(&b, &resources_beside(f));
    b
}

/// `cargo cardiac new`: `cargo new` with its arguments, then the `cardiac` crate added to the package's
/// dependencies and src/main.rs made the countdown from the README.
fn new(args: &[String]) -> ExitCode {
    // cargo new's options that take a value, so the path is told from them
    const VALUED: [&str; 7] = [
        "--vcs",
        "--edition",
        "--name",
        "--registry",
        "--color",
        "--config",
        "-Z",
    ];
    let mut path = None;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--lib" {
            bad_usage("a CARDIAC program is a binary: cargo cardiac new makes one, so --lib isn't for it")
        } else if matches!(a, "--help" | "-h") {
            print!("{}", usage());
            return ExitCode::SUCCESS;
        } else if VALUED.contains(&a) {
            i += 1;
        } else if !a.starts_with('-') {
            if path.is_some() {
                bad_usage("new takes one NAME (a path, as cargo new takes)")
            }
            path = Some(PathBuf::from(a));
        }
        i += 1;
    }
    let path = path.unwrap_or_else(|| bad_usage("new needs a NAME: cargo cardiac new countdown"));
    let made = std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .arg("new")
        .args(args)
        .status()
        .unwrap_or_else(|e| die(format!("cargo new: {e}")));
    if !made.success() {
        return ExitCode::FAILURE;
    }
    let manifest = path.join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).unwrap_or_else(|e| die(format!("{}: {e}", manifest.display())));
    let dep = format!("cardiac = \"{CARDIAC}\"\n");
    let text = match text.find("[dependencies]\n") {
        Some(k) => {
            let at = k + "[dependencies]\n".len();
            format!("{}{dep}{}", &text[..at], &text[at..])
        }
        None => format!("{text}\n[dependencies]\n{dep}"),
    };
    std::fs::write(&manifest, text).unwrap_or_else(|e| die(format!("{}: {e}", manifest.display())));
    let main = path.join("src/main.rs");
    let countdown = "\
use cardiac::*;

fn main() {
    let mut n = inp();
    while n >= 0 {
        out(n);
        n -= 1;
    }
}
";
    std::fs::write(&main, countdown).unwrap_or_else(|e| die(format!("{}: {e}", main.display())));
    eprintln!(
        "      Adding cardiac {CARDIAC} to the dependencies, and a countdown from the number it reads as src/main.rs"
    );
    ExitCode::SUCCESS
}

/// `cargo cardiac help`: the usage; with a command, its part of it (debug's and asm's are ekg's and carc's own);
/// with a code, what it means, as `rustc --explain` says it, and its page; `--json`, every code, for the website's
/// pages of them. `explain` is the same command, its old name.
fn help(args: &[String]) -> ExitCode {
    match args {
        [] => print!("{}", usage()),
        [json] if json == "--json" => println!("{:#}", codes::table_json()),
        [word] if word == "debug" => {
            return heartburn::tools::ekg::main(&format!("cargo {TOOL} debug"), vec!["--help".into()])
        }
        [word] if word == "asm" => {
            return heartburn::tools::carc::main(&format!("cargo {TOOL} asm"), vec!["--help".into()])
        }
        [word] if !word.starts_with('-') => {
            if let Some(part) = command_usage(word) {
                print!("{part}");
            } else if let Some(c) = codes::find(word) {
                // the address on a line of its own, so a terminal makes it a link
                print!("{}", c.explanation);
                println!("\n{}{}/", codes::PAGE, c.id);
            } else if codes::looks_like_a_code(word) {
                die(format!("{word} is not a cargo-cardiac code"))
            } else {
                die(format!(
                    "{word} is not a command or a code: the commands are {}, and a code is RCE, RCW or RCI and \
                     four digits",
                    COMMANDS.join(", ")
                ))
            }
        }
        _ => bad_usage("help takes one COMMAND, or one CODE (RCE0002 say), or --json for every code"),
    }
    ExitCode::SUCCESS
}

/// `cargo cardiac run`: build the one program asked for, then run its deck on burn, the map beside it.
fn run(a: &Args) -> ExitCode {
    QUIET_STDOUT.store(true, Ordering::Relaxed);
    let b = built_one(a, "run", Emit::default());
    more_information();
    let mut args = vec![b.deck_file.clone()];
    if let Some(input) = &a.input {
        args.push("--".into());
        args.extend(input.iter().cloned());
    }
    heartburn::tools::burn::main(&format!("cargo {TOOL} run"), args)
}

/// The one program `run` or `view` is about, built with `emit`: the package's binary target (`--bin` names one when
/// there are several), or the single FILE.rs.
fn built_one(a: &Args, command: &str, emit: Emit) -> Built {
    let built = if a.files.is_empty() {
        let pkgs = packages(a.manifest.as_deref(), a.package.as_deref()).unwrap_or_else(|e| die(e));
        let programs = chosen(&pkgs, &a.bins);
        match programs.as_slice() {
            [(p, t)] => build(
                std::slice::from_ref(*p),
                &[t.name.clone()],
                a.profile.as_deref(),
                emit,
            ),
            [] => die(format!("no program to {command}: the package has no binary target")),
            more => {
                let names: Vec<&str> = more.iter().map(|(_, t)| t.name.as_str()).collect();
                bad_usage(&format!(
                    "{command} takes one program, and there are {}: {}. Name one with --bin",
                    names.len(),
                    names.join(", ")
                ))
            }
        }
    } else {
        if a.files.len() > 1 {
            bad_usage(&format!("{command} takes one FILE.rs; its input goes after --"))
        }
        vec![build_single(&a.files[0], &a.rustc, a.auto_overlay, emit)]
    };
    built.into_iter().next().unwrap_or_else(|| die(format!("nothing was built to {command}")))
}

/// Where `view` opens a program: Cardigan, the CARDIAC IDE in a browser. CARDIGAN_URL says another (a local copy).
const CARDIGAN: &str = "https://lab.strangequery.com/cardigan/ide/";

/// Build the program and open it in Cardigan, stopped at the start with its input in the reader: the address carries
/// the listing (a program in phases, each phase's deck), as the site's own buttons hand Cardigan a program. It all
/// goes after the address's #, which a browser never sends, so the program stays on this machine.
fn view(a: &Args) -> ExitCode {
    let emit = Emit { deck: true, listing: true, map: true, asm: false };
    let b = built_one(a, "view", emit);
    more_information();
    let deck = Path::new(&b.deck_file);
    let name = deck.file_stem().and_then(|s| s.to_str()).unwrap_or("program").to_string();
    let read = |p: &Path| std::fs::read_to_string(p).unwrap_or_else(|e| die(format!("{}: {e}", p.display())));
    let mut q = vec![("name", name)];
    if b.code.len() == 1 {
        q.push(("cq", read(&deck.with_extension("cq"))));
    } else {
        for n in 1..=b.code.len() {
            q.push(("cqb", read(&heartburn::tape::Tape::phase_path(deck, n))));
        }
    }
    if let Some(input) = &a.input {
        q.push(("in", input.join(" ")));
    }
    let base = std::env::var("CARDIGAN_URL").unwrap_or_else(|_| CARDIGAN.to_string());
    let fragment: Vec<String> = q.iter().map(|(k, v)| format!("{k}={}", percent(v))).collect();
    let url = format!("{base}#{}", fragment.join("&"));
    eprintln!("     Opening {} in Cardigan", b.file);
    if !open_in_browser(&url) {
        eprintln!("note: no browser opened; the address is below");
    }
    println!("{url}");
    ExitCode::SUCCESS
}

/// Open an address in the browser, as the system's own command does: whether it did.
fn open_in_browser(url: &str) -> bool {
    let opener = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).status()
    } else if cfg!(windows) {
        std::process::Command::new("cmd").args(["/C", "start", ""]).arg(url).status()
    } else {
        std::process::Command::new("xdg-open").arg(url).status()
    };
    opener.is_ok_and(|s| s.success())
}

/// `s` for the address: letters, digits and `-_.~` as they are, every other byte as %XX.
fn percent(s: &str) -> String {
    s.bytes()
        .map(|c| match c {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (c as char).to_string(),
            _ => format!("%{c:02X}"),
        })
        .collect()
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
    match driver::single(f, rustc, &settings, verbosity() < 0).unwrap_or_else(|e| die(e)) {
        Some(r) => match r.outcome {
            Outcome::Failed(e, cells) => fail_in(e, cells),
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
    println!("{}", punched.unwrap_or_else(|e| fail(e)));
}

/// The `resources/` directory beside a file: a package's Cargo.toml or a single source file.
fn resources_beside(file: &str) -> PathBuf {
    Path::new(file).parent().unwrap_or(Path::new("")).join("resources")
}

/// Say what a build suggests the programmer change, and which resources it left empty, having no file (burn binds
/// the deck's slot when it runs it).
fn note_empty(b: &Built, dir: &Path) {
    for said in &b.advice {
        tell(Message::Advice { file: &b.file, said });
    }
    for name in &b.empty {
        tell(Message::Note {
            deck: &b.deck_file,
            resource: name,
            file: &dir.join(name),
        });
    }
}

/// After a build, run or view: whether a newer cargo-cardiac is out (`cargo_cardiac::update`).
fn update_note() {
    cargo_cardiac::update::note(verbosity() < 0 || json());
}

/// A panic of `cargo cardiac`'s own is said as Rust says one, then asks for a bug report, as rustc's do.
fn bug_report_on_panic() {
    let said = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        said(info);
        eprintln!("{}", codes::BUG_REPORT);
    }));
}

fn main() -> ExitCode {
    bug_report_on_panic();
    let a = args();
    match a.command.as_str() {
        "new" => return new(&a.rest),
        "debug" => return heartburn::tools::ekg::main(&format!("cargo {TOOL} debug"), a.rest),
        "asm" => return heartburn::tools::carc::main(&format!("cargo {TOOL} asm"), a.rest),
        // explain CODE, as it was before help took codes, stays
        "help" | "explain" => return help(&a.rest),
        "setup" => {
            driver::setup().unwrap_or_else(|e| die(e));
            return ExitCode::SUCCESS;
        }
        _ => {}
    }
    let single = !a.files.is_empty();
    let pkgs = || packages(a.manifest.as_deref(), a.package.as_deref()).unwrap_or_else(|e| die(e));
    let profile = a.profile.as_deref();
    if matches!(a.command.as_str(), "build" | "test" | "run") {
        a.settings_fit()
    }
    match a.command.as_str() {
        "build" if single => {
            for f in &a.files {
                build_single(f, &a.rustc, a.auto_overlay, a.emit);
            }
            more_information();
            update_note();
        }
        "build" => {
            build(&pkgs(), &a.bins, profile, a.emit);
            more_information();
            update_note();
        }
        "run" => {
            let ran = run(&a);
            update_note();
            return ran;
        }
        "view" => {
            let viewed = view(&a);
            update_note();
            return viewed;
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
            more_information();
        }
        // a command that is gone
        "cardiasm" => die(format!("cargo {TOOL} cardiasm is gone")),
        c => bad_usage(&format!("unknown command {c}")),
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_says_only_errors() {
        let note = codes::dev_defaults("opt-level = 2");
        let error = codes::too_big(143);
        let file = Path::new("resources/font");
        let said = [
            Message::Advice {
                file: "Cargo.toml",
                said: &note,
            },
            Message::Note {
                deck: "big.cqb",
                resource: "font",
                file,
            },
            Message::Compiling {
                package: "big",
                programs: 1,
                profile: "dev",
            },
        ];
        for m in said {
            assert_eq!(m.level(), 0, "{}", m.text());
        }
        assert_eq!(Message::Failed(&error, None).level(), -1);
        assert_eq!(Message::Error { why: "no", usage: false }.level(), -1);
    }

    /// The only test that sets the verbosity.
    #[test]
    fn verbose_says_where_the_cells_go_of_a_program_too_big() {
        let error = codes::too_big(143).at("src/main.rs");
        let cells = program::Sections {
            code: vec![("main".into(), 120), ("rt_panic".into(), 2)],
            variables: 18,
            constants: 3,
        };
        let m = Message::Failed(&error, Some(&cells));
        assert_eq!(m.text(), error.to_string());
        VERBOSITY.store(1, Ordering::Relaxed);
        let (text, json) = (m.text(), m.json());
        VERBOSITY.store(0, Ordering::Relaxed);
        assert_eq!(
            text,
            format!("{error}\nnote: where the cells go: code: main 120, rt_panic 2; data: 18 variables, 3 constants")
        );
        assert_eq!(json["code"], "RCE0032");
        assert_eq!(json["functions"], json!([{"name": "main", "cells": 120}, {"name": "rt_panic", "cells": 2}]));
        assert_eq!((json["variables"].as_u64(), json["constants"].as_u64()), (Some(18), Some(3)));
    }
}
