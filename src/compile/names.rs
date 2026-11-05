//! The listing in the program's own words. Lowering names a local's cell after the function and the local's
//! number in rustc's MIR (`main_9`), and comments a line with the MIR statement it came from
//! (`_9 = Div(copy _8, const 10_i16)`). The MIR's debug info says which variable each local holds
//! (`debug height => _9;`), so the listing can say it the way the source does: the cell is `height`, and the comment
//! reads `height = cell / 10`.
//!
//! The names are worked out once for the whole program (`Names::of`), before lowering: comments are rewritten as each
//! function is lowered (`comment`), and the cells are renamed on the finished program, before it is assembled
//! (`Names::rename`). Only listings and assembly read differently: every cell stays where it was, so the deck is the
//! same.
use std::collections::{HashMap, HashSet};

use super::ir::{self, Local};
use super::lower::Sym;
use super::machine::{Arg, Kind, Line, Op};
use super::Phases;

/// The source's name for each local that has one, function by function, as the listing shows it: `height`, or
/// `main_n` when another function has an `n` too.
#[derive(Clone, Debug, Default)]
pub struct Names {
    /// For each function, each named local's name in the listing.
    shown: HashMap<String, HashMap<Local, String>>,
    /// For each function, the other names the debug info gives a local it already named: `let mut row = horizon;`
    /// makes one local of both, shown as `horizon`, with `row` noted on its cell.
    others: HashMap<String, HashMap<Local, Vec<String>>>,
}

