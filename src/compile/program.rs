//! From a Rust source file to a deck: rustc's MIR, as the CARDIAC driver converts it to the IR, then the compiler.
//! The driver (`driver/`, `crate::driver`) is rustc, and it runs all of this in rustc's process once rustc has
//! analysed the program.
//!
//! `build` compiles a program and writes, next to its source,
//!   NAME.asm                               one phase: code and data, started at `start:`
//!   NAME_data.asm, NAME_1.asm..            more phases (overlay() calls): the data block, then each phase's code,
//!                                          started at start, p2, p3 ...
//!   NAME.cqb                               the deck CARDIAC loads it from (`tape`)
//!   NAME.map                               its panic map (`panics`)
//!
//! The MIR is rustc's, made with the program's settings. A program in a Cargo package gets its package's profile,
//! as cargo resolves it (`cargo::build`): its opt-level, its overflow checks (bounds checks are always on), and the
//! rest, as a native build of the same profile would. A single file has no Cargo.toml, so it takes rustc's own
//! flags instead (`Rustc`). The opt-level also sets how hard the compiler's own passes work (`OptLevel::passes`).
//! Small helper functions are inlined in the MIR, by rustc's inliner, which the driver runs at opt-level 1 and s
//! too (`driver/src/inline.rs`), and cost nothing on CARDIAC (mark one #[inline(never)] to keep a single shared
//! copy).
//!
//! A program can use other crates. Each is compiled by rustc too, its MIR in its metadata
//! (`-Zalways-encode-mir`), from which the driver converts it when it compiles a program that uses it
//! (`crates::link` joins them). rustc is given `--cfg cardiac` throughout (`CFG`), which is how the built-ins
//! crate knows to leave its functions as calls for the compiler to see.
use std::path::Path;

use super::crates::{link, Crate};
use super::enums::EnumDef;
use super::ir::visit_mut::{walk_body, walk_path, walk_rvalue, walk_statement, walk_terminator, walk_ty, VisitMut};
use super::structs::StructDef;
use super::{cells, compile_mir, split_phases};
use super::{ir, machine};
use crate::codes::{self, Diag};

/// The cfg rustc is given when it compiles for CARDIAC (`#[cfg(cardiac)]` in a crate's source).
pub const CFG: &str = "cardiac";

/// rustc's `-C opt-level`, named as rustc names its levels, from a profile's `opt-level` or a single file's flag.
/// It sets how hard the compiler's own passes work too (`passes`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptLevel {
    No,
    Less,
    More,
    Aggressive,
    Size,
    SizeMin,
}

impl std::str::FromStr for OptLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<OptLevel, String> {
        match s {
            "0" => Ok(OptLevel::No),
            "1" => Ok(OptLevel::Less),
            "2" => Ok(OptLevel::More),
            "3" => Ok(OptLevel::Aggressive),
            "s" => Ok(OptLevel::Size),
            "z" => Ok(OptLevel::SizeMin),
            _ => Err(format!("opt-level={s}: rustc's levels are 0, 1, 2, 3, s and z")),
        }
    }
}

impl std::fmt::Display for OptLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(match self {
            OptLevel::No => "0",
            OptLevel::Less => "1",
            OptLevel::More => "2",
            OptLevel::Aggressive => "3",
            OptLevel::Size => "s",
            OptLevel::SizeMin => "z",
        })
    }
}

impl OptLevel {
    /// Whether the compiler's passes run. At 0 every statement is lowered as rustc wrote it; from 1 up they all
    /// run (3, s and z work as 2 does: there is nothing more for them to do yet).
    pub fn passes(self) -> bool {
        self != OptLevel::No
    }
}

/// A compile error, with a note when the opt-level is 0: there rustc's MIR is unoptimized and the compiler's
/// passes are off, which few programs survive (an iterator's `next` is a call, and a program rarely fits).
pub fn hint(e: Diag, opt_level: OptLevel) -> Diag {
    if opt_level != OptLevel::No {
        return e;
    }
    e.with_note(codes::opt_level_zero())
}

/// rustc's flags for a single file, which has no Cargo profile: `-C opt-level`, `-C overflow-checks` and `-C
/// debug-assertions`, each passed to rustc as given. Without them rustc's defaults hold: opt-level 0, and overflow
/// checks and debug assertions on at opt-level 0 and off above it.
#[derive(Clone, Debug)]
pub struct Rustc {
    pub opt_level: OptLevel,
    /// `-C debug-assertions`, if given: rustc's default is on at opt-level 0 and off above it.
    pub debug_assertions: Option<bool>,
    /// The flags, `-C NAME=VALUE` each.
    pub flags: Vec<String>,
    /// `--edition`, if given: 2021 when not.
    pub edition: Option<String>,
}

