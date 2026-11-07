//! Cargo's side of `cargo cardiac`: a package's binary targets, which are the programs, and the crates they use,
//! from `cargo metadata`; then the programs built, by `cargo check` with the package's profile and the CARDIAC
//! driver as its rustc (`build`).
use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::codes::{self, Diag};
use crate::compile::program::CFG;
use crate::driver::{Listener, Report, Settings};
use serde_json::Value as Json;

/// A program of a package: its name and root source file.
pub struct Target {
    pub name: String,
    pub path: String,
}

/// A crate a program uses: the name it uses it by, its crate's name (its library target's), and its root source
/// file (its lib.rs).
#[derive(Clone)]
pub struct Extern {
    pub name: String,
    pub krate: String,
    pub path: String,
}

/// One package: its manifest, its programs, and every crate they use (each one after the crates it uses itself),
/// and the workspace's target directory.
///
/// `problem`: why its programs can't be compiled for CARDIAC (a dependency with no library, a proc-macro crate say),
/// said only when one of them is built, so a package that is never built for CARDIAC doesn't fail.
pub struct Package {
    pub name: String,
    pub id: String,
    pub manifest: String,
    pub programs: Vec<Target>,
    pub deps: Vec<Extern>,
    pub problem: Option<Diag>,
    pub target_dir: PathBuf,
    /// The workspace's own Cargo.toml, whose `[profile.dev]` says what `dev_defaults` leaves alone.
    pub workspace_manifest: PathBuf,
    /// `auto-overlay = true` under `[package.metadata.cardiac]`: the compiler splits its programs into phases
    /// itself (off when not given).
    pub auto_overlay: bool,
}

fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".into())
}

/// The package at `manifest` (the one cargo finds from here when None), as cargo would build it: that package
/// alone, or every package of a workspace whose root is not a package itself; or, with `package_name`, that member
/// of the workspace, as `cargo build -p NAME` builds.
pub fn packages(manifest: Option<&str>, package_name: Option<&str>) -> Result<Vec<Package>, String> {
    let cargo = cargo();
    // offline first: a program's package needs nothing from the network (its own dependencies, if it has any, are
    // already in cargo's cache once it has been built), and in cargo-cardiac's own workspace cargo would otherwise
    // fetch the tool's dependencies just to resolve the graph. Online only if that fails.
    let metadata = |offline: bool| -> Result<std::process::Output, String> {
        let mut cmd = Command::new(&cargo);
        cmd.args(["metadata", "--format-version", "1"]);
        if offline {
            cmd.arg("--offline");
        }
        if let Some(m) = manifest {
            cmd.args(["--manifest-path", m]);
        }
        cmd.stdin(Stdio::null())
            .stderr(if offline { Stdio::null() } else { Stdio::inherit() })
            .output()
            .map_err(|e| format!("{cargo} metadata: {e}"))
    };
    let out = match metadata(true)? {
        o if o.status.success() => o,
        _ => metadata(false)?,
    };
    if !out.status.success() {
        return Err("cargo metadata failed".into());
    }
    let meta: Json = serde_json::from_slice(&out.stdout).map_err(|e| format!("cargo metadata: {e}"))?;
    let text = |v: &Json| v.as_str().unwrap_or("").to_string();
    let kinds = |t: &Json| {
        list(&t["kind"])
            .iter()
            .filter_map(|k| k.as_str().map(String::from))
            .collect::<Vec<_>>()
    };
    let mut found = vec![];
    let members = || list(&meta["workspace_members"]).iter().filter_map(|i| i.as_str());
    let ids: Vec<&str> = match package_name {
        // -p NAME: that member of the workspace, wherever in it this is run
        Some(name) => {
            let id = members()
                .find(|id| package(&meta, id).is_some_and(|p| p["name"] == name))
                .ok_or_else(|| format!("no package {name} in the workspace"))?;
            vec![id]
        }
        None => match meta["resolve"]["root"].as_str() {
            Some(root) => vec![root],
            None => members().collect(),
        },
    };
    for id in ids {
        let p = package(&meta, id).ok_or_else(|| format!("cargo metadata: no package {id}"))?;
        let programs = list(&p["targets"])
            .iter()
            .filter(|t| kinds(t).iter().any(|k| k == "bin"))
            .map(|t| Target {
                name: text(&t["name"]),
                path: text(&t["src_path"]),
            })
            .collect();
        // the crates it uses (not those only for its tests or build script), deepest first
        fn visit(
            meta: &Json,
            id: &str,
            as_name: Option<&str>,
            seen: &mut HashSet<String>,
            deps: &mut Vec<Extern>,
            no_library: &mut Option<String>,
        ) -> Result<(), String> {
            if !seen.insert(id.to_string()) {
                return Ok(());
            }
            let n = node(meta, id).ok_or_else(|| format!("cargo metadata: {id} is not in the resolved graph"))?;
            for d in list(&n["deps"]) {
                let normal = list(&d["dep_kinds"]).iter().any(|k| k["kind"].is_null());
                if normal {
                    visit(
                        meta,
                        d["pkg"].as_str().unwrap_or(""),
                        d["name"].as_str(),
                        seen,
                        deps,
                        no_library,
                    )?;
                }
            }
            if let Some(name) = as_name {
                let p = package(meta, id).ok_or_else(|| format!("cargo metadata: no package {id}"))?;
                let Some(lib) = lib_of(meta, id) else {
                    no_library.get_or_insert(p["name"].as_str().unwrap_or(id).to_string());
                    return Ok(());
                };
                deps.push(Extern {
                    name: name.to_string(),
                    krate: crate_name(lib),
                    path: lib["src_path"].as_str().unwrap_or("").to_string(),
                });
            }
            Ok(())
        }
        let (mut deps, mut no_library) = (vec![], None);
        visit(&meta, id, None, &mut HashSet::new(), &mut deps, &mut no_library)?;
        let problem = no_library.map(|krate| codes::no_library(text(&p["name"]), krate));
        found.push(Package {
            name: text(&p["name"]),
            id: id.to_string(),
            manifest: text(&p["manifest_path"]),
            programs,
            deps,
            problem,
            target_dir: PathBuf::from(text(&meta["target_directory"])),
            workspace_manifest: PathBuf::from(text(&meta["workspace_root"])).join("Cargo.toml"),
            auto_overlay: p["metadata"]["cardiac"]["auto-overlay"].as_bool() == Some(true),
        });
    }
    Ok(found)
}

