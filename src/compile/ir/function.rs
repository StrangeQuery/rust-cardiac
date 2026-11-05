//! A function's body as MIR prints it: its locals' declarations, the names its debug info gives them, its scopes,
//! and its basic blocks.
//!
//! ```text
//!     debug x => _2;
//!     let mut _0: i16;
//!     let _2: i16;
//!     scope 1 {
//!         debug y => _3;
//!     }
//!
//!     bb0: {
//!         _2 = copy _1;
//!         return;
//!     }
//! ```
//!
//! A cleanup block (`bb5 (cleanup): {`) only unwinds a panic, which CARDIAC never does: it is not kept.
use super::*;

/// A local's declaration, `let mut _3: i16;`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decl {
    pub local: Local,
    pub mutable: bool,
    pub ty: Ty,
}

/// A name the debug info gives a value, `debug x => _3;`: a variable's name, or a field of one
/// (`((p: Point).0: i16)`), and the local or constant that holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Debug {
    pub name: String,
    pub value: String,
}

impl Debug {
    /// The variable's name and the local holding it, when it is a whole variable in a local.
    pub fn named_local(&self) -> Option<(&str, Local)> {
        let plain = !self.name.is_empty() && self.name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_');
        let l = Local::parse(&self.value).filter(|l| *l != Local::UNIT)?;
        plain.then_some((self.name.as_str(), l))
    }
}

/// A function's body.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Body {
    pub decls: Vec<Decl>,
    pub debug: Vec<Debug>,
    pub blocks: Vec<Block>,
}

/// A function's parameters, `_1: i16, _2: &mut [i16; 4]`: each local and its type, as written.
pub fn parse_params(text: &str) -> Vec<(Local, String)> {
    let mut out = vec![];
    let mut depth = 0i32;
    let mut start = 0;
    let b = text.as_bytes();
    let mut pieces = vec![];
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'[' | b'(' | b'<' => depth += 1,
            b']' | b')' | b'>' => depth -= 1,
            b',' if depth == 0 => {
                pieces.push(&text[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    pieces.push(&text[start..]);
    for p in pieces {
        if let Some((l, t)) = p.split_once(':') {
            if let Some(l) = Local::parse(l.trim()).filter(|l| *l != Local::UNIT) {
                out.push((l, t.trim().to_string()));
            }
        }
    }
    out
}

/// Parse a function's body.
pub fn parse_body(text: &str) -> Body {
    let mut body = Body::default();
    let mut lines = text.lines().map(str::trim).peekable();
    while let Some(line) = lines.next() {
        if let Some(rest) = line.strip_prefix("let ") {
            let rest = rest.strip_suffix(';').unwrap_or(rest);
            let (mutable, rest) = match rest.strip_prefix("mut ") {
                Some(r) => (true, r),
                None => (false, rest),
            };
            if let Some((l, t)) = rest.split_once(": ") {
                if let Some(local) = Local::parse(l).filter(|l| *l != Local::UNIT) {
                    let ty = parse_ty(t.trim()).unwrap_or_else(|_| Ty::Other(t.trim().to_string()));
                    body.decls.push(Decl { local, mutable, ty });
                }
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("debug ") {
            if let Some((name, value)) = rest.strip_suffix(';').and_then(|r| r.rsplit_once(" => ")) {
                body.debug.push(Debug {
                    name: name.to_string(),
                    value: value.to_string(),
                });
            }
            continue;
        }
        let Some(head) = line.strip_suffix(": {") else { continue };
        let cleanup = head.ends_with(" (cleanup)");
        let name = head.strip_suffix(" (cleanup)").unwrap_or(head);
        let Some(n) = name.strip_prefix("bb").and_then(|d| d.parse().ok()) else {
            continue;
        };
        // the block's lines, up to the line that closes it
        let mut text_lines: Vec<&str> = vec![];
        while let Some(l) = lines.peek() {
            if l.starts_with('}') {
                lines.next();
                break;
            }
            text_lines.push(lines.next().unwrap());
        }
        if cleanup {
            continue;
        }
        let kept: Vec<&str> = text_lines
            .into_iter()
            .filter(|l| !l.is_empty() && !l.starts_with("//") && !skipped(l))
            .collect();
        body.blocks.push(block(BasicBlock(n), &kept));
    }
    body
}

/// A statement no pass needs: storage markers, rustc's own bookkeeping, `nop`.
fn skipped(l: &str) -> bool {
    if l == "nop;" {
        return true;
    }
    [
        "StorageLive(",
        "StorageDead(",
        "PlaceMention(",
        "FakeRead(",
        "AscribeUserType(",
        "Retag(",
    ]
    .iter()
    .any(|p| l.starts_with(p))
        && l.ends_with(");")
}

/// A block from its lines: the statements, then the terminator. A line that does not parse is kept as
/// `Unsupported`, to be rejected if the compiler ever needs it.
fn block(name: BasicBlock, lines: &[&str]) -> Block {
    let (last, rest) = lines.split_last().expect("a block without a terminator");
    let statements = rest
        .iter()
        .map(|l| match parse_line(l) {
            Ok(Line::Statement(s)) => s,
            _ => Statement::Unsupported(Unsupported::of(l)),
        })
        .collect();
    let terminator = match parse_line(last) {
        Ok(Line::Terminator(t)) => t,
        _ => Terminator::Unsupported(Unsupported::of(last)),
    };
    Block {
        name,
        statements,
        terminator,
    }
}

// ---- printing, as rustc prints a body (without its scopes, which the IR doesn't keep) ----

impl fmt::Display for Decl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "let {}{}: {};",
            if self.mutable { "mut " } else { "" },
            self.local,
            self.ty
        )
    }
}

impl fmt::Display for Debug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "debug {} => {};", self.name, self.value)
    }
}

impl fmt::Display for Block {
    /// `    bb0: {`, its lines, `    }`: indented as within a body.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "    {}: {{", self.name)?;
        for s in &self.statements {
            writeln!(f, "        {s}")?;
        }
        writeln!(f, "        {}", self.terminator)?;
        writeln!(f, "    }}")
    }
}

impl fmt::Display for Body {
    /// The body's lines between an item's `{` and `}`: its names, its locals, then its blocks, each line ended.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for d in &self.debug {
            writeln!(f, "    {d}")?;
        }
        for d in &self.decls {
            writeln!(f, "    {d}")?;
        }
        for b in &self.blocks {
            writeln!(f)?;
            write!(f, "{b}")?;
        }
        Ok(())
    }
}