impl Names {
    /// The names for these functions, each given with its debug info in the order the MIR lists it. A variable whose
    /// name is used again further in (shadowing) gets a number, `n` then `n_2`. A name more than one function uses,
    /// or one that reads as a name the compiler makes itself (`k5`, `t0`, `main_bb3`), keeps its function in front.
    ///
    /// `merged` gives the locals the passes made share another's cell (`copies::share_cells`), each the local that went
    /// and the one it went into: a variable's name goes with it, so `let mut n = inp()` is still `n` once rustc's copy
    /// of it and n are one local.
    pub fn of<'a>(functions: impl IntoIterator<Item = (&'a str, &'a [ir::Debug], &'a [(Local, Local)])>) -> Names {
        let mut own: Vec<(String, Vec<(Local, String)>, HashMap<Local, Vec<String>>)> = vec![];
        for (func, debug, merged) in functions {
            let mut named: Vec<(Local, String)> = vec![];
            let mut others: HashMap<Local, Vec<String>> = HashMap::new();
            let mut uses: HashMap<(String, Option<String>), usize> = HashMap::new();
            // each name, numbered when it is used again (a field after its variable's number: `iter_2_0`); a
            // variable's own name before a field's, as `for k in 0..n` gives one local both `k` and the field of
            // rustc's iterator it counts in
            let mut all: Vec<(Local, String, bool)> = vec![];
            for d in debug {
                let Some((var, field, local)) = variable(d) else {
                    continue;
                };
                let n = uses.entry((var.clone(), field.clone())).or_default();
                *n += 1;
                let var = if *n == 1 { var } else { format!("{var}_{n}") };
                let name = match &field {
                    Some(k) => format!("{var}_{k}"),
                    None => var,
                };
                all.push((local, name, field.is_some()));
            }
            for (local, name, field) in all.iter().filter(|x| !x.2).chain(all.iter().filter(|x| x.2)) {
                if !named.iter().any(|(l, _)| l == local) {
                    named.push((*local, name.clone()));
                } else if !field {
                    others.entry(*local).or_default().push(name.clone());
                }
            }
            // a merged local keeps the name of the variable declared first, the one its value was first given
            // (`let height = cell / 10; let below = camera - height;` share height's cell), the other noted on it
            for (gone, into) in merged {
                let Some(at) = named.iter().position(|(l, _)| l == gone) else {
                    continue;
                };
                let (_, name) = named.remove(at);
                let gone_others = others.remove(gone).unwrap_or_default();
                let mut noted = vec![];
                match named.iter().position(|(l, _)| l == into) {
                    Some(j) if j < at => noted.push(name),
                    Some(j) => {
                        let (_, own) = named.remove(j);
                        named.insert(at, (*into, name));
                        noted.push(own);
                    }
                    None => named.insert(at, (*into, name)),
                }
                noted.extend(others.remove(into).unwrap_or_default());
                noted.extend(gone_others);
                if !noted.is_empty() {
                    others.insert(*into, noted);
                }
            }
            own.push((func.to_string(), named, others));
        }
        // how many functions use each name
        let mut functions_using: HashMap<String, usize> = HashMap::new();
        for (_, named, _) in &own {
            let distinct: HashSet<&String> = named.iter().map(|(_, n)| n).collect();
            for n in distinct {
                *functions_using.entry(n.clone()).or_default() += 1;
            }
        }
        let mut names = Names::default();
        for (func, named, others) in own {
            let shown = named
                .into_iter()
                .map(|(l, n)| {
                    let n = if functions_using[&n] > 1 || compilers_own(&n) {
                        format!("{func}_{n}")
                    } else {
                        n
                    };
                    (l, n)
                })
                .collect();
            names.shown.insert(func.clone(), shown);
            names.others.insert(func, others);
        }
        names
    }

    /// The names of `func`'s locals, for its comments.
    pub fn of_function(&self, func: &str) -> HashMap<Local, String> {
        self.shown.get(func).cloned().unwrap_or_default()
    }

    /// Rename the cells of a finished program: each named local's cell (`main_9`, and `main_9_1` for a field of it)
    /// takes its name (`height`, `height_1`), in its label, in every operand, in the constants built on it
    /// (`a_main_9` becomes `a_height`) and in the notes on shared cells (`also main_24`). A name that would be the
    /// same as another symbol's is left as it was, so no two cells ever end up with one name.
    pub fn rename(&self, r: &mut Phases) {
        // every symbol the program has, defined or used
        let mut symbols: Vec<String> = vec![];
        let mut add = |s: &str| {
            if !symbols.iter().any(|x| x == s) {
                symbols.push(s.to_string());
            }
        };
        for line in r.data.iter().chain(r.code.iter().flatten()) {
            match &line.kind {
                Kind::Label(s) | Kind::Table { label: s, .. } | Kind::Dat(Some(s), _) => add(s),
                _ => {}
            }
            if let Kind::Instr(_, Arg::Sym(s, _)) | Kind::Dat(_, Arg::Sym(s, _)) = &line.kind {
                add(s);
            }
        }
        // each local's cell, as lowering names it, and the name it takes
        let mut cells: Vec<(String, String)> = vec![];
        for (func, shown) in &self.shown {
            for (&local, name) in shown {
                cells.push((Sym::local(func, local).to_string(), name.clone()));
            }
        }
        // the longest first, so `main_1` doesn't take the start of `main_12`
        cells.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then(a.cmp(b)));
        let mut new: HashMap<String, String> = HashMap::new();
        for s in &symbols {
            if let Some(n) = renamed(s, &cells) {
                new.insert(s.clone(), n);
            }
        }
        // a name two symbols would share (another renamed cell, or a symbol kept as it is) is given up, until none is
        loop {
            let mut count: HashMap<String, usize> = HashMap::new();
            for s in &symbols {
                *count.entry(new.get(s).unwrap_or(s).clone()).or_default() += 1;
            }
            let clash: Vec<String> = new
                .iter()
                .filter(|(_, n)| count[*n] > 1)
                .map(|(s, _)| s.clone())
                .collect();
            if clash.is_empty() {
                break;
            }
            for s in clash {
                new.remove(&s);
            }
        }
        // the other names a cell holds, noted on it
        let mut notes: HashMap<String, Vec<String>> = HashMap::new();
        for (func, others) in &self.others {
            for (&local, names) in others {
                let cell = Sym::local(func, local).to_string();
                if new.contains_key(&cell) && !names.is_empty() {
                    notes.insert(cell, names.clone());
                }
            }
        }
        let rename = |s: &mut String| {
            if let Some(n) = new.get(s.as_str()) {
                *s = n.clone();
            }
        };
        for line in r.data.iter_mut().chain(r.code.iter_mut().flatten()) {
            if let Kind::Dat(Some(s), _) = &line.kind {
                if let Some(names) = notes.get(s) {
                    let also = format!("also {}", names.join(", "));
                    line.comment = Some(match line.comment.take() {
                        Some(c) => format!("{also}   ; {c}"),
                        None => also,
                    });
                }
            }
            match &mut line.kind {
                Kind::Label(s) | Kind::Table { label: s, .. } => rename(s),
                Kind::Dat(label, arg) => {
                    if let Some(s) = label {
                        rename(s);
                    }
                    if let Arg::Sym(s, _) = arg {
                        rename(s);
                    }
                }
                Kind::Instr(_, Arg::Sym(s, _)) => rename(s),
                _ => {}
            }
            // a note can name a cell that no longer is one, as `also main_24` names the local that shares the cell
            if let Some(c) = &mut line.comment {
                *c = replace_words(c, |w| match symbols.iter().any(|s| s == w) {
                    true => new.get(w).cloned(),
                    false => renamed(w, &cells),
                });
            }
        }
    }
}

