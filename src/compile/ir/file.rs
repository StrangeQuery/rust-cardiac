//! A whole MIR file, as `rustc --emit=mir` writes one for a crate: a list of items, each a function, a const, or
//! something the compiler does not read (an allocation's bytes, a static).
//!
//! ```text
//! fn arith::twice(_1: i16) -> i16 {
//!     ...
//! }
//!
//! const LIMIT: i16 = const 40_i16;
//!
//! const SQUARES: [i16; 4] = {
//!     let mut _0: [i16; 4];
//!
//!     bb0: {
//!         _0 = [const 0_i16, const 1_i16, const 4_i16, const 9_i16];
//!         return;
//!     }
//! }
//! ```
//!
//! An item starts at the start of a line, and one with a body ends at the first line that is just `}`. rustc writes
//! some functions twice: a `const fn` is followed by its body for compile-time evaluation, under a
//! `// MIR FOR CTFE` comment. Both are kept, in order.
//!
//! A `File` prints as MIR text that `parse_file` reads back as the same `File` (`Display`): what the IR keeps, in
//! rustc's syntax, without what the parser leaves out (scopes, storage markers, cleanup blocks, comments).
use std::fmt;

use super::visit_mut::VisitMut;
use super::*;

/// A crate's MIR.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct File {
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    Fn(FnItem),
    Const(ConstItem),
    Static(StaticItem),
    Alloc(AllocItem),
    /// An item the compiler does not read, as rustc wrote it.
    Other(String),
}

/// An item's name, as MIR prints it: `main`, `arith::twice`, `<impl at src/lib.rs:4:1: 4:11>::ORIGIN`,
/// `SQUARES::{constant#0}`. Each segment is kept as written.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ItemPath {
    pub segments: Vec<String>,
}

/// `fn name(_1: i16, _2: i16) -> i16 { body }`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FnItem {
    pub path: ItemPath,
    pub params: Vec<(Local, Ty)>,
    pub ret: Ty,
    pub body: Body,
}

/// `const NAME: T = const value;`, or `const NAME: T = { body }` when rustc gives the body that computes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstItem {
    pub path: ItemPath,
    pub ty: Ty,
    pub value: ConstValue,
}

/// `static NAME: T = { body }`, or `static mut`: the body works out its first value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticItem {
    pub path: ItemPath,
    pub ty: Ty,
    pub mutable: bool,
    pub body: Body,
}

/// An allocation's bytes, as rustc writes them after a body that points into it: `alloc1 (static: T, size: 8,
/// align: 2) { 0b 00 16 00 .. }`. Its name and, for a static's, the static; the bytes are kept as text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllocItem {
    pub name: String,
    pub of_static: Option<ItemPath>,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstValue {
    Const(Const),
    Body(Body),
    /// A value the IR does not model (a float, a char), as written.
    Other(String),
}

impl ItemPath {
    /// A path of plain names, `kit::arith::twice`, as opposed to one with an impl or a `{constant#0}` in it.
    pub fn is_plain(&self) -> bool {
        self.segments
            .iter()
            .all(|s| !s.is_empty() && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'))
    }

    /// The path's one segment, when it has only one.
    pub fn single(&self) -> Option<&str> {
        match self.segments.as_slice() {
            [s] => Some(s),
            _ => None,
        }
    }
}

impl fmt::Display for ItemPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.segments.join("::"))
    }
}