impl Default for Rustc {
    fn default() -> Rustc {
        Rustc {
            opt_level: OptLevel::No,
            debug_assertions: None,
            flags: vec![],
            edition: None,
        }
    }
}

impl Rustc {
    /// Take one `-C` option, `NAME=VALUE`, as rustc reads it (a later one of the same name wins).
    pub fn set(&mut self, option: &str) -> Result<(), String> {
        let (name, value) = option.split_once('=').unwrap_or((option, ""));
        match name {
            "opt-level" => self.opt_level = value.parse()?,
            // a bare -C overflow-checks is on, as for rustc; and debug-assertions likewise
            "overflow-checks" | "debug-assertions" => {
                let on = match value {
                    "" | "y" | "yes" | "on" | "true" => true,
                    "n" | "no" | "off" | "false" => false,
                    _ => return Err(format!("-C {name}={value}: give on or off")),
                };
                if name == "debug-assertions" {
                    self.debug_assertions = Some(on);
                }
            }
            _ => {
                return Err(format!(
                    "-C {option}: a single file takes rustc's -C opt-level=N, -C overflow-checks=on|off and -C debug-assertions=on|off"
                ))
            }
        }
        self.flags.extend(["-C".to_string(), option.to_string()]);
        Ok(())
    }

    /// Are debug assertions on: as given, or rustc's default for the opt-level.
    pub fn debug_assertions(&self) -> bool {
        self.debug_assertions.unwrap_or(self.opt_level == OptLevel::No)
    }
}

/// A program's IR: its own, as the driver converted it, the same joined with that of the crates it uses, and the
/// opt-level rustc made its MIR at.
pub struct Mir {
    /// The program's own IR, as `cargo cardiac test` writes it beside the program (`keep_mir`).
    pub program: ir::File,
    pub linked: ir::File,
    pub opt_level: OptLevel,
    /// Whether rustc built it with debug assertions on: core's `assert_unchecked` is then a check, called or not.
    pub debug_assertions: bool,
    /// The enums the driver found for it and its crates.
    pub enums: Vec<EnumDef>,
    /// The structs the driver found for it and its crates (`structs::StructDef`).
    pub structs: Vec<StructDef>,
    /// Whether the compiler cuts main into phases itself (`compile_mir`): the program's `auto_overlay`.
    pub auto_overlay: bool,
}

impl Mir {
    /// The IR of `file`, `program`, joined with that of the crates it uses.
    pub fn link(
        file: &str,
        program: ir::File,
        crates: &[Crate],
        opt_level: OptLevel,
        debug_assertions: bool,
        enums: Vec<EnumDef>,
        structs: Vec<StructDef>,
    ) -> Result<Mir, Diag> {
        let linked = link(&program, crates).map_err(|e| e.at(file))?;
        // a crate's enums are named in the program's MIR with the crate's name first (`kit::Colour`); another
        // crate's enum it uses has its whole path already, and is written down once
        let mut enums = enums;
        for c in crates {
            for e in &c.enums {
                let mut e = e.clone();
                if e.in_crate {
                    e.path.insert(0, c.name.clone());
                    e.in_crate = false;
                }
                if !enums.contains(&e) {
                    enums.push(e);
                }
            }
        }
        let mut structs = structs;
        for s in crates.iter().flat_map(|c| &c.structs) {
            if !structs.contains(s) {
                structs.push(s.clone());
            }
        }
        Ok(Mir {
            program,
            linked,
            opt_level,
            debug_assertions,
            enums,
            structs,
            auto_overlay: false,
        })
    }
}

/// The iterator adapters `explain` names: `core::iter::Map` and the like.
const ADAPTERS: [&str; 8] = ["Map", "Filter", "Zip", "Enumerate", "StepBy", "Chain", "Skip", "Take"];