/// The variable a line of debug info names and the local holding it: `debug d => _1;` is `d` in `_1`, and
/// `debug ((p: Point).0: i16) => _5;` is field 0 of `p` in `_5`.
fn variable(d: &ir::Debug) -> Option<(String, Option<String>, Local)> {
    if let Some((name, local)) = d.named_local() {
        return Some((name.to_string(), None, local));
    }
    // `((p: Point).0: i16)`: a variable's field
    let local = ir::Local::parse(&d.value).filter(|l| *l != Local::UNIT)?;
    let rest = d.name.strip_prefix("((")?;
    let (var, rest) = rest.split_once(':')?;
    let field = rest.rsplit_once(").")?.1.split_once(':')?.0;
    let plain = |s: &str| !s.is_empty() && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_');
    (plain(var) && !field.is_empty() && field.bytes().all(|c| c.is_ascii_digit()))
        .then(|| (var.to_string(), Some(field.to_string()), local))
}

/// Whether a name reads as one the compiler makes for its own cells and labels, or as a word the assembler knows:
/// such a name keeps its function in front of it.
fn compilers_own(name: &str) -> bool {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit());
    let after = |prefix: &str| name.strip_prefix(prefix).is_some_and(digits);
    // constants, scratch cells, the phases' starts, and a MIR local's own name
    after("k")
        || after("kn")
        || after("t")
        || after("p")
        || after("_")
        || ["a_", "rd_", "wr_", "out_", "tac_", "jmp_", "rt_"]
            .iter()
            .any(|p| name.starts_with(p))
        || name.ends_with("_entry")
        || name.ends_with("_exit")
        || name.rsplit_once("_bb").is_some_and(|(_, n)| digits(n))
        || name == "start"
        || Op::ALL.iter().any(|op| op.mnemonic().eq_ignore_ascii_case(name))
        || ["dat", "data"].iter().any(|w| w.eq_ignore_ascii_case(name))
}

/// The name `symbol` takes, when it is a named local's cell or is built on one: `main_9` is `height`, `main_9_1`
/// `height_1`, `a_main_9` `a_height`, `rd_main_xs_2` `rd_xs_2`.
fn renamed(symbol: &str, cells: &[(String, String)]) -> Option<String> {
    let (prefix, rest) = ["a_", "rd_", "wr_", "out_", "tac_", "jmp_"]
        .iter()
        .find_map(|p| symbol.strip_prefix(p).map(|r| (*p, r)))
        .filter(|(_, r)| cells.iter().any(|(c, _)| r.starts_with(c.as_str())))
        .unwrap_or(("", symbol));
    for (cell, name) in cells {
        let Some(tail) = rest.strip_prefix(cell.as_str()) else {
            continue;
        };
        // the cell itself, or a field or element of it: `_1`, `_1_2`
        let field = tail.is_empty()
            || tail.strip_prefix('_').is_some_and(|t| {
                t.split('_')
                    .all(|k| !k.is_empty() && k.bytes().all(|c| c.is_ascii_digit()))
            });
        if field {
            return Some(format!("{prefix}{name}{tail}"));
        }
    }
    None
}

