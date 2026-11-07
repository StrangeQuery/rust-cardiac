//! One program built and run one way, on CARDIAC and natively.

use crate::{Outcome, Row, Run};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// What a program's natively built copy reads for inp(): the cards on standard input, one at a time.
const NATIVE_INP: &str = "pub fn inp() -> i16 {
    thread_local! {
        static CARDS: Vec<i16> = std::io::read_to_string(std::io::stdin()).unwrap().split_whitespace().map(|c| c.parse().unwrap()).collect();
        static NEXT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }
    let n = NEXT.with(|n| n.replace(n.get() + 1));
    CARDS.with(|c| c[n])
}";

/// A command's output, given `input` on standard input, or none if it runs longer than `limit`.
fn run_for(cmd: &mut Command, input: &str, limit: Duration) -> Option<Output> {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    child.stdin.take().unwrap().write_all(input.as_bytes()).ok();
    let start = Instant::now();
    loop {
        if child.try_wait().ok()?.is_some() {
            return child.wait_with_output().ok();
        }
        if start.elapsed() > limit {
            child.kill().ok();
            return None;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// The compiler's first error line, without the file's path.
fn first_error(stderr: &str) -> String {
    // a panic: where, and the message on the line after (`panicked at src/cardiac.rs:284:5:` / `a card holds ...`)
    let lines: Vec<&str> = stderr.lines().collect();
    if let Some(i) = lines.iter().position(|l| l.contains("panicked at ")) {
        let at = &lines[i][lines[i].find("panicked at ").unwrap()..];
        let message = lines.get(i + 1).map_or("", |m| m.trim());
        return format!("{at} {message}").trim().chars().take(300).collect();
    }
    let line = stderr
        .lines()
        .find(|l| l.starts_with("error"))
        .or(stderr.lines().find(|l| !l.trim().is_empty()))
        .unwrap_or("(no message)");
    // `error: FILE.rs: why`, or with its code, `error[RCE0004]: FILE.rs: why` (kept as `RCE0004: why`)
    let line = if let Some((code, rest)) = line.strip_prefix("error[").and_then(|l| l.split_once("]: ")) {
        format!("{code}: {}", rest.split_once(".rs: ").map_or(rest, |(_, why)| why))
    } else {
        match line.strip_prefix("error: ").and_then(|l| l.split_once(".rs: ")) {
            Some((_, rest)) => rest.to_string(),
            None => line.to_string(),
        }
    };
    line.chars().take(300).collect()
}

/// The program's output natively, as the compiler's test scripts make it: no #![no_std], out() prints, inp() reads
/// standard input, and a panic is -999 at the end. An error if it doesn't build. Its crate is named as the row's
/// file is, as cargo-cardiac names it (`module_path!()` is the row's name both ways).
#[allow(clippy::too_many_arguments)]
fn native(
    src: &str,
    name: &str,
    checks: &str,
    work: &Path,
    cards: &str,
    edition: &str,
    debug: &str,
) -> Result<String, String> {
    let mut body = src
        .replace("#![no_std]\n", "")
        .replace("pub fn inp() -> i16 { 0 }", NATIVE_INP);
    for ty in ["i16", "u8"] {
        body = body.replace(
            &format!("pub fn out(_v: {ty}) {{}}"),
            &format!("pub fn out(v: {ty}) {{ println!(\"{{}}\", v); }}"),
        );
    }
    let file = work.join("native.rs");
    std::fs::write(&file, body).unwrap();
    let exe = work.join("native");
    let built = Command::new("rustc")
        .args([
            "--crate-name",
            name,
            "--edition",
            edition,
            "-C",
            &format!("overflow-checks={checks}"),
            "-C",
            &format!("debug-assertions={debug}"),
            "-C",
            "opt-level=0",
            "-A",
            "warnings",
        ])
        .arg("-o")
        .args([&exe, &file])
        .output()
        .map_err(|e| e.to_string())?;
    if !built.status.success() {
        return Err(first_error(&String::from_utf8_lossy(&built.stderr)));
    }
    let ran = run_for(&mut Command::new(&exe), cards, Duration::from_secs(60)).ok_or("native run timed out")?;
    let mut out: Vec<String> = String::from_utf8_lossy(&ran.stdout)
        .split_whitespace()
        .map(String::from)
        .collect();
    if !ran.status.success() {
        out.push("-999".into());
    }
    Ok(out.join(","))
}

/// One program built and run with one setting: on CARDIAC (with the cells its build takes), then natively.
pub fn row(cc: &Path, path: &Path, src: &str, row: &Row, opt: &str, checks: &str) -> Run {
    let work = std::env::temp_dir()
        .join("cardiac-support")
        .join(format!("{}-{}", std::process::id(), row.test.replace('/', "_")))
        .join(format!("o{opt}{checks}"));
    std::fs::create_dir_all(&work).unwrap();
    let file = work.join(path.file_name().unwrap());
    std::fs::copy(path, &file).unwrap();
    // the files it reads, where it finds them: beside it
    for f in &row.files {
        std::fs::copy(path.with_file_name(f), work.join(f)).unwrap_or_else(|e| panic!("{}: {f}: {e}", row.test));
    }
    let cards = row.input.clone().unwrap_or_default();
    // 2021, both builds' default, unless the row names another: given only then, so that a cargo-cardiac from
    // before --edition can still be tested
    let edition = row.edition.as_deref().unwrap_or("2021");
    // off, as rustc has them above opt-level 0, unless the row turns them on: the same for both builds
    let debug = row.debug_assertions.as_deref().unwrap_or("off");
    let mut flags = vec![
        "-C".to_string(),
        format!("opt-level={opt}"),
        "-C".into(),
        format!("overflow-checks={checks}"),
        "-C".into(),
        format!("debug-assertions={debug}"),
    ];
    if let Some(e) = &row.edition {
        flags.extend(["--edition".into(), e.clone()]);
    }
    let mut result = cardiac(cc, &file, &flags, &cards, src, checks, edition, debug, &work);
    std::fs::remove_dir_all(&work).ok();
    // a message naming the program by where it was built (a closure's type, `{closure@/var/.../o2on/x.rs:17:15}`)
    // names it by its file alone, so the message is the same from run to run
    if let Some(e) = &mut result.error {
        *e = e.replace(&format!("{}/", work.display()), "");
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn cardiac(
    cc: &Path,
    file: &Path,
    flags: &[String],
    cards: &str,
    src: &str,
    checks: &str,
    edition: &str,
    debug: &str,
    work: &Path,
) -> Run {
    let fail = |outcome, error: String| Run {
        outcome,
        output: None,
        native: None,
        cells: None,
        error: Some(error),
    };
    let limit = Duration::from_secs(300);
    let mut test = Command::new(cc);
    test.args(["cardiac", "test"])
        .args(flags)
        .arg(file)
        .arg("--")
        .args(cards.split_whitespace());
    let Some(t) = run_for(&mut test, "", limit) else {
        return fail(Outcome::Ice, "the build or the run took more than 5 minutes".into());
    };
    let stderr = String::from_utf8_lossy(&t.stderr);
    let stdout = String::from_utf8_lossy(&t.stdout).trim().to_string();
    if stderr.contains("internal compiler error") || stderr.contains("panicked at") {
        return fail(Outcome::Ice, first_error(&stderr));
    }
    if !t.status.success() {
        let outcome = if stderr.contains("runs past cell 98") {
            Outcome::TooBig
        } else {
            Outcome::Refused
        };
        return fail(outcome, first_error(&stderr));
    }
    let mut build = Command::new(cc);
    build.args(["cardiac", "build"]).args(flags).arg(file);
    let cells = run_for(&mut build, "", limit)
        .and_then(|b| cells(&(String::from_utf8_lossy(&b.stderr).into_owned() + &String::from_utf8_lossy(&b.stdout))));
    let name = file.file_stem().unwrap().to_string_lossy();
    match native(src, &name, checks, work, cards, edition, debug) {
        Err(e) => fail(Outcome::NativeFailed, e),
        Ok(want) if want == stdout => Run {
            outcome: Outcome::Match,
            output: Some(stdout),
            native: None,
            cells,
            error: None,
        },
        Ok(want) => Run {
            outcome: Outcome::Differ,
            output: Some(stdout),
            native: Some(want),
            cells,
            error: None,
        },
    }
}

/// The cells a build says the program takes: "…: 1 phase, 9 cells".
fn cells(text: &str) -> Option<u32> {
    let at = text.find(" cells")?;
    text[..at].rsplit(' ').next()?.parse().ok()
}

/// What the page says the results were made with: cargo-cardiac's checkout, as `git describe` names it.
pub fn compiler(cc: &Path) -> String {
    let repo = cc
        .canonicalize()
        .ok()
        .and_then(|p| p.ancestors().nth(3).map(Path::to_path_buf));
    let rev = repo
        .and_then(|r| {
            Command::new("git")
                .arg("-C")
                .arg(r)
                .args(["describe", "--always", "--dirty"])
                .output()
                .ok()
        })
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());
    match rev {
        Some(rev) => format!("cargo-cardiac {rev}"),
        None => format!("{}", cc.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::first_error;

    #[test]
    fn a_crash_gives_where_and_why() {
        let stderr = "\nthread 'main' (19592800) panicked at .../heartburn-0.1.1/src/cardiac.rs:284:5:\na card holds -999..999\nnote: run with `RUST_BACKTRACE=1`\n";
        assert_eq!(
            first_error(stderr),
            "panicked at .../heartburn-0.1.1/src/cardiac.rs:284:5: a card holds -999..999"
        );
    }

    #[test]
    fn a_refusal_keeps_its_code() {
        let stderr = "error[RCE0004]: zz.rs: unsupported local type Pair<i16> for _1\n";
        assert_eq!(first_error(stderr), "RCE0004: unsupported local type Pair<i16> for _1");
    }

    #[test]
    fn a_refusal_gives_its_error() {
        let stderr = "\nerror: zz.rs: unsupported local type Pair<i16> for _1\n";
        assert_eq!(first_error(stderr), "unsupported local type Pair<i16> for _1");
    }
}
