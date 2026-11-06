//! rustc's MIR inliner at opt-level 1, s and z, where rustc leaves it off (LLVM inlines there instead). The compiler
//! has no MIR of core's to compile a call to it with, so at those levels every call rustc left to a small library
//! function (`Option::and_then`, `u8::checked_add`, `From::from`), a closure or a generic function was refused,
//! where at opt-level 2 the inlined body works. With the inliner on (`enable`) the MIR is opt-level 2's: the same
//! passes, the same inlining, and `#[inline(never)]` keeps a call as it does there.
//!
//! But for the methods of core that stay calls (`provide`): the integer methods the compiler writes itself
//! (`methods::written_here`), which keep the code they had at these levels; and a few whose inlined body the
//! compiler gets wrong today (`WRONG_INLINED`), which stay calls, and refused.
use std::sync::OnceLock;

use rustc_hir::attrs::InlineAttr;
use rustc_hir::def::DefKind;
use rustc_middle::middle::codegen_fn_attrs::CodegenFnAttrs;
use rustc_middle::ty::{self, TyCtxt};
use rustc_session::config::{OptLevel, Options};
use rustc_span::def_id::DefId;

use cargo_cardiac::compile::ir::IntTy;
use cargo_cardiac::compile::methods;

/// Integer methods whose inlined body (at opt-level 2) the compiler can't compile right: kept calls, so that at
/// opt-level 1 and s they stay refused as they are at 2. `midpoint` adds in an `i32` past a card, and
/// `saturating_mul` picks between `i16::MIN` and `i16::MAX` as values.
const WRONG_INLINED: [(&str, IntTy); 2] = [("midpoint", IntTy::I16), ("saturating_mul", IntTy::I16)];

/// Whether the inliner should run where rustc wouldn't run it: opt-level 1, s or z, unless `-Zinline-mir` says.
/// (At 0 the MIR is unoptimized and the compiler's passes are off; from 2 rustc runs it already.)
pub fn wanted(opts: &Options) -> bool {
    opts.unstable_opts.inline_mir.is_none()
        && matches!(opts.optimize, OptLevel::Less | OptLevel::Size | OptLevel::SizeMin)
}

/// Turn the inliner on.
pub fn enable(opts: &mut Options) {
    opts.unstable_opts.inline_mir = Some(true);
}

/// rustc's own provider of another crate's codegen attributes, which reads them from its metadata.
static METADATA: OnceLock<for<'tcx> fn(TyCtxt<'tcx>, DefId) -> CodegenFnAttrs> = OnceLock::new();

/// rustc's codegen attributes of another crate's functions, with `#[inline(never)]` on the methods of core that
/// stay calls. Only another crate's are changed: an inherent method of an integer type is core's, and a crate's
/// own attributes go into its metadata, for crates that use it at any opt-level.
pub fn provide(providers: &mut rustc_middle::util::Providers) {
    let _ = METADATA.set(providers.extern_queries.codegen_fn_attrs);
    providers.extern_queries.codegen_fn_attrs = |tcx, def_id| {
        let mut attrs = METADATA.get().expect("set before any query runs")(tcx, def_id);
        if stays_called(tcx, def_id) {
            attrs.inline = InlineAttr::Never;
        }
        attrs
    };
}

/// Whether `def_id` is an inherent method of `i16`, `u8` or `usize` that stays a call.
fn stays_called(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    if tcx.def_kind(def_id) != DefKind::AssocFn {
        return false;
    }
    let Some(parent) = tcx.opt_parent(def_id) else {
        return false;
    };
    if tcx.def_kind(parent) != (DefKind::Impl { of_trait: false }) {
        return false;
    }
    let ty = match tcx.type_of(parent).instantiate_identity().skip_norm_wip().kind() {
        ty::Int(ty::IntTy::I16) => IntTy::I16,
        ty::Uint(ty::UintTy::U8) => IntTy::U8,
        ty::Uint(ty::UintTy::Usize) => IntTy::Usize,
        _ => return false,
    };
    let name = tcx.item_name(def_id);
    methods::written_here(name.as_str(), ty) || WRONG_INLINED.contains(&(name.as_str(), ty))
}