/// What a dev build for CARDIAC takes that cargo's dev profile doesn't give, the settings the examples are built
/// with: rustc's optimized MIR (at opt-level 0 it is unoptimized, and the compiler's passes are off), no incremental
/// compilation (the driver doesn't support it: rustc stops on an internal error), no debuginfo (which changes the
/// MIR, and so the listing), and no overflow checks or debug assertions, whose code a program of 96 cells often
/// has no room for (the voxel example is 87 cells without them, 134 with). `cargo run` keeps cargo's, and checks.
pub const DEV_DEFAULTS: [(&str, &str); 5] = [
    ("opt-level", "2"),
    ("incremental", "false"),
    ("debug", "false"),
    ("overflow-checks", "false"),
    ("debug-assertions", "false"),
];

/// The `DEV_DEFAULTS` a build of `p` takes: with the dev profile (none named, or dev), each one the workspace's
/// Cargo.toml doesn't set itself, in `[profile.dev]` or for this package (`[profile.dev.package.NAME]`, or `."*"`).
/// Cargo's `--config` would win over the Cargo.toml, so what the programmer has chosen is left out; and a setting
/// for the package wins over these anyway, so it isn't said to be one of them.
pub fn dev_defaults(p: &Package, profile: Option<&str>) -> Vec<(&'static str, &'static str)> {
    if profile.is_some_and(|name| name != "dev") {
        return vec![];
    }
    let manifest = std::fs::read_to_string(&p.workspace_manifest).unwrap_or_default();
    let mut set = HashSet::new();
    let mut inside = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            let table = line.trim_start_matches('[').trim_end_matches(']').trim().replace('"', "");
            inside = table == "profile.dev"
                || table == format!("profile.dev.package.{}", p.name)
                || table == "profile.dev.package.*";
        } else if let Some((key, _)) = line.split_once('=') {
            if inside {
                set.insert(key.trim().trim_matches('"').to_string());
            }
        }
    }
    DEV_DEFAULTS.into_iter().filter(|(k, _)| !set.contains(*k)).collect()
}

