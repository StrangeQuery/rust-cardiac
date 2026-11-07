//! The CARDIAC driver (`driver/`), and what passes between it and `cargo cardiac`. The driver is rustc, built as a
//! library on a pinned nightly, and it is the CARDIAC backend: once rustc has analysed a program it converts the
//! program's MIR to the compiler's IR, and the MIR of the crates the program uses from their metadata, links them,
//! compiles the program and writes what `cargo cardiac` writes (the listing, the tape, the panic map; for `test`
//! the program's IR printed as MIR and the listing), all in rustc's process. `cargo cardiac` runs it as cargo's
//! `RUSTC` for a package (`cargo check`, `cargo::build`) and itself for a single file (`single`).
//!
//! How the two talk, in each direction:
//! - **Settings**, from `cargo cardiac` to the driver: `CARDIAC_BUILD`, a JSON object (`Settings`): `build` or
//!   `test`; the programs, each by its source file (the driver compiles a crate whose root is one of them, given
//!   `--cfg cardiac`, `program::CFG`), with its name, its source as it is shown and its resources directory; and
//!   the crates the programs use, by the names they use them by, in the order the compiler links them. What rustc
//!   knows the driver takes from rustc's session: the opt-level and debug assertions (the Cargo profile's, as cargo
//!   resolved it, or a single file's `-C` flags). `CARDIAC_RUN` is new for each run of `cargo cardiac`, and a
//!   program's crate depends on it (rustc's env-dep), so cargo always compiles a program again: its outputs are
//!   not cargo's to know about, and the compiler or a resource may have changed since.
//! - **What it made**, from the driver back: a line of JSON on rustc's standard error, `{"$message_type":
//!   "cardiac", ...}` (`Report`). Under cargo, rustc's standard error is JSON already (`--error-format=json`), and
//!   cargo hands the line on as a `compiler-message` of the program's target; for a single file `cargo cardiac`
//!   reads the driver's standard error itself. A program that doesn't compile for CARDIAC is a rustc error too
//!   (rustc fails, and with it cargo's build): `cargo cardiac` says it as it says any error, `error: FILE: ...`,
//!   and renders rustc's other diagnostics as cargo would (`Listener`).
//!
//! The driver is found from `CARDIAC_DRIVER`, else beside `cargo-cardiac` itself, else in this repository's
//! `driver/` build, else where an earlier run downloaded it (`installed`). With none of those, `cargo cardiac`
//! downloads the one this version's release has for its platform. Being rustc's own libraries, it runs only with
//! the pinned nightly's: rustup gives their place, and installs that nightly (its minimal profile, which is all
//! running the driver needs) when it has none. It is required: there is no plain-rustc way to build any more
//! (`missing` says how to build it, for a platform the release has no driver for).
use std::collections::HashMap;
use std::io::{BufRead, BufReader, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use serde_json::{json, Value};

use crate::codes::Diag;
use crate::compile::program::{Built, Emit, OptLevel, Rustc, Sections, CFG};

/// The nightly the driver is built with: the last of 1.97's, whose MIR is the same as 1.97's.
pub const TOOLCHAIN: &str = "nightly-2026-05-22";

/// That nightly for the platform cargo-cardiac was built for, which is the driver's: on Windows, rustup's plain
/// `nightly-2026-05-22` is the MSVC one or the GNU one as rustup's default host says, and a driver loads only the
/// libraries of its own.
fn toolchain() -> String {
    format!("{TOOLCHAIN}-{}", env!("CARDIAC_HOST"))
}

/// The variable that carries `Settings` to the driver.
pub const SETTINGS: &str = "CARDIAC_BUILD";

/// The variable whose value is new for each run of `cargo cardiac`, which a program's crate depends on.
pub const RUN: &str = "CARDIAC_RUN";

const NAME: &str = if cfg!(windows) {
    "cardiac-driver.exe"
} else {
    "cardiac-driver"
};

/// The driver, and the directory of the toolchain's libraries it loads.
pub struct Driver {
    pub exe: PathBuf,
    libs: PathBuf,
}

impl Driver {
    /// A command to run the driver as rustc.
    pub fn rustc(&self) -> Command {
        let mut cmd = Command::new(&self.exe);
        self.env(&mut cmd);
        cmd
    }

    /// Give a command (cargo's, say, which runs rustc) what the driver needs to load: the toolchain's libraries.
    pub fn env(&self, cmd: &mut Command) {
        let var = if cfg!(target_os = "macos") {
            "DYLD_FALLBACK_LIBRARY_PATH"
        } else if cfg!(windows) {
            "PATH"
        } else {
            "LD_LIBRARY_PATH"
        };
        let mut paths = vec![self.libs.clone()];
        if let Some(old) = std::env::var_os(var) {
            paths.extend(std::env::split_paths(&old));
        }
        if let Ok(joined) = std::env::join_paths(paths) {
            cmd.env(var, joined);
        }
    }
}

/// The driver (looked for once), or why there is none to compile with and how to get one.
pub fn find() -> Result<&'static Driver, String> {
    static FOUND: OnceLock<Result<Driver, String>> = OnceLock::new();
    FOUND.get_or_init(look).as_ref().map_err(|e| e.clone())
}