/// A compile error, said in Rust's terms when the MIR shows a construct CARDIAC has no room for: the compiler's own
/// message names a MIR line or type, which means little to someone who wrote `println!`.
fn explain(mir: &ir::File, e: Diag) -> Diag {
    let uses = Uses::of(mir);
    let print = uses.path(&["io", "_print"]) || uses.path(&["io", "_eprint"]);
    let format =
        uses.path(&["fmt", "format"]) || uses.path(&["fmt", "rt", "Argument"]) || uses.path(&["fmt", "Arguments"]);
    let iterator = uses.path(&["slice", "Iter"])
        || uses.path(&["array", "IntoIter"])
        || uses.path(&["vec", "IntoIter"])
        || ADAPTERS.iter().any(|a| uses.path(&["iter", a]));
    // an array reference made a slice reference
    let slice = uses.unsize;
    let object = uses.object;
    let plain: [(bool, fn(String) -> Diag); 7] = [
        (print, codes::println),
        (format, codes::format),
        (uses.path(&["string", "String"]), codes::string),
        (uses.path(&["vec", "Vec"]), codes::vec),
        (iterator, codes::iterator),
        (object, codes::trait_object),
        (slice, codes::slice),
    ];
    // only for what the compiler does not model: an error with a cause of its own (a constant past what a card
    // holds) is said as it is, whatever else the program uses
    let unmodelled = [
        &codes::RCE0001,
        &codes::RCE0002,
        &codes::RCE0003,
        &codes::RCE0004,
        &codes::RCE0005,
        &codes::RCE0006,
        &codes::RCE0007,
    ]
    .iter()
    .any(|c| e.is(c));
    match plain.iter().find(|(found, _)| *found).filter(|_| unmodelled) {
        Some((_, said)) => said(e.message),
        None => e,
    }
}

/// What a program's MIR names, for `explain`: every path in it, in every item, as its segments' names
/// (`["std", "io", "_print"]`), whether some cast makes an array pointer a slice pointer, and whether one makes a
/// trait object.
#[derive(Default)]
struct Uses {
    paths: Vec<Vec<String>>,
    unsize: bool,
    object: bool,
}

impl Uses {
    fn of(file: &ir::File) -> Uses {
        let mut uses = Uses::default();
        // the visitor is a mutable one; this runs once, for an error, so a copy to walk costs nothing worth saving
        file.clone().visit(&mut uses);
        for item in &file.items {
            match item {
                ir::Item::Other(text) => uses.read(text),
                ir::Item::Const(ir::ConstItem {
                    value: ir::ConstValue::Other(text),
                    ..
                }) => uses.read(text),
                _ => {}
            }
        }
        uses
    }

    /// Whether some path has these names, one after the other: `std::io::_print` has `io`, `_print`.
    fn path(&self, names: &[&str]) -> bool {
        self.paths
            .iter()
            .any(|p| p.windows(names.len()).any(|w| w.iter().zip(names).all(|(a, b)| a == b)))
    }

    /// The names in what the IR keeps as rustc wrote it (a line it does not model, a type, an item it does not
    /// read): each run of words joined by `::` is a path, and an unsizing pointer cast is found by its kind.
    fn read(&mut self, text: &str) {
        let word = |c: char| c.is_alphanumeric() || c == '_';
        let mut path = vec![];
        let mut rest = text;
        while let Some(start) = rest.find(word) {
            let from = &rest[start..];
            let end = from.find(|c: char| !word(c)).unwrap_or(from.len());
            path.push(from[..end].to_string());
            rest = &from[end..];
            // a path goes on past `::` to the next name; anything else ends it
            match rest.strip_prefix("::") {
                Some(next) if next.starts_with(word) => rest = next,
                _ => self.paths.push(std::mem::take(&mut path)),
            }
        }
        if text
            .find("PointerCoercion(")
            .is_some_and(|at| text[at..].starts_with("PointerCoercion(Unsize"))
        {
            if text.contains(" as &dyn ") {
                self.object = true;
            } else {
                self.unsize = true;
            }
        }
    }
}

impl VisitMut for Uses {
    fn visit_body(&mut self, b: &mut ir::Body) {
        // the debug info names values as written
        for d in &b.debug {
            self.read(&d.name);
            self.read(&d.value);
        }
        walk_body(self, b)
    }

    fn visit_statement(&mut self, s: &mut ir::Statement) {
        if let ir::Statement::Unsupported(u) = s {
            self.read(&u.text);
        }
        walk_statement(self, s)
    }

    fn visit_terminator(&mut self, t: &mut ir::Terminator) {
        if let ir::Terminator::Unsupported(u) = t {
            self.read(&u.text);
        }
        walk_terminator(self, t)
    }

