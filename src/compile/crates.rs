//! Programs of more than one crate. The CARDIAC driver converts each crate's MIR to the IR, the program's from
//! rustc's analysis of it and a dependency's from its metadata (built with `-Zalways-encode-mir`), and here a
//! dependency's IR is joined to the program's before the compiler reads any of it. After that a dependency's
//! functions and consts are compiled exactly as the program's own are, and only those `main` can reach end up on
//! the tape.
//!
//! rustc prints names as short as they can be while still being unique, and it decides that separately for each
//! crate: the same function may be `twice` in its own crate's MIR and `kit::arith::twice` at a call in the
//! program's. So a call into another crate is matched to a function there by the end of its path. A dependency's
//! function keeps the name its own crate printed, unless another crate prints the same name, in which case it gets
//! its crate's name in front.
//!
//! The built-ins (`inp`, `out`, ...) are found the same way: a call to `cardiac::inp` reaches `inp` at the root of
//! the built-ins crate, and from then on it is the built-in `inp`, whichever crate defined it.
use std::collections::HashSet;

use super::ir::{File, Item, Segment, Terminator};
use super::Fail;
use super::BUILTINS;

/// One crate's IR, the name other crates know it by, and its enums.
pub struct Crate {
    pub name: String,
    pub file: File,
    /// The enums its functions use, as the driver found them (`compile::enums`).
    pub enums: Vec<super::enums::EnumDef>,
}

/// The functions a crate's MIR defines, by the names it prints for them (not the `<impl at ..>` ones).
fn functions(file: &File) -> HashSet<String> {
    file.items
        .iter()
        .filter_map(|i| match i {
            Item::Fn(f) if f.path.is_plain() => Some(f.path.to_string()),
            _ => None,
        })
        .collect()
}

fn parts(path: &str) -> Vec<&str> {
    path.split("::").collect()
}

fn ends_with(long: &[&str], short: &[&str]) -> bool {
    short.len() <= long.len() && long[long.len() - short.len()..] == *short
}

