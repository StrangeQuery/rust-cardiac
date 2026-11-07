//! The machine IR: a CARDIAC assembly listing as typed lines. Lowering emits it, the runtime routines are written
//! in it, the peephole rules and the block layout rewrite it, and the phase splitter and the tape read it. It is
//! printed as CARDIAC assembly at the very end.
//!
//! A line is a label, a resource's table label, an instruction, a data cell, a phase marker, the start of the data,
//! a comment of its own, or blank. An instruction names a symbol (`main_3`, `k999`, `xs+2`) or a cell by number
//! (`00`, `99`). A line keeps no spacing: it prints to a fixed layout, a data cell's `DAT` in the data column and
//! any comment in the comment column. Comments are for the reader only: no pass reads them.
use std::fmt;

/// The ten instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Op {
    Inp,
    Cla,
    Add,
    Tac,
    Sft,
    Out,
    Sto,
    Sub,
    Jmp,
    Hrs,
}

impl Op {
    pub const ALL: [Op; 10] = [
        Op::Inp,
        Op::Cla,
        Op::Add,
        Op::Tac,
        Op::Sft,
        Op::Out,
        Op::Sto,
        Op::Sub,
        Op::Jmp,
        Op::Hrs,
    ];

    pub fn mnemonic(self) -> &'static str {
        match self {
            Op::Inp => "INP",
            Op::Cla => "CLA",
            Op::Add => "ADD",
            Op::Tac => "TAC",
            Op::Sft => "SFT",
            Op::Out => "OUT",
            Op::Sto => "STO",
            Op::Sub => "SUB",
            Op::Jmp => "JMP",
            Op::Hrs => "HRS",
        }
    }

    pub fn from_mnemonic(s: &str) -> Option<Op> {
        Op::ALL.into_iter().find(|o| o.mnemonic() == s)
    }
}

/// An operand: a number as written (`00`, `-999`), or a symbol and an offset from it (`xs+2`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Arg {
    Num(String),
    Sym(String, Option<u32>),
}

impl Arg {
    pub fn sym(name: &str) -> Arg {
        Arg::Sym(name.to_string(), None)
    }

    /// An operand as written: `00`, `main_3`, `xs+2`.
    pub fn of(text: &str) -> Arg {
        parse_arg(text).unwrap_or_else(|| panic!("not an operand: {text}"))
    }

