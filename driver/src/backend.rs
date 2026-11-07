//! The CARDIAC backend: a program, once rustc has analysed it, compiled for CARDIAC in rustc's process. Its IR is
//! converted from rustc_public's MIR (`convert`), and that of each crate it uses from the crate's metadata, which
//! has every function's MIR in it (`-Zalways-encode-mir`), named as this session names it, without the crate's
//! name in front, and only those of its impls' functions that a call reaches;
//! they are linked as one (`crates::link`), and compiled. What `cargo cardiac build` or `test` writes is written
//! here, and what was made said to `cargo cardiac` in a line on standard error (`driver::Report`); a program that
//! doesn't compile is a rustc error too, so that rustc fails, and cargo's build with it.
use std::path::Path;

use cargo_cardiac::codes::{self, Diag};
use cargo_cardiac::compile::crates::Crate;
use cargo_cardiac::compile::program::{self, Mir, OptLevel};
use cargo_cardiac::driver::{Mode, Outcome, Program, Report, Settings};
use rustc_middle::ty::TyCtxt;
use rustc_public::CrateDef;
use rustc_span::def_id::DefId;

use crate::{check, convert, facts, instances};

/// Compile `program`, the crate rustc has analysed, as `settings` ask; with `oracle`, check each crate's IR too.
pub fn compile(tcx: TyCtxt<'_>, settings: &Settings, program: &Program, oracle: Option<&str>) {
    let outcome = rustc_public::rustc_internal::run(tcx, || made(tcx, settings, program, oracle))
        .unwrap_or_else(|e| {
            let e = codes::internal(format!("rustc_public: {e:?}")).at(&program.file);
            Outcome::Failed(e, None)
        });
    let report = Report {
        program: program.name.clone(),
        outcome,
    };
    // the report first, then the error it carries: `cargo cardiac` knows the error when it comes
    eprintln!("{}", report.line());
    if let Outcome::Failed(e, _) = &report.outcome {
        tcx.dcx().err(e.to_string());
    }
}