/// Join the IR of `main` and the crates it uses into one file the compiler can read: the dependencies' items
/// first, then the program's, every call into another crate renamed to the function it reaches.
pub fn link(main: &File, deps: &[Crate]) -> Result<File, Fail> {
    if deps.is_empty() {
        return Ok(main.clone());
    }
    let mut crates: Vec<(&str, File)> = std::iter::once(("this program", main.clone()))
        .chain(deps.iter().map(|c| (c.name.as_str(), c.file.clone())))
        .collect();
    // where a message comes from: nothing for the program itself, else the crate's name
    let names: Vec<&str> = crates.iter().map(|(n, _)| *n).collect();
    let within_crate = |k: usize| {
        if k == 0 {
            String::new()
        } else {
            format!("in crate {}: ", names[k])
        }
    };
    let defined: Vec<HashSet<String>> = crates.iter().map(|(_, file)| functions(file)).collect();
    // the name a crate's function goes by once the crates are joined (a built-in at a crate's root keeps its own)
    let joined = |k: usize, f: &str| -> String {
        if BUILTINS.contains(&f) {
            return f.to_string();
        }
        let clash = k > 0 && defined.iter().enumerate().any(|(j, d)| j != k && d.contains(f));
        if clash {
            format!("{}::{f}", names[k])
        } else {
            f.to_string()
        }
    };
    // a call, as crate k printed it, to the function it reaches (None: not one of these crates', such as core's)
    let resolve = |k: usize, callee: &str| -> Result<Option<String>, Fail> {
        if defined[k].contains(callee) {
            return Ok(Some(joined(k, callee)));
        }
        // the program's session names a crate's own items with the crate's name first (`kit::arith::twice` in kit),
        // where the crate's items are defined without it
        if let Some(own) = callee.strip_prefix(&format!("{}::", names[k])).filter(|_| k > 0) {
            if defined[k].contains(own) {
                return Ok(Some(joined(k, own)));
            }
        }
        if ["core", "std", "alloc"].contains(&parts(callee)[0]) {
            return Ok(None);
        }
        let path = parts(callee);
        let mut found = vec![];
        for (j, d) in defined.iter().enumerate().skip(1).filter(|&(j, _)| j != k) {
            // a path that starts with a crate's name is looked up in that crate only, without the name
            let within: &[&str] = if path.len() > 1 && path[0] == names[j] {
                &path[1..]
            } else if path.len() > 1 && names.iter().skip(1).any(|c| *c == path[0]) {
                continue;
            } else {
                &path
            };
            for f in d {
                let fp = parts(f);
                if ends_with(within, &fp) || ends_with(&fp, within) {
                    found.push((j, f.clone()));
                }
            }
        }
        match found.len() {
            0 => Ok(None),
            1 => Ok(Some(joined(found[0].0, &found[0].1))),
            _ => Err(format!(
                "{}call to {callee} could be any of {}",
                within_crate(k),
                found
                    .iter()
                    .map(|(j, f)| format!("{}::{f}", names[*j]))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    };
    // consts are found by their last name, so two crates may not both define one
    let mut consts: Vec<(String, &str)> = vec![];
    for (name, file) in &crates {
        for item in &file.items {
            let Item::Const(c) = item else { continue };
            let Some(n) = c.path.single().filter(|_| c.path.is_plain()) else {
                continue;
            };
            if let Some((_, other)) = consts.iter().find(|(c, _)| c == n) {
                return Err(format!("const {n} is defined in both {other} and {name}: rename one"));
            }
            consts.push((n.to_string(), *name));
        }
    }
    for (k, (_, file)) in crates.iter_mut().enumerate() {
        for b in file.bodies_mut().flat_map(|b| &mut b.blocks) {
            let Terminator::Call { dest, func, .. } = &mut b.terminator else {
                continue;
            };
            // a call by a path of plain names, the last perhaps with generic arguments, into a local
            if dest.as_local().is_none() || func.qself.is_some() {
                continue;
            }
            let Some((last, before)) = func.segments.split_last() else {
                continue;
            };
            if !before.iter().all(|s| matches!(s, Segment::Name { args: None, .. })) {
                continue;
            }
            let Segment::Name { args: generic, .. } = last else {
                continue;
            };
            let callee = func.to_string();
            let callee = match generic {
                Some(g) => callee.strip_suffix(&g.to_string()).unwrap().to_string(),
                None => callee,
            };
            let own = defined[k].contains(&callee);
            match resolve(k, &callee)? {
                Some(to) if generic.is_some() && !own && !BUILTINS.contains(&to.as_str()) => {
                    return Err(format!("{}call to {callee}{}, a generic function in another crate: rustc writes its MIR \
                        only in the generic form, which this compiler can't use. Make it non-generic, or #[inline] so rustc \
                        inlines it", within_crate(k), generic.as_ref().unwrap()));
                }
                Some(to) => {
                    let args = generic.clone();
                    func.segments = to
                        .split("::")
                        .map(|n| Segment::Name {
                            name: n.to_string(),
                            args: None,
                        })
                        .collect();
                    if let Some(Segment::Name { args: a, .. }) = func.segments.last_mut() {
                        *a = args;
                    }
                }
                None => {}
            }
        }
        if k > 0 {
            for item in &mut file.items {
                if let Item::Fn(f) = item {
                    if f.path.is_plain() {
                        f.path.segments = parts(&joined(k, &f.path.to_string()))
                            .iter()
                            .map(|s| s.to_string())
                            .collect();
                    }
                }
            }
        }
    }
    // the dependencies first, as they would come in one crate that held them as modules
    crates.rotate_left(1);
    Ok(File {
        items: crates.into_iter().flat_map(|(_, f)| f.items).collect(),
    })
}