/// `text` with each word (a run of letters, digits and underscores) that `to` gives a new one for replaced.
fn replace_words(text: &str, to: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::new();
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if !word.is_empty() {
            out.push_str(&to(word).unwrap_or_else(|| word.clone()));
            word.clear();
        }
    };
    for c in text.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

/// A line's comment in Rust's terms: each MIR statement in it (a comment can hold several, `   ; ` between them)
/// with its locals named as the source names them, and the common shapes written as Rust writes them.
/// `_7 = Div(copy _6, const 10_i16)` is `height = cell / 10`, `if Lt(copy _3, const 96_i16)` is `if row < 96`.
/// Anything else is left as the MIR has it, its locals named.
pub fn comment(text: &str, names: &HashMap<Local, String>) -> String {
    text.split("   ; ")
        .map(|part| statement(part, names))
        .collect::<Vec<_>>()
        .join("   ; ")
}

/// One statement of a comment, in Rust's terms.
fn statement(text: &str, names: &HashMap<Local, String>) -> String {
    for (head, negated) in [("if ", false), ("assert !", true), ("assert ", false)] {
        if let Some(cond) = text.strip_prefix(head) {
            let head = if negated { "assert " } else { head };
            if let Some(c) = comparison(cond, negated, names) {
                return format!("{head}{c}");
            }
        }
    }
    if let Some((place, value)) = text.split_once(" = ") {
        if !place.contains(' ') {
            return format!("{} = {}", operand(place, names), value_of(value, names));
        }
    }
    operand(text, names)
}

/// A comparison, `Lt(copy _3, const 96_i16)`, as `row < 96`; or as its opposite, `row >= 96`, when `negated`.
fn comparison(text: &str, negated: bool, names: &HashMap<Local, String>) -> Option<String> {
    let (op, args) = call(text)?;
    let ops = [
        ("Eq", "==", "!="),
        ("Ne", "!=", "=="),
        ("Lt", "<", ">="),
        ("Le", "<=", ">"),
        ("Gt", ">", "<="),
        ("Ge", ">=", "<"),
    ];
    let (_, plain, opposite) = ops.iter().find(|(name, ..)| *name == op)?;
    let [a, b] = args.as_slice() else { return None };
    let symbol = if negated { opposite } else { plain };
    Some(format!("{} {symbol} {}", term(a, names), term(b, names)))
}

/// The value a statement assigns, in Rust's terms where its shape is a common one.
fn value_of(text: &str, names: &HashMap<Local, String>) -> String {
    if let Some((op, args)) = call(text) {
        let infix = [
            ("Add", "+"),
            ("Sub", "-"),
            ("Mul", "*"),
            ("Div", "/"),
            ("Rem", "%"),
            ("UDiv", "/"),
            ("URem", "%"),
            ("AddUnchecked", "+"),
            ("SubUnchecked", "-"),
            ("MulUnchecked", "*"),
            ("BitAnd", "&"),
            ("BitOr", "|"),
            ("BitXor", "^"),
            ("Shl", "<<"),
            ("Shr", ">>"),
        ];
        match (op, args.as_slice()) {
            ("Lin", terms) => {
                if let Some(sum) = sum(terms, names) {
                    return sum;
                }
            }
            (op, [a, b]) if infix.iter().any(|(n, _)| *n == op) => {
                let symbol = infix.iter().find(|(n, _)| *n == op).unwrap().1;
                return format!("{} {symbol} {}", term(a, names), term(b, names));
            }
            (_, [_, _]) => {
                if let Some(c) = comparison(text, false, names) {
                    return c;
                }
            }
            ("Neg", [a]) => return format!("-{}", term(a, names)),
            ("Not", [a]) => return format!("!{}", term(a, names)),
            _ => {}
        }
    }
    // a cast: `copy _3 as usize (IntToInt)`
    if let Some((value, ty)) = text.split_once(" as ") {
        if let Some((ty, _kind)) = ty.split_once(" (") {
            return format!("{} as {ty}", term(value, names));
        }
    }
    operand(text, names)
}