    /// The symbol it names, offset aside.
    pub fn symbol(&self) -> Option<&str> {
        match self {
            Arg::Sym(s, _) => Some(s),
            Arg::Num(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Blank,
    /// `name:`
    Label(String),
    /// `label:` in the data, ahead of the first cell of a resource's array, which the loader fills. It names the
    /// resource for the tape; it prints as a label does, and reads back as a plain one.
    Table {
        label: String,
        resource: String,
    },
    /// `        CLA main_3`
    Instr(Op, Arg),
    /// `name:    DAT 0`, a labelled cell; or `          DAT 0`, a cell after the one labelled.
    Dat(Option<String>, Arg),
    /// `;@@PHASE label`: where a program's next phase starts (the loader reads it over this one).
    Phase(String),
    /// `; ---- locals and constants ----`: where the code ends and the data block starts.
    DataStart,
    /// A comment on a line of its own: what follows the `;`.
    Note(String),
}

/// How the start of the data block is written: a heading, which the parser reads back as `Kind::DataStart`.
const DATA_START: &str = "; ---- locals and constants ----";

#[derive(Clone, Debug)]
pub struct Line {
    pub kind: Kind,
    /// What follows the `;` at the end of the line, less the spaces it starts with.
    pub comment: Option<String>,
    /// On a branch to `rt_panic`: the check that failed when it is taken, as Rust says it ("index out of bounds"),
    /// for the panic map (`panics`). Never printed, and no part of what the line is: two lines that differ only
    /// here are the same line.
    pub panic: Option<String>,
    /// On a STO: the accumulator may hold a value past a card (a sum or product without overflow checks), so the
    /// cell keeps less of it than the accumulator holds, and the accumulator is no stand-in for the cell after it.
    /// Never printed, as `panic`.
    pub wide: bool,
}

impl PartialEq for Line {
    fn eq(&self, other: &Line) -> bool {
        self.kind == other.kind && self.comment == other.comment
    }
}

impl Eq for Line {}

impl std::hash::Hash for Line {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.kind.hash(h);
        self.comment.hash(h);
    }
}

/// Where a comment starts: past an instruction's mnemonic and operand padded to this width, and one space; or one
/// space past a line that is longer. A comment after any other line starts there too.
pub const COMMENT_COLUMN: usize = 14;

/// Where a data cell's `DAT` starts: past its label padded to this width, and one space; or one space past a label
/// that is longer.
pub const DATA_COLUMN: usize = 9;

impl Line {
    pub fn new(kind: Kind) -> Line {
        Line {
            kind,
            comment: None,
            panic: None,
            wide: false,
        }
    }

    pub fn instr(op: Op, arg: Arg) -> Line {
        Line::new(Kind::Instr(op, arg))
    }

    pub fn label(name: &str) -> Line {
        Line::new(Kind::Label(name.to_string()))
    }

    /// An instruction with a comment.
    pub fn commented(op: Op, arg: Arg, text: &str) -> Line {
        Line {
            kind: Kind::Instr(op, arg),
            comment: Some(text.to_string()),
            panic: None,
            wide: false,
        }
    }

    /// A data cell, `name: DAT value`.
    pub fn data_cell(name: &str, value: Arg) -> Line {
        Line::new(Kind::Dat(Some(name.to_string()), value))
    }

    /// Whether it goes to `rt_panic` when it is taken.
    pub fn to_panic(&self) -> bool {
        matches!(&self.kind, Kind::Instr(Op::Jmp | Op::Tac, Arg::Sym(s, None)) if s == "rt_panic")
    }

    /// The same line with `text` said after what it already says: `   ; text`.
    pub fn note(&mut self, text: &str) {
        match &mut self.comment {
            Some(c) => *c += &format!("   ; {text}"),
            None => self.comment = Some(text.to_string()),
        }
    }

    /// The instruction, when the line is one.
    pub fn as_instr(&self) -> Option<(Op, &Arg)> {
        match &self.kind {
            Kind::Instr(op, arg) => Some((*op, arg)),
            _ => None,
        }
    }
}

impl fmt::Display for Arg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Arg::Num(n) => f.write_str(n),
            Arg::Sym(s, None) => f.write_str(s),
            Arg::Sym(s, Some(k)) => write!(f, "{s}+{k}"),
        }
    }
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let body = match &self.kind {
            Kind::Blank => String::new(),
            Kind::Label(name) | Kind::Table { label: name, .. } => format!("{name}:"),
            Kind::Instr(op, arg) => format!("{INDENT}{} {arg}", op.mnemonic()),
            Kind::Dat(Some(name), arg) => format!("{}DAT {arg}", pad(&format!("{name}:"), DATA_COLUMN + 1)),
            Kind::Dat(None, arg) => format!("{}DAT {arg}", pad("", DATA_COLUMN + 1)),
            Kind::Phase(label) => format!(";@@PHASE {label}"),
            Kind::DataStart => DATA_START.to_string(),
            Kind::Note(text) => format!(";{text}"),
        };
        match &self.comment {
            None => f.write_str(&body),
            Some(text) if text.is_empty() => write!(f, "{};", pad(&body, COMMENT_START)),
            Some(text) => write!(f, "{}; {text}", pad(&body, COMMENT_START)),
        }
    }
}

/// What an instruction starts with.
const INDENT: &str = "        ";

/// Where a comment's `;` goes on any line: past an instruction padded to the comment column, and one space.
const COMMENT_START: usize = INDENT.len() + COMMENT_COLUMN + 1;

/// `text` padded with spaces to `width`, or with one space when it is that long already.
fn pad(text: &str, width: usize) -> String {
    format!("{text:<width$}", width = width.max(text.len() + 1))
}

/// A listing as assembly text, one line each.
pub fn print(lines: &[Line]) -> String {
    lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n")
}