fn look() -> Result<Driver, String> {
    let exe = match on_disk() {
        Some(exe) => exe,
        // a driver named with CARDIAC_DRIVER that isn't there is a mistake to say, not one to download past
        None if std::env::var_os("CARDIAC_DRIVER").is_some() => return Err(missing()),
        None => download()?,
    };
    let libs = match sysroot() {
        Some(libs) => libs,
        None => install_toolchain()?,
    };
    Ok(Driver { exe, libs })
}

/// The driver already on this machine: the one CARDIAC_DRIVER names, else beside `cargo-cardiac`, else this
/// repository's `driver/` build, else where an earlier run downloaded it.
fn on_disk() -> Option<PathBuf> {
    let given = std::env::var_os("CARDIAC_DRIVER").map(PathBuf::from);
    let here = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf));
    let candidates = given.into_iter().chain(here.iter().flat_map(|dir| {
        [
            dir.join(NAME),
            // a build in this repository: target/release/cargo-cardiac, and driver/target/release/cardiac-driver
            dir.join("../../driver/target/release").join(NAME),
        ]
    }));
    candidates
        .chain(installed().map(|dir| dir.join(NAME)))
        .find(|p| p.is_file())
        .map(|p| std::fs::canonicalize(&p).unwrap_or(p))
}

/// How much the pinned nightly takes, said before rustup installs it.
const TOOLCHAIN_SIZE: &str = "about 550 MB on disk";

/// How much the driver's download is, said before it starts.
const DRIVER_SIZE: &str = "about 1 MB";

/// Install the pinned nightly with rustup, in its minimal profile (all running the driver needs), and give the
/// directory of its libraries.
fn install_toolchain() -> Result<PathBuf, String> {
    let name = toolchain();
    eprintln!(
        "  Installing {name} with rustup, in its minimal profile ({TOOLCHAIN_SIZE}), once: the CARDIAC driver runs on \
         that nightly's libraries"
    );
    let _ = Command::new("rustup")
        .args(["toolchain", "install", &name, "--profile", "minimal"])
        .status();
    sysroot().ok_or_else(|| format!("the CARDIAC driver needs the {name} toolchain's libraries, and rustup couldn't install it: install it with `rustup toolchain install {name} --profile minimal`"))
}