/// What the program is made into, or why it can't be.
fn made(tcx: TyCtxt<'_>, settings: &Settings, program: &Program, oracle: Option<&str>) -> Outcome {
    let file = &program.file;
    // the driver's own failures, not the program's: internal errors, which ask for a bug report
    let at = |e: String| codes::internal(e).at(file);
    let local: Vec<DefId> = rustc_public::all_local_items()
        .into_iter()
        .map(|item| rustc_public::rustc_internal::internal(tcx, item.def_id()))
        .collect();
    let Some(instances) = quietly(|| instances::find(tcx, &local)) else {
        let e = at("the CARDIAC driver couldn't find the program's generic functions' instances".into());
        return Outcome::Failed(e, None);
    };
    let Some((mut own, renamed)) = quietly(|| convert::items_reached(tcx, &local, None, &instances.called)) else {
        return Outcome::Failed(at("the CARDIAC driver couldn't build the IR from rustc's MIR".into()), None);
    };
    if let Some(to) = oracle {
        check::write_report(to, &check::report(tcx, &own, &renamed, &check::label(tcx)));
    }
    // the instances after the program's items, which the oracle has checked against rustc's text
    let Some(made) = quietly(|| convert::bodies(tcx, &instances.bodies)) else {
        let e = at("the CARDIAC driver couldn't build the IR of a generic function's instance".into());
        return Outcome::Failed(e, None);
    };
    own.items.extend(made.items);
    instances.rename_calls(&mut own);
    if !tcx.sess.overflow_checks() {
        cargo_cardiac::compile::front::drop_inherited_overflow_checks(&mut own);
    }
    let bodies: Vec<&rustc_middle::mir::Body<'_>> = instances.bodies.iter().map(|(_, b)| *b).collect();
    let enums = facts::enums(tcx, &local, &bodies);
    let structs = facts::structs(tcx, &local, &bodies);
    if settings.mode == Mode::Test {
        // beside the program before anything else, so it is there when linking fails
        if let Err(e) = program::keep_mir(&program.source, &own) {
            return Outcome::Failed(Diag::tool(e), None);
        }
    }
    // the crates it uses, those rustc loaded for it, in the order `cargo cardiac` gives; of each, an impl's function
    // only if a call reaches it (one that nothing calls, a derived Clone say, needn't compile)
    let loaded: Vec<(&cargo_cardiac::driver::Dep, rustc_span::def_id::CrateNum)> = settings
        .crates
        .iter()
        .filter_map(|dep| {
            let krate = *tcx
                .crates(())
                .iter()
                .find(|&&c| tcx.crate_name(c).as_str() == dep.krate)?;
            Some((dep, krate))
        })
        .collect();
    let mut reached = convert::calls(tcx, &local);
    for (_, krate) in &loaded {
        reached.extend(convert::calls(tcx, &convert::crate_items(tcx, *krate)));
    }
    let in_impl = |def: rustc_span::def_id::DefId| {
        use rustc_hir::def::DefKind;
        tcx.def_kind(def) == DefKind::AssocFn
            && tcx
                .opt_parent(def)
                .is_some_and(|p| matches!(tcx.def_kind(p), DefKind::Impl { .. }))
    };
    let mut crates = vec![];
    for &(dep, krate) in &loaded {
        let mut defs = convert::crate_items(tcx, krate);
        defs.retain(|&d| !in_impl(d) || reached.contains(&d));
        let name = tcx.crate_name(krate).to_string();
        let Some((file, _)) = quietly(|| convert::items(tcx, &defs, Some(&name))) else {
            let e = at(format!(
                "the CARDIAC driver couldn't build the IR of crate {} from its metadata",
                dep.name
            ));
            return Outcome::Failed(e, None);
        };
        if let Some(to) = oracle {
            let label = format!("{} > {}", check::label(tcx), dep.name);
            if let Some((natural, renamed)) = quietly(|| convert::items(tcx, &defs, None)) {
                check::write_report(to, &check::report_items(tcx, &defs, &natural, &renamed, &label));
            }
        }
        let mut file = file;
        if !tcx.sess.overflow_checks() {
            cargo_cardiac::compile::front::drop_inherited_overflow_checks(&mut file);
        }
        crates.push(Crate {
            name: dep.name.clone(),
            enums: facts::enums(tcx, &defs, &[]),
            structs: facts::structs(tcx, &defs, &[]),
            file,
        });
    }
    let opt_level = opt_level(tcx);
    let debug_assertions = tcx.sess.opts.debug_assertions;
    let mut mir = match Mir::link(file, own, &crates, opt_level, debug_assertions, enums, structs) {
        Ok(m) => m,
        Err(e) => return Outcome::Failed(e, None),
    };
    mir.auto_overlay = program.auto_overlay;
    let compiled = loudly(|| match settings.mode {
        Mode::Build => Ok(
            match program::build(
                file,
                &program.source,
                &program.name,
                &mir,
                &program.resources,
                &program.emit,
            ) {
                Ok(b) => Outcome::Built(b),
                Err(n) => Outcome::Failed(n.error, n.sections),
            },
        ),
        Mode::Test => program::compile_test(file, &program.source, &mir).map(|_| Outcome::Tested {
            asm: Path::new(file).with_extension("asm").display().to_string(),
            opt_level,
        }),
    });
    match compiled {
        Some(Ok(o)) => o,
        Some(Err(e)) => Outcome::Failed(e, None),
        None => Outcome::Failed(at("the CARDIAC compiler panicked".into()), None),
    }
}

/// The opt-level rustc made the MIR at, the session's.
fn opt_level(tcx: TyCtxt<'_>) -> OptLevel {
    use rustc_session::config::OptLevel as O;
    match tcx.sess.opts.optimize {
        O::No => OptLevel::No,
        O::Less => OptLevel::Less,
        O::More => OptLevel::More,
        O::Aggressive => OptLevel::Aggressive,
        O::Size => OptLevel::Size,
        O::SizeMin => OptLevel::SizeMin,
    }
}

/// `f`, or None if it panics, saying nothing: the conversion meeting a construct rustc_public can't give, say.
pub fn quietly<T>(f: impl FnOnce() -> T) -> Option<T> {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).ok();
    std::panic::set_hook(hook);
    r
}

/// `f`, or None if it panics, the panic said as Rust says one (`thread 'rustc' panicked at ...`), without the
/// driver's bug report (`main`): the caller fails with an internal error, which `cargo cardiac` follows with it.
fn loudly<T>(f: impl FnOnce() -> T) -> Option<T> {
    // taking the driver's hook leaves Rust's own in its place
    let hook = std::panic::take_hook();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).ok();
    std::panic::set_hook(hook);
    r
}