impl FnItem {
    /// Whether a parameter's type holds a tuple (other than one of numbers and bools, or a reference to one, or an
    /// `Option` or a `Result` of one) or is `()`, or a function pointer the front end doesn't make a number of, or
    /// the return type is one the IR does not model and that contains a brace. The compiler does not read such a
    /// function (yet). A `()` in a type's arguments (`Option<()>`) takes no cell, as an enum's field.
    pub fn unread(&self) -> bool {
        fn tuple(t: &Ty) -> bool {
            match t {
                Ty::Tuple(_) | Ty::Unit => true,
                Ty::Other(text) => text.contains(')'),
                Ty::Array(t, _) | Ty::Slice(t) | Ty::Ref { to: t, .. } | Ty::Ptr { to: t, .. } => tuple(t),
                Ty::Path(p) | Ty::Dyn(p) => path_tuple(p),
                // (the front end makes a function pointer a number: `indirect::function_pointers`)
                Ty::Int(_) | Ty::Bool | Ty::Char | Ty::Str | Ty::Never | Ty::FnPtr { .. } => false,
            }
        }
        fn path_tuple(p: &Path) -> bool {
            let q = p
                .qself
                .as_ref()
                .is_some_and(|q| tuple(&q.ty) || q.as_trait.as_ref().is_some_and(path_tuple));
            // (an `Option` or a `Result` of a tuple of numbers and bools is read: the enum pass lays one out)
            q || p.segments.iter().any(|s| match s {
                Segment::Name { name, args: Some(a) } => a.args.iter().any(|a| {
                    let laid_out = |t: &Ty| (name == "Option" || name == "Result") && scalars(t);
                    matches!(a, GenericArg::Ty(t) if *t != Ty::Unit && tuple(t) && !laid_out(t))
                }),
                Segment::Impl(t) => tuple(t),
                Segment::Name { args: None, .. } => false,
            })
        }
        // a tuple of numbers, bools and references to them (the `(usize, &i16)` an enumerate's closure takes),
        // `fn join(p: (i16, i16))`, or of such tuples, is read, and a reference to one: the struct pass takes it
        // apart
        fn scalars(t: &Ty) -> bool {
            let cell = |t: &Ty| matches!(t, Ty::Int(_) | Ty::Bool);
            match t {
                Ty::Tuple(ts) => {
                    let element = |t: &Ty| {
                        cell(t)
                            || matches!(t, Ty::Ref { to, .. } if cell(to))
                            || matches!(t, Ty::Tuple(_)) && scalars(t)
                    };
                    ts.len() >= 2 && ts.iter().all(element)
                }
                Ty::Ref { to, .. } => scalars(to),
                _ => false,
            }
        }
        // (and one that holds a function pointer other than as the front end makes a number of it)
        let pointer = |t: &Ty| t.has_fn_pointer() && !t.is_fn_pointers();
        self.params.iter().any(|(_, t)| (tuple(t) && !scalars(t)) || pointer(t))
            || matches!(&self.ret, Ty::Other(t) if t.contains('{'))
    }
}

impl File {
    /// Every body in the file, functions', consts' and statics', in order.
    pub fn bodies(&self) -> impl Iterator<Item = &Body> {
        self.items.iter().filter_map(|i| match i {
            Item::Fn(f) => Some(&f.body),
            Item::Static(s) => Some(&s.body),
            Item::Const(ConstItem {
                value: ConstValue::Body(b),
                ..
            }) => Some(b),
            _ => None,
        })
    }

    pub fn bodies_mut(&mut self) -> impl Iterator<Item = &mut Body> {
        self.items.iter_mut().filter_map(|i| match i {
            Item::Fn(f) => Some(&mut f.body),
            Item::Static(s) => Some(&mut s.body),
            Item::Const(ConstItem {
                value: ConstValue::Body(b),
                ..
            }) => Some(b),
            _ => None,
        })
    }

    /// Run a visitor over every item: each function's parameters, return type and body, each const's type and value.
    pub fn visit(&mut self, v: &mut impl VisitMut) {
        for item in &mut self.items {
            match item {
                Item::Fn(f) => {
                    for (_, t) in &mut f.params {
                        v.visit_ty(t);
                    }
                    v.visit_ty(&mut f.ret);
                    v.visit_body(&mut f.body);
                }
                Item::Const(c) => {
                    v.visit_ty(&mut c.ty);
                    match &mut c.value {
                        ConstValue::Const(k) => v.visit_const(k),
                        ConstValue::Body(b) => v.visit_body(b),
                        ConstValue::Other(_) => {}
                    }
                }
                Item::Static(s) => {
                    v.visit_ty(&mut s.ty);
                    v.visit_body(&mut s.body);
                }
                Item::Alloc(_) | Item::Other(_) => {}
            }
        }
    }
}