/// `cargo cardiac setup`: what the first build would fetch, fetched now: the driver for this platform and the
/// pinned nightly. Each is said with its size before it starts; what is already here is said as being here.
pub fn setup() -> Result<(), String> {
    let exe = on_disk();
    let libs = sysroot();
    if let (Some(exe), Some(_)) = (&exe, &libs) {
        eprintln!(
            "cargo-cardiac is already set up: the CARDIAC driver is {}, and rustup has {}",
            exe.display(),
            toolchain()
        );
        return Ok(());
    }
    let mut todo = vec![];
    if exe.is_none() {
        todo.push(format!("the CARDIAC driver ({DRIVER_SIZE})"));
    }
    if libs.is_none() {
        todo.push(format!(
            "{} in rustup's minimal profile ({TOOLCHAIN_SIZE})",
            toolchain()
        ));
    }
    eprintln!("   Setting up cargo-cardiac: {}", todo.join(", and "));
    let exe = match exe {
        Some(exe) => exe,
        None if std::env::var_os("CARDIAC_DRIVER").is_some() => return Err(missing()),
        None => download()?,
    };
    let libs = match libs {
        Some(libs) => libs,
        None => install_toolchain()?,
    };
    eprintln!(
        "    Finished: the CARDIAC driver is {}, and its libraries are in {}",
        exe.display(),
        libs.display()
    );
    Ok(())
}

/// The directory of the pinned nightly's libraries, as rustup has it, or None if rustup has no such toolchain.
fn sysroot() -> Option<PathBuf> {
    let out = Command::new("rustup")
        .args(["run", &toolchain(), "rustc", "--print", "sysroot"])
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let sysroot = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    Some(sysroot.join(if cfg!(windows) { "bin" } else { "lib" }))
}

/// Where this version's downloaded driver lives: CARDIAC_HOME, else .cardiac in the home directory, in a folder
/// for the version (a newer cargo-cardiac downloads its own driver, and the old one stays for the old one).
fn installed() -> Option<PathBuf> {
    let home = std::env::var_os("CARDIAC_HOME").map(PathBuf::from).or_else(|| {
        std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(|h| PathBuf::from(h).join(".cardiac"))
    })?;
    Some(home.join(format!("driver-{}", env!("CARGO_PKG_VERSION"))))
}

/// Where the releases are: each one's driver for a platform is cardiac-driver-TARGET.tar.gz in its release.
const RELEASES: &str = "https://github.com/StrangeQuery/rust-cardiac/releases/download";

/// Download this version's driver for this platform into `installed`, with curl and tar (both come with macOS,
/// Windows 10 and later, and every Linux), and give where it is. Unpacked beside its place and moved there whole,
/// so a download that stops part way leaves nothing that looks installed.
fn download() -> Result<PathBuf, String> {
    let host = env!("CARDIAC_HOST");
    let version = env!("CARGO_PKG_VERSION");
    let dir = installed().ok_or_else(|| format!("{}: and no home directory to download one into", missing()))?;
    let url = format!("{RELEASES}/v{version}/cardiac-driver-{host}.tar.gz");
    eprintln!(
        " Downloading the CARDIAC driver for {host} ({DRIVER_SIZE}), once, into {}",
        dir.display()
    );
    let part = dir.with_extension("part");
    let _ = std::fs::remove_dir_all(&part);
    std::fs::create_dir_all(&part).map_err(|e| format!("{}: {e}", part.display()))?;
    let archive = part.join("driver.tar.gz");
    let fetched = Command::new("curl")
        .args(["-fsSL", "--proto", "=https", "-o"])
        .arg(&archive)
        .arg(&url)
        .status()
        .is_ok_and(|s| s.success());
    if !fetched {
        let _ = std::fs::remove_dir_all(&part);
        return Err(format!(
            "couldn't download the CARDIAC driver from {url} (this platform may have no driver in the release): {}",
            missing()
        ));
    }
    let unpacked = Command::new("tar")
        .arg("-xzf")
        .arg(&archive)
        .arg("-C")
        .arg(&part)
        .status()
        .is_ok_and(|s| s.success());
    let _ = std::fs::remove_file(&archive);
    if !unpacked || !part.join(NAME).is_file() {
        let _ = std::fs::remove_dir_all(&part);
        return Err(format!(
            "the CARDIAC driver downloaded from {url} didn't unpack to {NAME}"
        ));
    }
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::rename(&part, &dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(dir.join(NAME))
}

/// Why there is no driver, and how to build one.
pub fn missing() -> String {
    format!(
        "no CARDIAC driver ({NAME}), which compiles every program: build it in rust-cardiac's repository with \
         `cd driver && cargo build --release` (rustup installs the nightly it needs, {TOOLCHAIN}, with its \
         rustc-dev component, from driver/rust-toolchain.toml), then put it beside cargo-cardiac or name it with \
         CARDIAC_DRIVER"
    )
}

/// What the driver is asked to make of a program: `cargo cardiac build`'s outputs, or `test`'s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The listing (NAME.asm, or one per phase), the tape and the panic map, beside the source.
    Build,
    /// The program's IR printed as MIR (FILE.mir) and its listing as the compiler's tests run it (FILE.asm).
    Test,
}

