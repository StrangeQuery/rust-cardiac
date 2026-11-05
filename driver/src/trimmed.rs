//! Trimmed paths: which names rustc's printer gives as a bare name (`Range` for `core::ops::Range`), the ones
//! that are unique among what a crate can name. rustc works this out once per session, for the crate it is
//! compiling (`rustc_middle::ty::print::trimmed_def_paths`); here it is
//!
//! the session's own (`provide`), the same as rustc's but for its check that some diagnostic is printed: rustc
//! works trimmed paths out only for diagnostics and for `--emit=mir`, and the driver prints names for the IR with
//! neither, which rustc would take for a mistake. It is rustc's algorithm (`for_each_def`, and a name kept if one
//! definition alone has it in its namespace).
use std::collections::HashMap;

use rustc_hir::def::{self, DefKind, Namespace};
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{CrateNum, DefId, DefIdSet};
use rustc_span::Symbol;

/// The session's trimmed paths, as rustc's provider makes them (`trimmed_def_paths`), without its check.
pub fn provide(providers: &mut rustc_middle::util::Providers) {
    providers.queries.trimmed_def_paths = |tcx, ()| of_session(tcx).into_iter().collect();
}

/// The session's trimmed paths (`provide`).
pub fn of_session(tcx: TyCtxt<'_>) -> HashMap<DefId, Symbol> {
    {
        let globbed = tcx.resolutions(()).glob_map.values().flatten().copied();
        let mut local = vec![];
        for id in tcx.hir_free_items() {
            if tcx.def_kind(id.owner_id) == DefKind::Use {
                continue;
            }
            let item = tcx.hir_item(id);
            let Some(ident) = item.kind.ident() else { continue };
            let def_id = item.owner_id.to_def_id();
            local.push((
                ident.name,
                tcx.def_kind(def_id).ns().unwrap_or(Namespace::TypeNS),
                def_id,
            ));
        }
        let direct = tcx
            .crates(())
            .iter()
            .copied()
            .filter(|&c| tcx.extern_crate(c).is_some_and(|e| e.is_direct()));
        unique(tcx, globbed, local, direct)
    }
}

/// rustc's `trimmed_def_paths` over these definitions: the names imported with a `*`, never unique; the crate's
/// own items; and the public items of the crates it uses directly, through their public modules, enums and traits.
fn unique(
    tcx: TyCtxt<'_>,
    globbed: impl Iterator<Item = Symbol>,
    local: Vec<(Symbol, Namespace, DefId)>,
    direct: impl Iterator<Item = CrateNum>,
) -> HashMap<DefId, Symbol> {
    let mut unique: HashMap<(Namespace, Symbol), Option<DefId>> = HashMap::new();
    for symbol in globbed {
        for ns in [Namespace::TypeNS, Namespace::ValueNS, Namespace::MacroNS] {
            unique.insert((ns, symbol), None);
        }
    }
    let mut collect = |name: Symbol, ns: Namespace, def_id: DefId| {
        unique
            .entry((ns, name))
            .and_modify(|v| {
                if v.is_some_and(|existing| existing != def_id) {
                    *v = None;
                }
            })
            .or_insert(Some(def_id));
    };
    for (name, ns, def_id) in local {
        collect(name, ns, def_id);
    }
    let mut queue: Vec<DefId> = direct.map(|c| c.as_def_id()).collect();
    let mut seen = DefIdSet::default();
    while let Some(module) = queue.pop() {
        for child in tcx.module_children(module).iter() {
            if !child.vis.is_public() {
                continue;
            }
            match child.res {
                def::Res::Def(DefKind::AssocTy, _) | def::Res::Def(DefKind::TyAlias, _) => {}
                def::Res::Def(kind, def_id) => {
                    // external #[doc(hidden)] items, and what is within them, are left out
                    if tcx.is_doc_hidden(def_id) {
                        continue;
                    }
                    if let Some(ns) = kind.ns() {
                        collect(child.ident.name, ns, def_id);
                    }
                    if kind.is_module_like() && seen.insert(def_id) {
                        queue.push(def_id);
                    }
                }
                _ => {}
            }
        }
    }
    // one definition known by two names is trimmed to the lexicographically smaller
    let mut map: HashMap<DefId, Symbol> = HashMap::new();
    for ((_, symbol), def_id) in unique {
        let Some(def_id) = def_id else { continue };
        map.entry(def_id)
            .and_modify(|s| {
                if s.as_str() > symbol.as_str() {
                    *s = symbol;
                }
            })
            .or_insert(symbol);
    }
    map
}