    fn visit_rvalue(&mut self, r: &mut ir::Rvalue) {
        match r {
            ir::Rvalue::Cast(_, ir::Ty::Ref { to, .. }, ir::CastKind::Unsize(_)) if matches!(**to, ir::Ty::Dyn(_)) => {
                self.object = true
            }
            ir::Rvalue::Cast(_, _, ir::CastKind::Unsize(_)) => self.unsize = true,
            ir::Rvalue::Cast(_, _, ir::CastKind::Other(text)) => self.read(text),
            _ => {}
        }
        walk_rvalue(self, r)
    }

    fn visit_ty(&mut self, t: &mut ir::Ty) {
        if let ir::Ty::Other(text) = t {
            self.read(text);
        }
        walk_ty(self, t)
    }

    fn visit_path(&mut self, p: &mut ir::Path) {
        // an impl's segment, `<impl [T]>`, is no name: a path's run of names ends there
        let names = p.segments.iter().map(|s| match s {
            ir::Segment::Name { name, .. } => name.clone(),
            ir::Segment::Impl(_) => String::new(),
        });
        self.paths.push(names.collect());
        walk_path(self, p)
    }
}

/// What `cargo cardiac build` writes beside a program (`--emit`, as rustc's): its decks (NAME.cqb, NAME.2.cqb, ...),
/// its listings in Brian Stuart's six columns (NAME.cq, NAME.2.cq, ...), its panic map (NAME.map), and its assembly
/// (NAME.asm, or NAME_data.asm and NAME_1.asm, ...).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Emit {
    pub deck: bool,
    pub listing: bool,
    pub map: bool,
    pub asm: bool,
}

impl Default for Emit {
    /// deck and map: what a program needs to run, and to say why it panicked
    fn default() -> Self {
        Emit {
            deck: true,
            listing: false,
            map: true,
            asm: false,
        }
    }
}

impl Emit {
    /// `deck,listing,map`: the kinds named, and only those.
    pub fn parse(list: &str) -> Result<Emit, String> {
        let mut e = Emit {
            deck: false,
            listing: false,
            map: false,
            asm: false,
        };
        for k in list.split(',').map(str::trim) {
            match k {
                "deck" => e.deck = true,
                "listing" => e.listing = true,
                "map" => e.map = true,
                "asm" => e.asm = true,
                _ => {
                    return Err(format!(
                        "--emit: {k} is not something build writes: deck, listing, map or asm"
                    ))
                }
            }
        }
        Ok(e)
    }

    pub fn names(&self) -> Vec<&'static str> {
        [
            (self.deck, "deck"),
            (self.listing, "listing"),
            (self.map, "map"),
            (self.asm, "asm"),
        ]
        .into_iter()
        .filter(|k| k.0)
        .map(|k| k.1)
        .collect()
    }
}

/// Whether `file` is one that an earlier build of NAME wrote: NAME.asm, NAME_data.asm, NAME_1.asm, ..., NAME.cqb,
/// NAME.2.cqb, ..., NAME.cqb.meta, NAME.cq, NAME.2.cq, ...; and the .tape, .tape.meta and .lst files that builds
/// wrote before decks had the names they have now.
fn earlier_output(file: &str, name: &str) -> bool {
    let Some(rest) = file.strip_prefix(name) else {
        return false;
    };
    let digits = |n: &str| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit());
    if let Some(suffix) = rest.strip_suffix(".asm") {
        return suffix.is_empty() || suffix == "_data" || suffix.strip_prefix('_').is_some_and(digits);
    }
    if rest == ".cqb.meta" || rest == ".tape.meta" {
        return true;
    }
    // .cqb and .cq, and .N.cqb and .N.cq for a later phase
    [".cqb", ".cq", ".tape", ".lst"]
        .iter()
        .filter_map(|ext| rest.strip_suffix(ext))
        .any(|s| s.is_empty() || s.strip_prefix('.').is_some_and(digits))
}

/// What `build` made: the program's size, its deck's file and its panic map's, and the resources whose slots are
/// still empty (no file for them). The driver builds a program and `cargo cardiac` says what it made, so this is
/// what passes between them (`json`, `from_json`); the deck and the map are in their files.
#[derive(Clone, Debug, PartialEq)]
pub struct Built {
    pub file: String,
    /// the cells of its data, and of each phase's code (one phase, most programs)
    pub data: usize,
    pub code: Vec<usize>,
    /// where the cells go, for `build -v`
    pub sections: Sections,
    pub deck_file: String,
    /// its panic map, where its checks are, for saying why a run panicked (beside the deck, NAME.map)
    pub map_file: String,
    pub empty: Vec<String>,
    /// what the programmer might change, as notes and warnings (`Compiled::advice`)
    pub advice: Vec<Diag>,
}

