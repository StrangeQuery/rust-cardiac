//! What the MIR text leaves out, which the compiler needs: each enum a crate's functions use, its variants in
//! order, each one's number, which the MIR names but never gives, and the types of its fields (see
//! `compile::enums`); and which of the structs they use have no fields.
use cargo_cardiac::compile::enums::{EnumDef, Variant};
use cargo_cardiac::compile::ir;
use cargo_cardiac::compile::structs::StructDef;
use rustc_middle::ty::print::NoTrimmedGuard;
use rustc_middle::ty::{self as mty, TyCtxt};
use rustc_public::rustc_internal::{internal, stable};
use rustc_public::ty::{AdtDef, GenericArgKind, GenericArgs, IntTy, RigidTy, Ty, TyKind, UintTy, VariantIdx};
use rustc_public::CrateDef;
use rustc_public_bridge::IndexedVal;
use rustc_span::def_id::DefId;

/// The enums that the bodies' locals of `defs` (a crate's items, `convert::items`) have, each as the locals have it,
/// with its type arguments (`Option<i16>`): one of that crate by its path within the crate (`["paint", "Colour"]`),
/// another crate's by its whole path (`["core", "option", "Option"]`). nightly-2026-05-22's rustc_public lists no
/// crate's types.
pub fn enums(tcx: TyCtxt<'_>, defs: &[DefId]) -> Vec<EnumDef> {
    let adts = adts(tcx, defs);
    let Some(&krate) = defs.first().map(|d| &d.krate) else {
        return vec![];
    };
    let mut enums = vec![];
    for (adt, args) in adts.iter().filter(|(a, _)| a.kind().is_enum()) {
        let def: DefId = internal(tcx, adt.def_id());
        let in_crate = def.krate == krate;
        // rustc_public's name is the full path, the crate's name first
        // std's enums are core's (`Option`, which a program with std names `std::option::Option`): written down
        // once, by core's path
        let path: Vec<String> = adt
            .name()
            .split("::")
            .skip(in_crate as usize)
            .enumerate()
            .map(|(i, w)| if i == 0 && !in_crate && w == "std" { "core" } else { w }.to_string())
            .collect();
        let Some(args_ty) = args
            .0
            .iter()
            .filter_map(|a| match a {
                GenericArgKind::Type(t) => Some(ir_ty(tcx, *t)),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let variants = adt
            .variants_iter()
            .enumerate()
            .map(|(i, v)| {
                let d = adt.discriminant_for_variant(VariantIdx::to_val(i));
                Variant {
                    name: v.name(),
                    value: signed(d.val, d.ty),
                    fields: v
                        .fields()
                        .iter()
                        .map(|f| ir_ty(tcx, f.ty_with_args(args)).unwrap_or(ir::Ty::Other(String::new())))
                        .collect(),
                }
            })
            .collect();
        enums.push(EnumDef {
            path,
            args: args_ty,
            in_crate,
            variants,
        });
    }
    enums
}

/// The structs that the bodies' locals of `defs` have, each by its name and whether it has no fields: a unit struct
/// (`struct Flag;`) needs no cells, which the MIR doesn't show where nothing builds one.
pub fn structs(tcx: TyCtxt<'_>, defs: &[DefId]) -> Vec<StructDef> {
    let mut structs: Vec<StructDef> = vec![];
    for (adt, _) in adts(tcx, defs).iter().filter(|(a, _)| a.kind().is_struct()) {
        let s = StructDef {
            name: adt.name().rsplit("::").next().unwrap_or_default().to_string(),
            unit: adt.variants_iter().all(|v| v.fields().is_empty()),
        };
        if !structs.contains(&s) {
            structs.push(s);
        }
    }
    structs
}

/// The ADTs the bodies' locals of `defs` have, each with its type arguments.
fn adts(tcx: TyCtxt<'_>, defs: &[DefId]) -> Vec<(AdtDef, GenericArgs)> {
    let mut adts: Vec<(AdtDef, GenericArgs)> = vec![];
    for &def in defs {
        if tcx.trivial_const(def).is_some() {
            continue;
        }
        let body: rustc_public::mir::Body = stable(tcx.instance_mir(mty::InstanceKind::Item(def)));
        for l in body.locals() {
            adts_in(l.ty, &mut adts);
        }
    }
    adts
}

/// A discriminant's bits as the number they are in its type: an isize's -5 is 2^64 - 5 in a u128.
fn signed(bits: u128, ty: Ty) -> i128 {
    let width = match ty.kind() {
        TyKind::RigidTy(RigidTy::Int(t)) => match t {
            IntTy::I8 => 8,
            IntTy::I16 => 16,
            IntTy::I32 => 32,
            IntTy::I64 | IntTy::Isize => 64,
            IntTy::I128 => 128,
        },
        TyKind::RigidTy(RigidTy::Uint(UintTy::U128)) => 128,
        _ => 128,
    };
    if width < 128 {
        let shift = 128 - width;
        ((bits << shift) as i128) >> shift
    } else {
        bits as i128
    }
}

/// A type as the IR has it, rustc's text with its paths in full, if the parser reads it.
fn ir_ty(tcx: TyCtxt<'_>, t: Ty) -> Option<ir::Ty> {
    let t: mty::Ty<'_> = internal(tcx, t);
    let _full = NoTrimmedGuard::new();
    ir::parse_ty(&t.to_string()).ok()
}

/// The ADTs in a type, itself or within it, each with its type arguments.
fn adts_in(t: Ty, out: &mut Vec<(AdtDef, GenericArgs)>) {
    match t.kind() {
        TyKind::RigidTy(RigidTy::Adt(def, args)) => {
            if !out.iter().any(|(d, a)| *d == def && *a == args) {
                out.push((def, args.clone()));
            }
            for a in args.0 {
                if let GenericArgKind::Type(t) = a {
                    adts_in(t, out);
                }
            }
        }
        TyKind::RigidTy(RigidTy::Ref(_, t, _) | RigidTy::RawPtr(t, _) | RigidTy::Array(t, _) | RigidTy::Slice(t)) => {
            adts_in(t, out)
        }
        TyKind::RigidTy(RigidTy::Tuple(ts)) => ts.into_iter().for_each(|t| adts_in(t, out)),
        _ => {}
    }
}