/// A program to compile: the source file of its crate's root, from anywhere (absolute), its name, its source as
/// `cargo cardiac` shows it (in messages and in its outputs' names), where its resources are, and whether the
/// compiler splits it into phases itself (`auto-overlay`), and what build writes beside it (`--emit`).
#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub source: PathBuf,
    pub name: String,
    pub file: String,
    pub resources: PathBuf,
    pub auto_overlay: bool,
    pub emit: Emit,
}

/// A crate the programs use: the name they use it by, and its crate's name, by which the driver finds it among
/// the crates rustc loaded.
#[derive(Clone, Debug, PartialEq)]
pub struct Dep {
    pub name: String,
    pub krate: String,
}

/// What `cargo cardiac` tells the driver (`CARDIAC_BUILD`).
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub mode: Mode,
    pub programs: Vec<Program>,
    /// Deepest first, as the compiler links them.
    pub crates: Vec<Dep>,
    /// A single file, compiled by `cargo cardiac` running the driver itself: there is nothing for rustc to write
    /// once the driver is done.
    pub single: bool,
}

impl Settings {
    pub fn json(&self) -> String {
        let programs: Vec<Value> = self
            .programs
            .iter()
            .map(|p| json!({"source": p.source, "name": p.name, "file": p.file, "resources": p.resources, "auto_overlay": p.auto_overlay, "emit": p.emit.names().join(",")}))
            .collect();
        let crates: Vec<Value> = self
            .crates
            .iter()
            .map(|c| json!({"name": c.name, "crate": c.krate}))
            .collect();
        let mode = match self.mode {
            Mode::Build => "build",
            Mode::Test => "test",
        };
        json!({"mode": mode, "programs": programs, "crates": crates, "single": self.single}).to_string()
    }

    pub fn from_json(text: &str) -> Result<Settings, String> {
        let bad = || format!("{SETTINGS} isn't as cargo cardiac writes it: {text}");
        let v: Value = serde_json::from_str(text).map_err(|_| bad())?;
        let s = |v: &Value| v.as_str().map(str::to_string).ok_or_else(bad);
        let list = |v: &Value| v.as_array().cloned().ok_or_else(bad);
        let mode = match v["mode"].as_str() {
            Some("build") => Mode::Build,
            Some("test") => Mode::Test,
            _ => return Err(bad()),
        };
        let programs = list(&v["programs"])?
            .iter()
            .map(|p| {
                Ok(Program {
                    source: PathBuf::from(s(&p["source"])?),
                    name: s(&p["name"])?,
                    file: s(&p["file"])?,
                    resources: PathBuf::from(s(&p["resources"])?),
                    auto_overlay: p["auto_overlay"].as_bool().ok_or_else(bad)?,
                    emit: Emit::parse(&s(&p["emit"])?)?,
                })
            })
            .collect::<Result<_, String>>()?;
        let crates = list(&v["crates"])?
            .iter()
            .map(|c| {
                Ok(Dep {
                    name: s(&c["name"])?,
                    krate: s(&c["crate"])?,
                })
            })
            .collect::<Result<_, String>>()?;
        Ok(Settings {
            mode,
            programs,
            crates,
            single: v["single"].as_bool().ok_or_else(bad)?,
        })
    }