fn word(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

fn parse_arg(s: &str) -> Option<Arg> {
    let digits = s.strip_prefix('-').unwrap_or(s);
    if !digits.is_empty() && digits.bytes().all(|c| c.is_ascii_digit()) {
        return Some(Arg::Num(s.to_string()));
    }
    match s.split_once('+') {
        Some((name, k)) if word(name) => Some(Arg::Sym(name.to_string(), Some(k.parse().ok()?))),
        None if word(s) => Some(Arg::sym(s)),
        _ => None,
    }
}

/// An operand and the comment after it, from the text after `OP` or `DAT`: spaces, the operand, then any comment.
fn arg_and_comment(rest: &str) -> Option<(Arg, Option<String>)> {
    let rest = rest.strip_prefix(char::is_whitespace)?.trim_start();
    let end = rest.find(|c: char| c.is_whitespace() || c == ';').unwrap_or(rest.len());
    Some((parse_arg(&rest[..end])?, comment(&rest[end..])?))
}

/// A comment, or nothing, from what is left of a line: any spaces, then `;` and its text.
fn comment(rest: &str) -> Option<Option<String>> {
    let rest = rest.trim_start();
    if rest.is_empty() {
        return Some(None);
    }
    let text = rest.strip_prefix(';')?;
    Some(Some(text.trim_start().to_string()))
}

/// An instruction or a data cell, from what follows its indent or its label: `OP arg` or `DAT arg`, and a comment.
fn statement(body: &str, label: Option<&str>) -> Option<Line> {
    let word = body.get(..3)?;
    let (arg, comment) = arg_and_comment(&body[3..])?;
    let kind = match (word, label) {
        ("DAT", label) => Kind::Dat(label.map(str::to_string), arg),
        (op, None) => Kind::Instr(Op::from_mnemonic(op)?, arg),
        (_, Some(_)) => return None,
    };
    Some(Line {
        kind,
        comment,
        panic: None,
        wide: false,
    })
}

/// Read one line of a listing, with any spacing; `Display` prints it back in its layout.
pub fn parse_line(text: &str) -> Option<Line> {
    if text.is_empty() {
        return Some(Line::new(Kind::Blank));
    }
    if let Some(label) = text.strip_prefix(";@@PHASE ") {
        return Some(Line::new(Kind::Phase(label.to_string())));
    }
    if text == DATA_START {
        return Some(Line::new(Kind::DataStart));
    }
    if let Some(note) = text.strip_prefix(';') {
        return Some(Line::new(Kind::Note(note.to_string())));
    }
    if text.starts_with(char::is_whitespace) {
        return statement(text.trim_start(), None);
    }
    let (name, rest) = text.split_once(':')?;
    if !word(name) {
        return None;
    }
    let body = rest.trim_start();
    if body.starts_with("DAT") {
        return statement(body, Some(name));
    }
    Some(Line {
        kind: Kind::Label(name.to_string()),
        comment: comment(rest)?,
        panic: None,
        wide: false,
    })
}

/// Read a listing; the lines that are not in its syntax, as errors.
pub fn parse(text: &str) -> Result<Vec<Line>, String> {
    text.split('\n')
        .map(|l| parse_line(l).ok_or_else(|| format!("not a line of CARDIAC assembly: {l}")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING: &str = "; entry: main_bb0\n\
        main_bb0:\n\
        \x20       CLA 00\n\
        \x20       STO main_3     ; _3 = copy _1\n\
        \x20       ADD xs+2       ; src\n\
        \x20       JMP rt_chk_entry ; is it a card?\n\
        ; ---- locals and constants ----\n\
        rt_mul_exit: DAT 0     ; becomes JMP back to the caller\n\
        main_xs:  DAT 5        ; [i16; 3]\n\
        \x20         DAT -7\n\
        resource_tab:          ; a resource\n\
        ;@@PHASE p2\n";

    #[test]
    fn lines_print_back_in_their_layout() {
        let lines = parse(LISTING).unwrap();
        assert_eq!(print(&lines), LISTING);
        assert_eq!(lines[3].as_instr(), Some((Op::Sto, &Arg::sym("main_3"))));
        assert_eq!(lines[3].comment.as_deref(), Some("_3 = copy _1"));
        assert_eq!(lines[6].kind, Kind::DataStart);
        assert!(parse("        LDA 5").is_err());
        assert!(parse("main_xs: LDA 5").is_err());
    }

    #[test]
    fn any_spacing_reads_and_prints_in_the_layout() {
        let text = "; entry: main_bb0\nmain_bb0:\n CLA   00\n\tSTO main_3;_3 = copy _1\n  ADD xs+2 ;   src\n\
            \x20 JMP rt_chk_entry  ; is it a card?\n; ---- locals and constants ----\n\
            rt_mul_exit:DAT 0 ; becomes JMP back to the caller\n\
            main_xs: DAT  5;[i16; 3]\n DAT -7   \nresource_tab:; a resource\n;@@PHASE p2\n";
        assert_eq!(print(&parse(text).unwrap()), LISTING);
    }

    #[test]
    fn comments_start_in_the_comment_column() {
        let mut l = Line::commented(Op::Jmp, Arg::sym("rt_chk_entry"), "is it a card?");
        assert_eq!(l.to_string(), "        JMP rt_chk_entry ; is it a card?");
        l.note("src");
        assert_eq!(l.to_string(), "        JMP rt_chk_entry ; is it a card?   ; src");
        assert_eq!(
            Line::commented(Op::Cla, Arg::Num("00".into()), "one").to_string(),
            "        CLA 00         ; one"
        );
        let mut cell = Line::data_cell("rt_mul_exit", Arg::Num("0".into()));
        cell.note("becomes JMP back to the caller");
        assert_eq!(
            cell.to_string(),
            "rt_mul_exit: DAT 0     ; becomes JMP back to the caller"
        );
        assert_eq!(
            Line::data_cell("t0", Arg::Num("0".into())).to_string(),
            "t0:       DAT 0"
        );
    }
}