/// The terms of a `Lin` (`+1*copy _1`, `-2*copy _2`) as a sum, `a - 2 * b`.
fn sum(terms: &[&str], names: &HashMap<Local, String>) -> Option<String> {
    let mut out = String::new();
    for (i, t) in terms.iter().enumerate() {
        let (minus, t) = match t.split_at_checked(1)? {
            ("+", t) => (false, t),
            ("-", t) => (true, t),
            _ => return None,
        };
        // a coefficient, when there is one: `3*copy _1`
        let (k, value) = match t.split_once('*') {
            Some((k, v)) if !k.is_empty() && k.bytes().all(|c| c.is_ascii_digit()) => (k, v),
            _ => ("1", t),
        };
        let value = term(value, names);
        let value = if k == "1" { value } else { format!("{k} * {value}") };
        match (i, minus) {
            (0, false) => out += &value,
            (0, true) => out += &format!("-{value}"),
            (_, false) => out += &format!(" + {value}"),
            (_, true) => out += &format!(" - {value}"),
        }
    }
    Some(out)
}

/// An operand inside an expression: in brackets when it has spaces in it, so it reads as one value.
fn term(text: &str, names: &HashMap<Local, String>) -> String {
    let t = operand(text.trim(), names);
    if t.contains(' ') && !t.starts_with('(') && !t.starts_with('[') {
        format!("({t})")
    } else {
        t
    }
}

/// An operand, or any other text, in Rust's terms: `copy` and `move` dropped, a constant's suffix dropped
/// (`const 10_i16` is `10`), and each local the source names named.
fn operand(text: &str, names: &HashMap<Local, String>) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        // `copy`, `move` and `const` only as words of their own, not inside another (`cell_copy`)
        let word_start = !out.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_');
        if let (true, Some(r)) = (
            word_start,
            rest.strip_prefix("copy ").or_else(|| rest.strip_prefix("move ")),
        ) {
            rest = r;
        } else if let (true, Some(r)) = (word_start, rest.strip_prefix("const ")) {
            // the constant's value with its type's suffix dropped: `10_i16`, `-1_i16`, `true`
            let end = r.find([' ', ',', ')', ']']).unwrap_or(r.len());
            out += &constant(&r[..end]);
            rest = &r[end..];
        } else {
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    replace_words(&out, |w| {
        let local = w
            .strip_prefix('_')
            .filter(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))?;
        names.get(&Local(local.parse().ok()?)).cloned()
    })
}

/// A constant without its type: `10_i16` is `10`; anything else stays as it is.
fn constant(k: &str) -> String {
    let int = [
        "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
    ];
    match k.rsplit_once('_') {
        Some((v, ty)) if int.contains(&ty) && v.trim_start_matches('-').parse::<i128>().is_ok() => v.to_string(),
        _ => k.to_string(),
    }
}