    /// The settings `cargo cardiac` gave, if it is what runs this driver.
    pub fn from_env() -> Option<Result<Settings, String>> {
        let text = std::env::var(SETTINGS).ok()?;
        Some(Settings::from_json(&text))
    }

    /// Give a command the settings, and this run's `CARDIAC_RUN`.
    pub fn env(&self, cmd: &mut Command) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        cmd.env(SETTINGS, self.json())
            .env(RUN, format!("{}-{nanos}", std::process::id()));
    }
}

/// What the driver made of a program.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// `build`: what `program::build` wrote.
    Built(Built),
    /// `test`: the listing's file, as it is shown, and the opt-level, for a hint if the run fails.
    Tested { asm: String, opt_level: OptLevel },
    /// It doesn't compile for CARDIAC: the error, as `cargo cardiac` says it; and where the cells go, when it
    /// compiled but is too big for CARDIAC's memory (`program::NotBuilt`).
    Failed(Diag, Option<Sections>),
}

/// The driver's line about one program (`{"$message_type": "cardiac", "program": NAME, ...}`).
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub program: String,
    pub outcome: Outcome,
}

impl Report {
    /// The line, a JSON object.
    pub fn line(&self) -> String {
        let mut v = json!({"$message_type": "cardiac", "program": self.program});
        match &self.outcome {
            Outcome::Built(b) => v["built"] = b.json(),
            Outcome::Tested { asm, opt_level } => v["tested"] = json!({"asm": asm, "opt_level": opt_level.to_string()}),
            Outcome::Failed(e, sections) => {
                v["error"] = e.json();
                if let Some(s) = sections {
                    v["sections"] = s.json();
                }
            }
        }
        v.to_string()
    }

    /// A report, from a message rustc's standard error carried, if it is one.
    pub fn of(v: &Value) -> Option<Report> {
        if v["$message_type"] != "cardiac" {
            return None;
        }
        let outcome = if v["error"].is_object() {
            let sections = match &v["sections"] {
                Value::Null => None,
                s => Some(Sections::from_json(s)?),
            };
            Outcome::Failed(Diag::from_json(&v["error"])?, sections)
        } else if v["built"].is_object() {
            Outcome::Built(Built::from_json(&v["built"])?)
        } else {
            let t = &v["tested"];
            Outcome::Tested {
                asm: t["asm"].as_str()?.to_string(),
                opt_level: t["opt_level"].as_str()?.parse().ok()?,
            }
        };
        Some(Report {
            program: v["program"].as_str()?.to_string(),
            outcome,
        })
    }
}

/// What rustc says while the driver compiles: the driver's reports, kept by program, and rustc's diagnostics,
/// rendered on standard error as cargo renders them, but for those that are the driver's errors, which `cargo
/// cardiac` says itself; with `quiet` (-q), only its errors.
#[derive(Default)]
pub struct Listener {
    quiet: bool,
    pub reports: HashMap<String, Report>,
    /// the driver's errors not yet seen as diagnostics
    pending: Vec<String>,
    /// rustc's errors that are not the driver's
    errors: usize,
}

impl Listener {
    /// One that renders rustc's diagnostics, or with `quiet` only its errors.
    pub fn new(quiet: bool) -> Listener {
        Listener {
            quiet,
            ..Listener::default()
        }
    }

    /// Whether a diagnostic of rustc's isn't rendered: with -q, all but the errors (warnings, and rustc's notes of
    /// its own).
    fn leaves_out(&self, m: &Value) -> bool {
        self.quiet && matches!(m["level"].as_str(), Some("warning" | "note" | "help" | "failure-note"))
    }

