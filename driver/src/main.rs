//! The CARDIAC driver: rustc, and the CARDIAC backend. It runs as rustc does (configured as rustc's own driver
//! configures itself, so its MIR, and the names it prints, are the same), and when the crate it has analysed is a
//! program `cargo cardiac` asked for, it compiles the program for CARDIAC in rustc's process (`backend`) and
//! writes what `cargo cardiac` writes: the listing, the tape and the panic map, or for `test` the program's IR
//! printed as MIR and its listing.
//!
//! How settings, outputs and errors flow (`cargo_cardiac::driver` has the two sides' formats):
//! - `CARDIAC_BUILD` (JSON, `driver::Settings`) says `build` or `test`, which programs (by the source file of the
//!   crate's root, each with its name, its source as shown, and its resources directory) and which crates they
//!   use. A crate rustc is given `--cfg cardiac` for, whose root is one of those files, is a program; any other
//!   crate (a program's dependency, a build script) is compiled as rustc compiles it. The opt-level and debug
//!   assertions are the session's: rustc has them from cargo's profile, or a single file's `-C` flags.
//! - A program's crate depends on `CARDIAC_RUN` (an env-dep in its dep-info), new for each run of `cargo
//!   cardiac`, so cargo never takes a program's crate for up to date: its CARDIAC outputs aren't cargo's to track.
//! - The crates a program uses are built with `-Zalways-encode-mir`, so their metadata has every function's MIR;
//!   the driver converts it in the program's session, named as that session names it.
//! - What was made goes back as a line of JSON on standard error (`driver::Report`), which cargo, given rustc's
//!   JSON diagnostics, hands on as one of the program's messages. A program that doesn't compile is also a rustc
//!   error, with the same text as `cargo cardiac`'s message, so rustc, and cargo's build, fail.
//! - For a single file `cargo cardiac` runs the driver itself; rustc writes nothing (the driver stops it once the
//!   program is compiled).
//!
//! With `CARDIAC_IR_CHECK` set, it also checks the IR it converts against the IR the compiler parses from rustc's
//! MIR text (`check`), for the crate it compiles (any crate given `--cfg cardiac`) and, for a program, each crate
//! it uses: `1` reports on standard error, a path appends the report to that file.
#![feature(rustc_private)]

extern crate rustc_abi;
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_public;
extern crate rustc_public_bridge;
extern crate rustc_session;
extern crate rustc_span;

mod backend;
mod check;
mod convert;
mod facts;
mod inline;
mod trimmed;

use cargo_cardiac::compile::program::CFG;
use cargo_cardiac::driver::{Program, Settings, RUN};
use rustc_session::config::Input;

/// rustc, with a look at what it compiled once it has analysed it: as `rustc_public::run!` does, but configured as
/// rustc's own driver configures itself, so its MIR prints the same (trimmed paths: `PartialEq`, not
/// `core::cmp::PartialEq`).
struct Driver {
    /// What `cargo cardiac` asked for, if it runs this driver.
    settings: Option<Settings>,
    /// The program this crate is, if it is one.
    program: Option<Program>,
    /// Whether rustc compiles the crate for CARDIAC (`--cfg cardiac`).
    cardiac: bool,
}

impl rustc_driver::Callbacks for Driver {
    #[allow(rustc::bad_opt_access)]
    fn config(&mut self, config: &mut rustc_interface::interface::Config) {
        config.opts.trimmed_def_paths = true;
        config.override_queries = Some(|_session, providers| trimmed::provide(providers));
        self.cardiac = config.crate_cfg.iter().any(|c| c == CFG);
        if self.cardiac && inline::wanted(&config.opts) {
            // opt-level 1, s or z: rustc's MIR inliner, as at 2, but for the calls the compiler keeps
            inline::enable(&mut config.opts);
            config.override_queries = Some(|_session, providers| {
                trimmed::provide(providers);
                inline::provide(providers);
            });
        } else if self.cardiac {
            // opt-level 2 and 3: rustc's inliner, but for the calls the compiler writes at every level
            config.override_queries = Some(|_session, providers| {
                trimmed::provide(providers);
                inline::provide_written(providers);
            });
        }
        let source = match &config.input {
            Input::File(path) => std::fs::canonicalize(path).ok(),
            Input::Str { .. } => None,
        };
        self.program = self
            .settings
            .iter()
            .flat_map(|s| &s.programs)
            .find(|p| self.cardiac && source.is_some() && std::fs::canonicalize(&p.source).ok() == source)
            .cloned();
        if self.program.is_some() {
            // the crate depends on this run of `cargo cardiac`, so that cargo compiles it every time
            let run = std::env::var(RUN).unwrap_or_default();
            config.track_state = Some(Box::new(move |sess| {
                sess.env_depinfo
                    .borrow_mut()
                    .insert((rustc_span::Symbol::intern(RUN), Some(rustc_span::Symbol::intern(&run))));
            }));
        }
    }

    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        tcx: rustc_middle::ty::TyCtxt<'tcx>,
    ) -> rustc_driver::Compilation {
        let oracle = std::env::var("CARDIAC_IR_CHECK").ok().filter(|v| !v.is_empty());
        match (&self.settings, &self.program) {
            (Some(settings), Some(program)) => {
                backend::compile(tcx, settings, program, oracle.as_deref());
                if settings.single {
                    // a single file: rustc has nothing more to write
                    return rustc_driver::Compilation::Stop;
                }
            }
            _ if self.cardiac => {
                if let Some(to) = &oracle {
                    let report = rustc_public::rustc_internal::run(tcx, || {
                        let label = check::label(tcx);
                        match backend::quietly(|| convert::file(tcx)) {
                            Some((c, renamed)) => check::report(tcx, &c, &renamed, &label),
                            None => format!("ir-check: {label}: the conversion failed\n"),
                        }
                    })
                    .expect("rustc_public");
                    check::write_report(to, &report);
                }
            }
            _ => {}
        }
        rustc_driver::Compilation::Continue
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let settings = match Settings::from_env() {
        Some(Ok(s)) => Some(s),
        Some(Err(e)) => {
            eprintln!("cardiac-driver: {e}");
            std::process::exit(1);
        }
        None => None,
    };
    rustc_driver::run_compiler(
        &args,
        &mut Driver {
            settings,
            program: None,
            cardiac: false,
        },
    );
}