/// A call, `Div(copy _6, const 10_i16)`: its name and its arguments, split at the commas between them.
fn call(text: &str) -> Option<(&str, Vec<&str>)> {
    let (name, args) = text.split_once('(')?;
    let args = args.strip_suffix(')')?;
    if name.is_empty() || !name.bytes().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    let mut out = vec![];
    let (mut depth, mut start) = (0i32, 0);
    for (i, c) in args.char_indices() {
        match c {
            '(' | '[' | '<' => depth += 1,
            ')' | ']' | '>' => depth -= 1,
            ',' if depth == 0 => {
                out.push(args[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
        if depth < 0 {
            return None;
        }
    }
    out.push(args[start..].trim());
    Some((name, out))
}

/// Comments as a line keeps them, rewritten for `func` (see `comment`).
pub fn comments(lines: &mut [Line], names: &HashMap<Local, String>) {
    if names.is_empty() {
        return;
    }
    for line in lines {
        if let Some(c) = &mut line.comment {
            *c = comment(c, names);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> HashMap<Local, String> {
        [(1, "d"), (3, "row"), (6, "cell"), (7, "height"), (11, "n")]
            .into_iter()
            .map(|(l, n)| (Local(l), n.to_string()))
            .collect()
    }

    #[test]
    fn comments_read_as_rust() {
        let n = names();
        for (mir, rust) in [
            ("_7 = Div(copy _6, const 10_i16)", "height = cell / 10"),
            ("if Lt(copy _3, const 96_i16)", "if row < 96"),
            ("_11 = Sub(move _12, copy _1)", "n = _12 - d"),
            ("_24 = const 4_i16", "_24 = 4"),
            ("_3 = copy _6", "row = cell"),
            ("_19 = Neg(copy _1)", "_19 = -d"),
            (
                "_5 = Lin(+1*copy _1, -1*copy _3, +2*copy _6)",
                "_5 = d - row + 2 * cell",
            ),
            ("_5 = Lin(-1*copy _1, +1*const 5_i16)", "_5 = -d + 5"),
            ("assert !Eq(copy _1, const 0_i16)", "assert d != 0"),
            ("if Gt(copy _6[_3], const 5_i16)", "if cell[row] > 5"),
            ("_2 = move _7 as usize (IntToInt)", "_2 = height as usize"),
            (
                "_2 = AddWithOverflow(copy _1, const 1_i16)",
                "_2 = AddWithOverflow(d, 1)",
            ),
            ("goto", "goto"),
            ("out(...)", "out(...)"),
            ("also main_3 (never live at once)", "also main_3 (never live at once)"),
            (
                "_11 = Sub(copy _11, copy _1)   ; read through address",
                "n = n - d   ; read through address",
            ),
        ] {
            assert_eq!(comment(mir, &n), rust, "{mir}");
        }
    }

    #[test]
    fn shadowed_and_shared_names() {
        let debug = |pairs: &[(&str, &str)]| -> Vec<ir::Debug> {
            pairs
                .iter()
                .map(|(n, v)| ir::Debug {
                    name: n.to_string(),
                    value: v.to_string(),
                })
                .collect()
        };
        let main = debug(&[
            ("n", "_1"),
            ("x", "_2"),
            ("n", "_3"),
            ("k5", "_4"),
            ("row", "_2"),
            ("((p: Point).1: i16)", "_6"),
            ("((iter: Range<i16>).0: i16)", "_7"),
            ("k", "_7"),
            ("cell", "_8"),
            ("colour", "_10"),
        ]);
        // cell's local goes into an unnamed one, which then goes into colour's: cell is the earlier
        let merged = [(Local(8), Local(9)), (Local(9), Local(10))];
        let add = debug(&[("x", "_1"), ("y", "_2")]);
        let names = Names::of([("main", main.as_slice(), &merged[..]), ("add", add.as_slice(), &[][..])]);
        let main = names.of_function("main");
        let add = names.of_function("add");
        assert_eq!(main[&Local(1)], "n");
        assert_eq!(main[&Local(3)], "n_2");
        assert_eq!(main[&Local(2)], "main_x");
        assert_eq!(add[&Local(1)], "add_x");
        assert_eq!(add[&Local(2)], "y");
        assert_eq!(main[&Local(4)], "main_k5");
        assert_eq!(main[&Local(6)], "p_1");
        assert_eq!(main[&Local(7)], "k");
        assert_eq!(names.others["main"][&Local(2)], vec!["row".to_string()]);
        assert!(!names.others["main"].contains_key(&Local(7)));
        assert_eq!(main[&Local(10)], "cell");
        assert!(!main.contains_key(&Local(8)) && !main.contains_key(&Local(9)));
        assert_eq!(names.others["main"][&Local(10)], vec!["colour".to_string()]);
    }
}
