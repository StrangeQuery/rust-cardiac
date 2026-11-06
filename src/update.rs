//! Whether a newer cargo-cardiac is out. `cargo install` never looks again at what it installed, so at most once a
//! day cargo-cardiac asks crates.io's index for its newest version, and after a build says when there is one:
//!
//! ```text
//! note: cargo-cardiac 0.1.4 is out (you have 0.1.3): cargo install cargo-cardiac
//! ```
//!
//! The index is fetched in the background, by curl (as the driver is), into .cardiac beside the driver, and what it
//! holds is read the next time: the build never waits for the network. Offline, or on any error, nothing is said.
//! Only the public index is read; nothing about the user or the program is sent. CARGO_CARDIAC_NO_UPDATE_CHECK=1
//! turns it off, and it is off with -q, with --message-format=json, and when standard error isn't a terminal.
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

/// cargo-cardiac's entry in crates.io's sparse index: a line of JSON for each version published.
const INDEX: &str = "https://index.crates.io/ca/rg/cargo-cardiac";

/// How long the last answer is kept before asking again.
const DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// After a build: say so if the index, as last fetched, has a newer version than this one, and fetch it again if
/// it is a day old. `quiet` is -q or --message-format=json.
pub fn note(quiet: bool) {
    let off = std::env::var_os("CARGO_CARDIAC_NO_UPDATE_CHECK").is_some_and(|v| !v.is_empty() && v != "0");
    if quiet || off || !std::io::stderr().is_terminal() {
        return;
    }
    let Some(file) = cache() else { return };
    let age = std::fs::metadata(&file)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok());
    if age.is_none_or(|a| a > DAY) {
        fetch(&file);
    }
    let Some(newest) = std::fs::read_to_string(&file).ok().and_then(|t| newest(&t)) else {
        return;
    };
    let mine = env!("CARGO_PKG_VERSION");
    if version(&newest) > version(mine) {
        eprintln!("note: cargo-cardiac {newest} is out (you have {mine}): {}", how());
    }
}

/// Where the index's answer is kept: .cardiac, beside the driver.
fn cache() -> Option<PathBuf> {
    let home = std::env::var_os("CARDIAC_HOME").map(PathBuf::from).or_else(|| {
        std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(|h| PathBuf::from(h).join(".cardiac"))
    })?;
    std::fs::create_dir_all(&home).ok()?;
    Some(home.join("cargo-cardiac.index"))
}

/// Fetch the index into `file`, in the background: curl outlives this process if it must, and writes the file only
/// when it has an answer (`-f`: not on an HTTP error).
fn fetch(file: &PathBuf) {
    let _ = Command::new("curl")
        .args(["-fsS", "--max-time", "20", "-o"])
        .arg(file)
        .arg(INDEX)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

/// The newest version the index lists that isn't yanked or a pre-release.
fn newest(index: &str) -> Option<String> {
    index
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|v| v["yanked"] != true)
        .filter_map(|v| v["vers"].as_str().map(str::to_string))
        .filter(|v| !v.contains('-'))
        .max_by_key(|v| version(v))
}

/// A version's numbers, to compare: 0.1.10 is after 0.1.9.
fn version(v: &str) -> Vec<u64> {
    v.split('.').map(|n| n.parse().unwrap_or(0)).collect()
}

/// How this copy was installed is how to update it: Homebrew's live in its Cellar.
fn how() -> &'static str {
    let brewed = std::env::current_exe().is_ok_and(|e| e.components().any(|c| c.as_os_str() == "Cellar"));
    if brewed {
        "brew upgrade cargo-cardiac"
    } else {
        "cargo install cargo-cardiac"
    }
}

#[cfg(test)]
mod tests {
    use super::{newest, version};

    #[test]
    fn the_newest_is_neither_yanked_nor_a_prerelease() {
        let index = concat!(
            r#"{"name":"cargo-cardiac","vers":"0.1.9","yanked":false}"#,
            "\n",
            r#"{"name":"cargo-cardiac","vers":"0.1.10","yanked":false}"#,
            "\n",
            r#"{"name":"cargo-cardiac","vers":"0.1.11","yanked":true}"#,
            "\n",
            r#"{"name":"cargo-cardiac","vers":"0.2.0-beta.1","yanked":false}"#,
            "\n",
            r#"{"name":"cargo-cardiac","vers":"#,
        );
        assert_eq!(newest(index).as_deref(), Some("0.1.10"));
        assert!(version("0.1.10") > version("0.1.9"));
    }
}