/// Build these programs of `p` for CARDIAC, as `settings` asks: `cargo check` with the profile named (dev when
/// None), the CARDIAC driver as its rustc. Cargo runs rustc on each crate with the settings its profile resolves
/// to, `[profile.*]` and its `inherits` and per-package overrides included; each crate a program uses keeps its MIR
/// in its metadata (`-Zalways-encode-mir`), and the driver compiles each program from its own MIR and theirs, in
/// rustc's process. The build has a target directory of its own, `cardiac-unknown-none` in the workspace's, since
/// rustc's flags for it differ from a native build's. Like any cargo build it compiles again only what changed,
/// but for the programs themselves, compiled each time (`driver::RUN`).
///
/// Each program's report, in the order given (None for one cargo stopped before), and whether cargo's build
/// succeeded. rustc's diagnostics are rendered on standard error as they come, but for the driver's errors, which
/// are in the reports; with `quiet` (-q), only its errors.
pub fn build(
    p: &Package,
    programs: &[&Target],
    profile: Option<&str>,
    settings: &Settings,
    quiet: bool,
) -> Result<(Vec<Option<Report>>, bool), String> {
    let driver = crate::driver::find()?;
    let mut cmd = Command::new(cargo());
    // cargo's rustc is the driver (it builds the build scripts too, as rustc)
    cmd.env("RUSTC", &driver.exe);
    driver.env(&mut cmd);
    settings.env(&mut cmd);
    cmd.args([
        "check",
        "--quiet",
        "--message-format=json",
        "--manifest-path",
        &p.manifest,
    ]);
    cmd.arg("--target-dir").arg(p.target_dir.join(crate::TARGET));
    if let Some(name) = profile {
        cmd.args(["--profile", name]);
    }
    for (key, value) in dev_defaults(p, profile) {
        cmd.args(["--config", &format!("profile.dev.{key}={value}")]);
    }
    for t in programs {
        cmd.args(["--bin", &t.name]);
    }
    // rustc's flags for every crate of the build, the program's dependencies too: any the environment gives, then
    // ours. Set, they replace a `build.rustflags` of cargo's config, as they would for cargo itself.
    let mut flags: Vec<String> = match std::env::var("CARGO_ENCODED_RUSTFLAGS") {
        Ok(encoded) => encoded
            .split('\x1f')
            .filter(|f| !f.is_empty())
            .map(String::from)
            .collect(),
        Err(_) => std::env::var("RUSTFLAGS")
            .unwrap_or_default()
            .split_whitespace()
            .map(String::from)
            .collect(),
    };
    flags.extend(["--cfg".to_string(), CFG.to_string(), "-Zalways-encode-mir".to_string()]);
    cmd.env("CARGO_ENCODED_RUSTFLAGS", flags.join("\x1f"));
    cmd.env_remove("RUSTFLAGS");
    // cargo's messages on standard output, rustc's among them; cargo's own words on standard error
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("{} check: {e}", cargo()))?;
    let mut listener = Listener::new(quiet);
    for line in BufReader::new(child.stdout.take().unwrap()).lines() {
        let line = line.map_err(|e| format!("{} check: {e}", cargo()))?;
        let Ok(m) = serde_json::from_str::<Json>(&line) else {
            continue;
        };
        if m["reason"] == "compiler-message" {
            listener.take(&m["message"]);
        }
    }
    let status = child.wait().map_err(|e| format!("{} check: {e}", cargo()))?;
    let reports = programs.iter().map(|t| listener.reports.remove(&t.name)).collect();
    Ok((reports, status.success()))
}

/// A package's library target, if it has one.
fn lib_of<'a>(meta: &'a Json, id: &str) -> Option<&'a Json> {
    list(&package(meta, id)?["targets"]).iter().find(|t| {
        list(&t["kind"])
            .iter()
            .any(|k| matches!(k.as_str(), Some("lib" | "rlib")))
    })
}

/// A target's crate name, as rustc knows it: its name with `-` made `_`.
fn crate_name(target: &Json) -> String {
    target["name"].as_str().unwrap_or("").replace('-', "_")
}

fn package<'a>(meta: &'a Json, id: &str) -> Option<&'a Json> {
    list(&meta["packages"]).iter().find(|p| p["id"].as_str() == Some(id))
}

fn node<'a>(meta: &'a Json, id: &str) -> Option<&'a Json> {
    list(&meta["resolve"]["nodes"])
        .iter()
        .find(|n| n["id"].as_str() == Some(id))
}

/// A JSON array's items, or none when the value is missing or not an array.
fn list(v: &Json) -> &[Json] {
    v.as_array().map_or(&[], |a| a.as_slice())
}