impl Built {
    /// The cells in memory at once: the data and the largest phase's code (CARDIAC holds 96).
    pub fn cells(&self) -> usize {
        self.data + self.code.iter().max().unwrap_or(&0)
    }

    /// The line that reports the program's size.
    pub fn line(&self) -> String {
        match self.code.as_slice() {
            [_] => format!("{}: 1 phase, {} cells", self.file, self.cells()),
            code => format!(
                "{}: {} phases, data {} cells, code {} cells",
                self.file,
                code.len(),
                self.data,
                code.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" / ")
            ),
        }
    }

    pub fn json(&self) -> serde_json::Value {
        let mut v = self.sections.json();
        let more = serde_json::json!({"file": self.file, "data": self.data, "code": self.code, "deck": self.deck_file,
            "map": self.map_file, "empty": self.empty,
            "advice": self.advice.iter().map(Diag::json).collect::<Vec<_>>()});
        if let (Some(v), serde_json::Value::Object(more)) = (v.as_object_mut(), more) {
            v.extend(more);
        }
        v
    }

    pub fn from_json(v: &serde_json::Value) -> Option<Built> {
        let text = |k: &str| v[k].as_str().map(str::to_string);
        let number = |v: &serde_json::Value| v.as_u64().map(|n| n as usize);
        Some(Built {
            file: text("file")?,
            data: number(&v["data"])?,
            code: v["code"].as_array()?.iter().map(number).collect::<Option<_>>()?,
            sections: Sections::from_json(v)?,
            deck_file: text("deck")?,
            map_file: text("map")?,
            empty: v["empty"]
                .as_array()?
                .iter()
                .map(|e| e.as_str().map(str::to_string))
                .collect::<Option<_>>()?,
            advice: v["advice"]
                .as_array()?
                .iter()
                .map(Diag::from_json)
                .collect::<Option<_>>()?,
        })
    }
}

/// Where a program's cells go: its code by function, in the order they come (a shared routine, `rt_div` say, is
/// one too), all phases' together, and its data's: the variables (arrays among them) and the constants.
#[derive(Clone, Debug, PartialEq)]
pub struct Sections {
    pub code: Vec<(String, usize)>,
    pub variables: usize,
    pub constants: usize,
}

impl Sections {
    /// As the driver passes them to `cargo cardiac`: `functions` (each `[name, cells]`), `variables` and
    /// `constants`.
    pub fn json(&self) -> serde_json::Value {
        let code: Vec<serde_json::Value> = self.code.iter().map(|(name, n)| serde_json::json!([name, n])).collect();
        serde_json::json!({"functions": code, "variables": self.variables, "constants": self.constants})
    }

    pub fn from_json(v: &serde_json::Value) -> Option<Sections> {
        let number = |v: &serde_json::Value| v.as_u64().map(|n| n as usize);
        let functions = v["functions"]
            .as_array()?
            .iter()
            .map(|f| Some((f[0].as_str()?.to_string(), number(&f[1])?)));
        Some(Sections {
            code: functions.collect::<Option<_>>()?,
            variables: number(&v["variables"])?,
            constants: number(&v["constants"])?,
        })
    }
}

impl std::fmt::Display for Sections {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let code: Vec<String> = self.code.iter().map(|(name, n)| format!("{name} {n}")).collect();
        write!(
            f,
            "code: {}; data: {} variables, {} constants",
            code.join(", "),
            self.variables,
            self.constants
        )
    }
}

/// A path's directory, `.` for none, as an earlier build's outputs are looked for in it.
fn dir_of(path: &Path) -> &Path {
    match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    }
}

/// Why `build` made nothing: the error; and where the cells go when the program compiled but is too big for
/// CARDIAC's memory, for `build -v` to say what to shrink.
#[derive(Clone, Debug, PartialEq)]
pub struct NotBuilt {
    pub error: Diag,
    pub sections: Option<Sections>,
}

impl From<Diag> for NotBuilt {
    fn from(error: Diag) -> NotBuilt {
        NotBuilt { error, sections: None }
    }
}

