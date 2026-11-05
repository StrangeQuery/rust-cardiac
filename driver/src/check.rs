//! The oracle (`CARDIAC_IR_CHECK`): the IR converted from rustc_public (`convert.rs`) against the IR the compiler
//! parses from rustc's MIR text, item by item. The text is rustc's own, made here in memory as `--emit=mir` makes
//! it, by the same function (`rustc_middle::mir::write_mir_pretty`); for the items of a crate a program uses,
//! converted from that crate's metadata, it is rustc's text of the same bodies, written as `write_mir_pretty`
//! writes a crate's (`report_items`). Both files are also printed and read back: each must be the same `File`
//! again.
//!
//! It says, for each item that differs, the first line where the two print differently (or, where they print the
//! same, the first place their structures differ), and a summary line per crate:
//!
//! ```text
//! ir-check: fizzbuzz (fizzbuzz-1234.mir): 9 items, 9 equal, 0 unequal; read back: text ok, converted ok
//! ```
use cargo_cardiac::compile::ir;
use rustc_middle::mir::pretty::MirWriter;
use rustc_middle::ty::{self as mty, TyCtxt};
use rustc_span::def_id::DefId;

/// The crate's name, and its source file's, for the report.
pub fn label(tcx: TyCtxt<'_>) -> String {
    let file = tcx
        .sess
        .local_crate_source_file()
        .and_then(|f| {
            f.local_path()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_default();
    format!("{} ({file})", tcx.crate_name(rustc_span::def_id::LOCAL_CRATE))
}

/// The oracle's report, on standard error (`CARDIAC_IR_CHECK=1`) or appended to a file.
pub fn write_report(to: &str, report: &str) {
    if to == "1" {
        eprint!("{report}");
        return;
    }
    use std::io::Write;
    let file = std::fs::OpenOptions::new().create(true).append(true).open(to);
    if let Err(e) = file.and_then(|mut f| f.write_all(report.as_bytes())) {
        eprintln!("cardiac-driver: {to}: {e}");
    }
}

/// The report on another crate's items (`defs`), converted from its metadata with this session's names
/// (`converted`, no view): against rustc's text of the same bodies, as `write_mir_pretty` writes a crate's.
pub fn report_items(
    tcx: TyCtxt<'_>,
    defs: &[DefId],
    converted: &ir::File,
    renamed: &[(usize, ir::ItemPath)],
    label: &str,
) -> String {
    let writer = MirWriter::new(tcx);
    let mut text = vec![];
    let w: &mut dyn std::io::Write = &mut text;
    for (k, &def) in defs.iter().enumerate() {
        let _ = (|| -> std::io::Result<()> {
            if k > 0 {
                writeln!(w)?;
            }
            let promoteds = |w: &mut dyn std::io::Write| -> std::io::Result<()> {
                for body in tcx.promoted_mir(def) {
                    writeln!(w)?;
                    writer.write_mir_fn(body, w)?;
                }
                Ok(())
            };
            if tcx.is_const_fn(def) {
                writer.write_mir_fn(tcx.optimized_mir(def), w)?;
                promoteds(w)?;
                writeln!(w)?;
                writeln!(w, "// MIR FOR CTFE")?;
                writer.write_mir_fn(tcx.mir_for_ctfe(def), w)?;
            } else if let Some((val, ty)) = tcx.trivial_const(def) {
                let _impls = rustc_middle::ty::print::ForcedImplGuard::new();
                writeln!(
                    w,
                    "const {}: {} = const {};",
                    tcx.def_path_str(def),
                    ty,
                    rustc_middle::mir::Const::Val(val, ty)
                )?;
            } else {
                writer.write_mir_fn(tcx.instance_mir(mty::InstanceKind::Item(def)), w)?;
                promoteds(w)?;
            }
            Ok(())
        })();
    }
    compare(
        &ir::parse_file(&String::from_utf8_lossy(&text)),
        converted,
        renamed,
        label,
    )
}

/// The oracle's report on a crate: the converted IR against the text's. The items `renamed` (each one's index and
/// the text's name for it, `convert::file`) are the conversion's deliberate difference, and are compared under the
/// text's name.
pub fn report(tcx: TyCtxt<'_>, converted: &ir::File, renamed: &[(usize, ir::ItemPath)], label: &str) -> String {
    let mut text = vec![];
    rustc_middle::mir::write_mir_pretty(tcx, &mut text).expect("writing MIR to memory");
    compare(
        &ir::parse_file(&String::from_utf8_lossy(&text)),
        converted,
        renamed,
        label,
    )
}

/// The text's IR (`parsed`) against the converted (`converted_ir`), the items `renamed` compared under the text's
/// names.
fn compare(parsed: &ir::File, converted_ir: &ir::File, renamed: &[(usize, ir::ItemPath)], label: &str) -> String {
    let mut converted = converted_ir.clone();
    for (k, path) in renamed {
        if let Some(ir::Item::Fn(f)) = converted.items.get_mut(*k) {
            f.path = path.clone();
        }
    }
    let converted = &converted;
    let mut out = String::new();
    let (mut equal, mut unequal) = (0, 0);
    for (k, (a, b)) in parsed.items.iter().zip(&converted.items).enumerate() {
        if a == b {
            equal += 1;
            continue;
        }
        unequal += 1;
        out.push_str(&format!("ir-check: {label}: item {k} `{}` differs\n", name(a)));
        out.push_str(&first_difference(a, b));
    }
    let (n, m) = (parsed.items.len(), converted.items.len());
    if n != m {
        out.push_str(&format!(
            "ir-check: {label}: the text has {n} items, the conversion {m}\n"
        ));
        for (k, item) in parsed.items.iter().enumerate().skip(m) {
            out.push_str(&format!(
                "ir-check: {label}: item {k} `{}` is only in the text\n",
                name(item)
            ));
        }
        for (k, item) in converted.items.iter().enumerate().skip(n) {
            out.push_str(&format!(
                "ir-check: {label}: item {k} `{}` is only converted\n",
                name(item)
            ));
        }
    }
    let back = |f: &ir::File| {
        if ir::parse_file(&f.to_string()) == *f {
            "ok"
        } else {
            "DIFFERS"
        }
    };
    out.push_str(&format!(
        "ir-check: {label}: {} items, {equal} equal, {} unequal; read back: text {}, converted {}; {} renamed\n",
        n.max(m),
        unequal + n.abs_diff(m),
        back(&parsed),
        back(converted_ir),
        renamed.len()
    ));
    out
}

/// An item's name, for the report.
fn name(item: &ir::Item) -> String {
    match item {
        ir::Item::Fn(f) => f.path.to_string(),
        ir::Item::Const(c) => c.path.to_string(),
        ir::Item::Other(text) => text.lines().next().unwrap_or("").to_string(),
    }
}

/// Where two items first differ: the first line they print differently, else (they print the same) the first
/// place their structures do.
fn first_difference(text: &ir::Item, converted: &ir::Item) -> String {
    let (a, b) = (text.to_string(), converted.to_string());
    let (la, lb): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
    for k in 0..la.len().max(lb.len()) {
        let (x, y) = (la.get(k).copied(), lb.get(k).copied());
        if x != y {
            return format!(
                "    text:      {}\n    converted: {}\n",
                x.unwrap_or("(nothing)").trim(),
                y.unwrap_or("(nothing)").trim()
            );
        }
    }
    let (da, db) = (format!("{text:?}"), format!("{converted:?}"));
    let at = da.bytes().zip(db.bytes()).take_while(|(x, y)| x == y).count();
    let from = da[..at].char_indices().rev().nth(60).map_or(0, |(i, _)| i);
    let window = |s: &str| s.get(from..(at + 60).min(s.len())).unwrap_or("").to_string();
    format!(
        "    (they print the same)\n    text:      ..{}..\n    converted: ..{}..\n",
        window(&da),
        window(&db)
    )
}
