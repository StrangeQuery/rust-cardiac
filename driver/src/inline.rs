//! rustc's MIR inliner at opt-level 1, s and z, where rustc leaves it off (LLVM inlines there instead). The compiler
//! has no MIR of core's to compile a call to it with, so at those levels every call rustc left to a small library
//! function (`Option::and_then`, `u8::checked_add`, `From::from`), a closure or a generic function was refused,
//! where at opt-level 2 the inlined body works. With the inliner on (`enable`) the MIR is opt-level 2's: the same
//! passes, the same inlining, and `#[inline(never)]` keeps a call as it does there.
//!
//! But for the methods of core that stay calls (`provide`): the integer methods the compiler writes itself
//! (`methods::written_here`), which keep the code they had at these levels; and a few whose inlined body the
//! compiler gets wrong today (`WRONG_INLINED`), which stay calls, and refused. A few of the ones it writes stay
//! calls at every level (`provide_written`), where what rustc inlines works on numbers no card holds.
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

/// Integer methods the compiler writes itself (`methods::written_here`) whose inlined body works on numbers no
/// card holds: kept calls at opt-level 2 and 3 too. `ilog10` is bit operations on u32 constants (`393206`);
/// `isqrt` calls core's root of a u16; a u8's `saturating_mul` multiplies two u8s with the overflow flag, which no
/// card holds the product of.
const WRITTEN_AT_EVERY_LEVEL: [(&str, IntTy); 3] = [
    ("ilog10", IntTy::I16),
    ("isqrt", IntTy::I16),
    ("saturating_mul", IntTy::U8),
];

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
        let called = |(name, ty): (&str, IntTy)| methods::written_here(name, ty) || WRONG_INLINED.contains(&(name, ty));
        never_inlined_if(tcx, def_id, called)
    };
}

/// The same at the levels where rustc runs the inliner itself, for `WRITTEN_AT_EVERY_LEVEL` alone.
pub fn provide_written(providers: &mut rustc_middle::util::Providers) {
    let _ = METADATA.set(providers.extern_queries.codegen_fn_attrs);
    providers.extern_queries.codegen_fn_attrs =
        |tcx, def_id| never_inlined_if(tcx, def_id, |m: (&str, IntTy)| WRITTEN_AT_EVERY_LEVEL.contains(&m));
}

/// rustc's codegen attributes of `def_id`, with `#[inline(never)]` where it is an inherent method of `i16`, `u8`
/// or `usize` that `called` says stays a call.
fn never_inlined_if(tcx: TyCtxt<'_>, def_id: DefId, called: impl Fn((&str, IntTy)) -> bool) -> CodegenFnAttrs {
    let mut attrs = METADATA.get().expect("set before any query runs")(tcx, def_id);
    if int_method(tcx, def_id).is_some_and(|(name, ty)| called((name.as_str(), ty))) {
        attrs.inline = InlineAttr::Never;
    }
    attrs
}

/// The name of `def_id` and the integer type, where it is an inherent method of `i16`, `u8` or `usize`.
fn int_method(tcx: TyCtxt<'_>, def_id: DefId) -> Option<(rustc_span::Symbol, IntTy)> {
    if tcx.def_kind(def_id) != DefKind::AssocFn {
        return None;
    }
    let parent = tcx.opt_parent(def_id)?;
    if tcx.def_kind(parent) != (DefKind::Impl { of_trait: false }) {
        return None;
    }
    let ty = match tcx.type_of(parent).instantiate_identity().skip_norm_wip().kind() {
        ty::Int(ty::IntTy::I16) => IntTy::I16,
        ty::Uint(ty::UintTy::U8) => IntTy::U8,
        ty::Uint(ty::UintTy::Usize) => IntTy::Usize,
        _ => return None,
    };
    Some((tcx.item_name(def_id), ty))
}