/// Compile one program, NAME, from `file`'s IR, and write its assembly, its deck and its panic map next to it.
/// `file` is the source as it is shown (relative to where `cargo cardiac` runs); `source` is where it is from
/// here. The deck's resource slots are filled from the files in `resources` that are there; `Built::empty` names
/// the others.
pub fn build(
    file: &str,
    source: &Path,
    name: &str,
    mir: &Mir,
    resources: &Path,
    emit: &Emit,
) -> Result<Built, NotBuilt> {
    let linked = &mir.linked;
    let compile = || -> Result<_, Diag> {
        let compiled = compile_mir(
            linked,
            mir.opt_level.passes(),
            mir.debug_assertions,
            &mir.enums,
            &mir.structs,
            mir.auto_overlay,
        )
        .map_err(|e| hint(explain(linked, e).at(file), mir.opt_level))?;
        let r = split_phases(&compiled.listing).map_err(|e| e.at(file))?;
        Ok((compiled, r))
    };
    let (mut compiled, mut r) = compile()?;
    // hoisting can cost cells; a program it pushes past memory is compiled again without it
    if super::hoist::enabled() && r.code.iter().any(|c| cells(&r.data) + cells(c) > 96) {
        super::hoist::set_off(true);
        let again = compile();
        super::hoist::set_off(false);
        (compiled, r) = again?;
    }
    // the cells named as the source names them, for the listing and the assembly (the deck is the same)
    compiled.names.rename(&mut r);
    let src_name = Path::new(file).file_name().and_then(|s| s.to_str()).unwrap_or(file);
    let head = format!("; compiled from {src_name} by cargo cardiac build; do not edit\n");
    let dcells = cells(&r.data);
    // clear what an earlier build left (a program can change its number of phases)
    let dir = dir_of(source);
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            if e.file_name().to_str().is_some_and(|n| earlier_output(n, name)) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    // the files, from here; and as they are shown
    let base = dir.join(name).display().to_string();
    let shown = dir_of(Path::new(file)).join(name).display().to_string();
    let shown = shown.strip_prefix("./").unwrap_or(&shown).to_string();
    let write = |p: &str, t: String| std::fs::write(p, t).map_err(|e| Diag::tool(format!("{p}: {e}")));
    if !emit.asm {
    } else if r.code.len() == 1 {
        write(
            &format!("{base}.asm"),
            format!("{head}{}{}", r.code_text(0), r.data_text()),
        )?;
    } else {
        write(&format!("{base}_data.asm"), format!("{head}{}", r.data_text()))?;
        for i in 0..r.code.len() {
            write(&format!("{base}_{}.asm", i + 1), format!("{head}{}", r.code_text(i)))?;
        }
    }
    if let Some(n) = r.code.iter().map(|c| dcells + cells(c)).find(|&n| n > 96) {
        // what makes it big, when the build has a warning of that
        let mut e = codes::too_big(n).at(file);
        for w in compiled.advice.iter().filter(|d| d.kind() == codes::Kind::Warning) {
            e = e.with_note(w.clone().at(file));
        }
        return Err(NotBuilt {
            error: hint(e, mir.opt_level),
            sections: Some(sections(&r)),
        });
    }
    let mut tape = crate::tape::of(&r).map_err(|e| e.at(file))?;
    let empty = tape
        .bind_dir(resources)
        .map_err(|e| Diag::tool(format!("{file}: {e}")))?;
    if emit.deck {
        // a deck for each phase, NAME.cqb, NAME.2.cqb, ..., and the slots in NAME.cqb.meta
        tape.save(Path::new(&format!("{base}.cqb"))).map_err(Diag::tool)?;
    }
    if emit.listing {
        let listings = crate::tape::listings(&r).map_err(|e| e.at(file))?;
        for (i, l) in listings.into_iter().enumerate() {
            let p = crate::tape::Tape::phase_path(Path::new(&format!("{base}.cq")), i + 1);
            write(&p.display().to_string(), l)?;
        }
    }
    let map = crate::panics::map_of(&r).map_err(|e| e.at(file))?;
    if emit.map {
        write(
            &format!("{base}.map"),
            map.text(&format!(
                "{name}.map: the panic map of {name}.cqb, compiled from {src_name}"
            )),
        )?;
    }
    Ok(Built {
        file: file.to_string(),
        data: dcells,
        code: r.code.iter().map(|c| cells(c)).collect(),
        sections: sections(&r),
        deck_file: format!("{shown}.cqb"),
        map_file: format!("{shown}.map"),
        empty,
        advice: compiled.advice,
    })
}

