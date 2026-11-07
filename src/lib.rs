//! cargo-cardiac: Rust for CARDIAC. The compiler (`compile`) takes the MIR that rustc writes and lowers it to CARDIAC
//! assembly, which its own assembler (`asm`) turns into cells; `cargo cardiac build` drives it. CARDIAC's side is
//! heartburn's: the machine (`cardiac`), decks (`tape`, with the deck of a compiled program added
//! here) and listings in Brian Stuart's six columns (`listing`), re-exported under the names they have there. Panic
//! maps, which say why a compiled program panicked, are written here (`panics`) and read by heartburn's burn, which
//! `cargo cardiac run` runs.
//! `cargo` reads a Cargo package for `cargo cardiac`, from what `cargo metadata` prints.
pub use heartburn::{cardiac, listing};
pub mod asm;
pub mod cargo;
pub mod codes;
pub mod compile;
pub mod driver;
pub mod panics;
pub mod tape;
pub mod update;

/// The tool's name: the cargo subcommand is `cargo NAME`, its binary `cargo-NAME`.
pub const TOOL: &str = "cardiac";

/// The version of the `cardiac` crate that `cargo cardiac new` makes a program depend on: the one released with
/// this cargo-cardiac.
pub const CARDIAC: &str = "0.2.3";

/// The notional target triple for CARDIAC: the machine, no vendor, no operating system. It isn't one rustc knows (an
/// official target needs a backend in rustc itself), so there is no `--target` to give: `cargo cardiac` is what
/// builds for it, with the CARDIAC driver. Its builds go in a target directory of this name.
pub const TARGET: &str = "cardiac-unknown-none";