    /// One message from rustc's standard error.
    pub fn take(&mut self, m: &Value) {
        if let Some(r) = Report::of(m) {
            if let Outcome::Failed(e, _) = &r.outcome {
                self.pending.push(e.to_string());
            }
            self.reports.insert(r.program.clone(), r);
            return;
        }
        if m["$message_type"] != "diagnostic" {
            return;
        }
        let text = m["message"].as_str().unwrap_or("");
        if let Some(k) = self.pending.iter().position(|e| e == text) {
            self.pending.remove(k);
            return;
        }
        let error = m["level"] == "error";
        if self.leaves_out(m) {
            return;
        }
        // rustc's last word on a failure that was only the driver's
        if error && text.starts_with("aborting due to") && self.errors == 0 {
            return;
        }
        if error {
            self.errors += 1;
        }
        if let Some(r) = m["rendered"].as_str() {
            eprint!("{r}");
        }
    }
}

/// rustc's flags to render its diagnostics as JSON, in color when standard error is a terminal, as cargo has it do.
fn json_errors(cmd: &mut Command) {
    cmd.arg("--error-format=json");
    if std::io::stderr().is_terminal() {
        cmd.arg("--json=diagnostic-rendered-ansi");
    }
}

/// Compile a single file, which uses no other crate, with these flags: rustc compiles it as a library, so nothing
/// calls its `fn main` (not a warning worth printing), and the driver makes what `settings` asks of it. Its report,
/// or None if rustc stopped first (an error of Rust's, rendered already). With `quiet` (-q), only rustc's errors
/// are rendered.
pub fn single(file: &str, rustc: &Rustc, settings: &Settings, quiet: bool) -> Result<Option<Report>, String> {
    let driver = find()?;
    let mut cmd = driver.rustc();
    cmd.args([
        "--edition",
        rustc.edition.as_deref().unwrap_or("2021"),
        "--crate-type",
        "lib",
        "--cfg",
        CFG,
        "-A",
        "dead_code",
    ])
    .args(&rustc.flags);
    json_errors(&mut cmd);
    cmd.arg(file);
    settings.env(&mut cmd);
    let mut child = cmd
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{}: {e}", driver.exe.display()))?;
    let mut listener = Listener::new(quiet);
    for line in BufReader::new(child.stderr.take().unwrap()).lines() {
        let line = line.map_err(|e| format!("{}: {e}", driver.exe.display()))?;
        match serde_json::from_str::<Value>(&line) {
            Ok(m) if m.is_object() => listener.take(&m),
            // anything else rustc says (a panic, say) as it says it
            _ => eprintln!("{line}"),
        }
    }
    child.wait().map_err(|e| format!("{}: {e}", driver.exe.display()))?;
    Ok(listener.reports.into_values().next())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_carries_where_the_cells_go() {
        let sections = Sections {
            code: vec![("main".into(), 90), ("rt_div".into(), 12)],
            variables: 7,
            constants: 2,
        };
        let r = Report {
            program: "big".into(),
            outcome: Outcome::Failed(crate::codes::too_big(111).at("src/main.rs"), Some(sections)),
        };
        let line: Value = serde_json::from_str(&r.line()).unwrap();
        assert_eq!(Report::of(&line), Some(r));
        let r = Report {
            program: "bad".into(),
            outcome: Outcome::Failed(crate::codes::runs_past_98().at("src/main.rs"), None),
        };
        let line: Value = serde_json::from_str(&r.line()).unwrap();
        assert_eq!(Report::of(&line), Some(r));
    }

    #[test]
    fn quiet_renders_only_rustcs_errors() {
        let (quiet, said) = (Listener::new(true), Listener::new(false));
        for level in ["warning", "note", "help", "failure-note"] {
            let m = json!({"$message_type": "diagnostic", "level": level, "message": "unused variable"});
            assert!(quiet.leaves_out(&m) && !said.leaves_out(&m), "{level}");
        }
        let m = json!({"$message_type": "diagnostic", "level": "error", "message": "mismatched types"});
        assert!(!quiet.leaves_out(&m));
    }
}