/// Parse a MIR file.
pub fn parse_file(text: &str) -> File {
    let lines: Vec<&str> = text.lines().collect();
    let mut items = vec![];
    let mut i = 0;
    while i < lines.len() {
        let head = lines[i];
        i += 1;
        if head.trim().is_empty() || head.starts_with("//") {
            continue;
        }
        // the item's body: the lines up to the one that closes it
        let mut body: Vec<&str> = vec![];
        if head.ends_with('{') {
            while i < lines.len() && lines[i] != "}" {
                body.push(lines[i]);
                i += 1;
            }
            i += 1;
        }
        let body = body.join("\n");
        items.push(item(head, &body));
    }
    File { items }
}

/// One item from its first line and its body's text.
fn item(head: &str, body: &str) -> Item {
    let other = || {
        let mut text = head.to_string();
        if head.ends_with('{') {
            text = format!("{text}\n{body}\n}}");
        }
        Item::Other(text)
    };
    if let Some(rest) = head.strip_prefix("fn ") {
        let Some((path, rest)) = item_path(rest) else {
            return other();
        };
        let Some(rest) = rest.strip_prefix('(') else {
            return other();
        };
        let Some(close) = closing_paren(rest) else {
            return other();
        };
        let params = parse_params(&rest[..close])
            .into_iter()
            .map(|(l, t)| (l, ty_or_other(&t)))
            .collect();
        let Some(ret) = rest[close + 1..]
            .strip_prefix(" -> ")
            .and_then(|r| r.strip_suffix(" {"))
        else {
            return other();
        };
        return Item::Fn(FnItem {
            path,
            params,
            ret: ty_or_other(ret),
            body: parse_body(body),
        });
    }
    if let Some(rest) = head.strip_prefix("static ") {
        let (mutable, rest) = match rest.strip_prefix("mut ") {
            Some(rest) => (true, rest),
            None => (false, rest),
        };
        let Some((path, rest)) = item_path(rest) else {
            return other();
        };
        let Some(ty) = rest.strip_prefix(": ").and_then(|r| r.strip_suffix(" = {")) else {
            return other();
        };
        return Item::Static(StaticItem {
            path,
            ty: ty_or_other(ty),
            mutable,
            body: parse_body(body),
        });
    }
    // `alloc1 (static: T, size: 8, align: 2) {`
    if let Some((name, rest)) = head.split_once(" (") {
        if name.starts_with("alloc") && name[5..].bytes().all(|b| b.is_ascii_digit()) && head.ends_with('{') {
            let of_static = rest
                .strip_prefix("static: ")
                .and_then(|r| r.split_once(", "))
                .and_then(|(path, _)| parse_item_path(path));
            let Item::Other(text) = other() else { unreachable!() };
            return Item::Alloc(AllocItem {
                name: name.to_string(),
                of_static,
                text,
            });
        }
    }
    if let Some(rest) = head.strip_prefix("const ") {
        let Some((path, rest)) = item_path(rest) else {
            return other();
        };
        let Some((ty, value)) = rest.strip_prefix(": ").and_then(|r| r.split_once(" = ")) else {
            return other();
        };
        let value = if value == "{" {
            ConstValue::Body(parse_body(body))
        } else {
            match value.strip_suffix(';').map(parse_operand) {
                Some(Ok(Operand::Constant(c))) => ConstValue::Const(c),
                _ => ConstValue::Other(value.to_string()),
            }
        };
        return Item::Const(ConstItem {
            path,
            ty: ty_or_other(ty),
            value,
        });
    }
    other()
}

fn ty_or_other(text: &str) -> Ty {
    parse_ty(text).unwrap_or_else(|_| Ty::Other(text.to_string()))
}

/// An item's path, the whole text: `<impl at src/lib.rs:4:1: 4:11>::ORIGIN`.
pub fn parse_item_path(text: &str) -> Option<ItemPath> {
    item_path(text).and_then(|(p, rest)| rest.is_empty().then_some(p))
}