/// Where a program's cells go (`Sections`).
fn sections(r: &super::Phases) -> Sections {
    // main's code comes first; each function's (and routine's) starts at its entry label
    let mut code: Vec<(String, usize)> = vec![("main".to_string(), 0)];
    for phase in &r.code {
        let mut at = 0;
        for line in phase {
            match &line.kind {
                machine::Kind::Label(l) if l.len() > "_entry".len() && l.ends_with("_entry") || l == "rt_panic" => {
                    let name = l.strip_suffix("_entry").unwrap_or(l);
                    at = match code.iter().position(|(c, _)| c == name) {
                        Some(k) => k,
                        None => {
                            code.push((name.to_string(), 0));
                            code.len() - 1
                        }
                    };
                }
                machine::Kind::Instr(..) | machine::Kind::Dat(..) => code[at].1 += 1,
                _ => {}
            }
        }
    }
    // a constant is a cell kN or knN (k999, kn1)
    let constant = |name: &str| {
        let rest = name.strip_prefix("kn").or_else(|| name.strip_prefix('k')).unwrap_or("");
        !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
    };
    let (mut variables, mut constants, mut last) = (0, 0, false);
    for line in &r.data {
        match &line.kind {
            machine::Kind::Dat(Some(name), _) => {
                last = constant(name);
            }
            machine::Kind::Dat(None, _) => {}
            _ => continue,
        }
        if last {
            constants += 1
        } else {
            variables += 1
        }
    }
    Sections {
        code,
        variables,
        constants,
    }
}

/// Write a test program's own IR next to it, printed as MIR, as FILE.mir: `source` is the program's source.
pub fn keep_mir(source: &Path, file: &ir::File) -> Result<(), String> {
    let mir = source.with_extension("mir");
    std::fs::write(&mir, file.to_string()).map_err(|e| format!("{}: {e}", mir.display()))
}

/// Compile a test program as the compiler's tests do: one program, started at main_bb0. Writes FILE.asm next to
/// it (`source`; `file` as it is shown) and returns the assembly.
pub fn compile_test(file: &str, source: &Path, mir: &Mir) -> Result<String, Diag> {
    let asm = compile_mir(
        &mir.linked,
        mir.opt_level.passes(),
        mir.debug_assertions,
        &mir.enums,
        &mir.structs,
        mir.auto_overlay,
    )
    .map_err(|e| hint(explain(&mir.linked, e).at(file), mir.opt_level))?
    .text();
    let out = source.with_extension("asm");
    std::fs::write(&out, &asm).map_err(|e| Diag::tool(format!("{}: {e}", out.display())))?;
    Ok(asm)
}

/// Run a test program's assembly from main_bb0 and return the cards it punched, as `cargo cardiac test` prints
/// them. A program with overlay() calls runs as its tape would: split into phases, one tape each, the first carrying
/// the data.
pub fn run_test(file: &str, asm_text: &str, input: &[i32]) -> Result<String, Diag> {
    let at = |e: Diag| e.at(file);
    let mut tapes = vec![];
    if asm_text.contains(";@@PHASE") {
        let lines = machine::parse(asm_text).map_err(|e| codes::internal(e).at(file))?;
        let r = split_phases(&lines).map_err(at)?;
        let data = r.data_text();
        let data_end = crate::asm::assemble(3, &data).map_err(at)?.end;
        for i in 0..r.code.len() {
            let p = crate::asm::assemble(3, &format!("{data}\n{}", r.code_text(i))).map_err(at)?;
            let cells: Vec<(i32, i32)> = p
                .cells
                .iter()
                .copied()
                .filter(|&(a, _)| i == 0 || a >= data_end)
                .collect();
            let start = if i == 0 {
                "start".to_string()
            } else {
                format!("p{}", i + 1)
            };
            tapes.push(crate::asm::tape(p.label(&start), &cells));
        }
    } else {
        let p = crate::asm::assemble(3, asm_text).map_err(at)?;
        tapes.push(crate::asm::tape(p.label("main_bb0"), &p.cells));
    }
    let mut punched = vec![];
    // the input after the first tape; a later phase's tape is read when the phase before ends, as `Tape::run` does
    crate::cardiac::run_phases(&tapes[0], input, &tapes[1..], &mut punched, 200_000);
    Ok(punched.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(","))
}