/// An item's path at the start of `text`, and the text after it. A segment is a name, or `<...>` or `{...}` as a
/// whole.
fn item_path(text: &str) -> Option<(ItemPath, &str)> {
    let b = text.as_bytes();
    let mut segments = vec![];
    let mut i = 0;
    loop {
        let start = i;
        match b.get(i) {
            Some(open @ (b'<' | b'{')) => {
                let close = if *open == b'<' { b'>' } else { b'}' };
                let mut depth = 0;
                while i < b.len() {
                    if b[i] == *open {
                        depth += 1;
                    } else if b[i] == close {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    i += 1;
                }
                i += 1;
            }
            _ => {
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
            }
        }
        if i == start || i > b.len() {
            return None;
        }
        segments.push(text[start..i].to_string());
        if text[i..].starts_with("::") {
            i += 2;
        } else {
            return Some((ItemPath { segments }, &text[i..]));
        }
    }
}

/// Where the parenthesis closing the one just before `text` is.
fn closing_paren(text: &str) -> Option<usize> {
    let mut depth = 1;
    for (i, c) in text.bytes().enumerate() {
        match c {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

// ---- printing, as `rustc --emit=mir` writes a file: what `parse_file` reads back as the same `File` ----

impl fmt::Display for FnItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "fn {}(", self.path)?;
        for (i, (l, t)) in self.params.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{l}: {t}")?;
        }
        writeln!(f, ") -> {} {{", self.ret)?;
        write!(f, "{}", self.body)?;
        writeln!(f, "}}")
    }
}

impl fmt::Display for ConstItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "const {}: {} = ", self.path, self.ty)?;
        match &self.value {
            ConstValue::Const(c) => writeln!(f, "const {c};"),
            ConstValue::Body(b) => {
                writeln!(f, "{{")?;
                write!(f, "{b}")?;
                writeln!(f, "}}")
            }
            // as written, its `;` and all
            ConstValue::Other(text) => writeln!(f, "{text}"),
        }
    }
}

impl fmt::Display for StaticItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mutable = if self.mutable { "mut " } else { "" };
        writeln!(f, "static {mutable}{}: {} = {{", self.path, self.ty)?;
        write!(f, "{}", self.body)?;
        writeln!(f, "}}")
    }
}

impl fmt::Display for Item {
    /// The item's lines, each ended.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Item::Fn(x) => write!(f, "{x}"),
            Item::Const(x) => write!(f, "{x}"),
            Item::Static(x) => write!(f, "{x}"),
            Item::Alloc(x) => writeln!(f, "{}", x.text),
            Item::Other(text) => writeln!(f, "{text}"),
        }
    }
}

impl fmt::Display for File {
    /// The items, a blank line between each two, as rustc writes them.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{item}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every MIR file the tests and examples have left behind (`cargo cardiac test` keeps a test program's beside
    /// it), or those of the directory `CARDIAC_MIR_CORPUS` names: each prints as MIR that reads back the same.
    #[test]
    fn a_file_prints_as_mir_that_reads_back_the_same() {
        let dirs: Vec<std::path::PathBuf> = match std::env::var_os("CARDIAC_MIR_CORPUS") {
            Some(d) => vec![d.into()],
            None => ["tests/programs", "tests/programs/crates"]
                .iter()
                .map(|d| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(d))
                .collect(),
        };
        let mut files = 0;
        for dir in dirs {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().is_none_or(|x| x != "mir") {
                    continue;
                }
                let text = std::fs::read_to_string(&p).unwrap();
                let parsed = parse_file(&text);
                let again = parse_file(&parsed.to_string());
                for (a, b) in parsed.items.iter().zip(&again.items) {
                    assert_eq!(a, b, "{}: printed as\n{b}", p.display());
                }
                assert_eq!(parsed.items.len(), again.items.len(), "{}", p.display());
                files += 1;
            }
        }
        eprintln!("{files} MIR files printed and read back");
    }

    #[test]
    fn items_print_as_rustc_writes_them() {
        let text = "fn twice(_1: i16) -> i16 {\n    debug x => _1;\n    let mut _0: i16;\n\n    bb0: {\n        \
                    _0 = Add(copy _1, copy _1);\n        return;\n    }\n}\n\nconst N: i16 = const 40_i16;\n";
        assert_eq!(parse_file(text).to_string(), text);
    }
}
