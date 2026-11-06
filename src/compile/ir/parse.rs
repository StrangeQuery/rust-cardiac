//! A recursive-descent parser for one line of a MIR block: a statement or a terminator.
use super::lex::{tokens, Tok, Token};
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub at: usize,
    pub expected: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "at byte {}: expected {}", self.at, self.expected)
    }
}

type Parsed<T> = Result<T, ParseError>;

/// Parse a statement or a terminator.
pub fn parse_line(text: &str) -> Parsed<Line> {
    let toks = tokens(text).map_err(|at| ParseError {
        at,
        expected: "a token".into(),
    })?;
    let mut p = Parser { text, toks, pos: 0 };
    let line = p.line()?;
    if p.pos != p.toks.len() {
        return Err(p.error("the end of the line"));
    }
    Ok(line)
}

/// Parse an operand, as a const item's value gives it: `const 40_i16`.
pub fn parse_operand(text: &str) -> Parsed<Operand> {
    let toks = tokens(text).map_err(|at| ParseError {
        at,
        expected: "a token".into(),
    })?;
    let mut p = Parser { text, toks, pos: 0 };
    let o = p.operand()?;
    if p.pos != p.toks.len() {
        return Err(p.error("the end of the operand"));
    }
    Ok(o)
}

/// Parse a path as an expression names it (a call's function, `<Range<i16> as Iterator>::next`): the whole text.
pub fn parse_path(text: &str) -> Parsed<Path> {
    let toks = tokens(text).map_err(|at| ParseError {
        at,
        expected: "a token".into(),
    })?;
    let mut p = Parser { text, toks, pos: 0 };
    let path = p.path(true)?;
    if p.pos != p.toks.len() {
        return Err(p.error("the end of the path"));
    }
    Ok(path)
}

/// Parse a type, as a local's declaration gives it.
pub fn parse_ty(text: &str) -> Parsed<Ty> {
    let toks = tokens(text).map_err(|at| ParseError {
        at,
        expected: "a token".into(),
    })?;
    let mut p = Parser { text, toks, pos: 0 };
    let t = p.ty()?;
    if p.pos != p.toks.len() {
        return Err(p.error("the end of the type"));
    }
    Ok(t)
}

impl Unsupported {
    /// A line kept as text, with the locals it names.
    pub fn of(text: &str) -> Unsupported {
        let locals = tokens(text)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|t| match t.tok {
                Tok::Ident(s) => local_named(&s),
                _ => None,
            })
            .collect();
        Unsupported {
            text: text.to_string(),
            locals,
        }
    }
}

/// `_5` is local 5; `_unit` is the struct pass's own.
pub(super) fn local_named(s: &str) -> Option<Local> {
    if s == "_unit" {
        return Some(Local::UNIT);
    }
    let digits = s.strip_prefix('_')?;
    if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok().map(Local)
}

struct Parser<'a> {
    text: &'a str,
    toks: Vec<Token>,
    pos: usize,
}

impl Parser<'_> {
    fn error(&self, expected: &str) -> ParseError {
        let at = self.toks.get(self.pos).map_or(self.text.len(), |t| t.start);
        ParseError {
            at,
            expected: expected.to_string(),
        }
    }

    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|t| &t.tok)
    }

    fn peek_at(&self, k: usize) -> Option<&Tok> {
        self.toks.get(self.pos + k).map(|t| &t.tok)
    }

    fn is_punct(&self, p: &str) -> bool {
        matches!(self.peek(), Some(Tok::Punct(q)) if *q == p)
    }

    fn is_ident(&self, s: &str) -> bool {
        matches!(self.peek(), Some(Tok::Ident(x)) if x == s)
    }

    fn eat_punct(&mut self, p: &str) -> bool {
        if self.is_punct(p) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn eat_ident(&mut self, s: &str) -> bool {
        if self.is_ident(s) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn punct(&mut self, p: &str) -> Parsed<()> {
        if self.eat_punct(p) {
            Ok(())
        } else {
            Err(self.error(&format!("`{p}`")))
        }
    }

    fn keyword(&mut self, s: &str) -> Parsed<()> {
        if self.eat_ident(s) {
            Ok(())
        } else {
            Err(self.error(&format!("`{s}`")))
        }
    }

    fn ident(&mut self) -> Parsed<String> {
        match self.peek() {
            Some(Tok::Ident(s)) => {
                let s = s.clone();
                self.pos += 1;
                Ok(s)
            }
            _ => Err(self.error("a name")),
        }
    }

    /// An integer without a suffix.
    fn number(&mut self) -> Parsed<u128> {
        match self.peek() {
            Some(Tok::Int { digits, suffix: None }) => {
                let v = digits.parse().map_err(|_| self.error("a number that fits"))?;
                self.pos += 1;
                Ok(v)
            }
            _ => Err(self.error("a number")),
        }
    }

    fn local(&mut self) -> Parsed<Local> {
        match self.peek() {
            Some(Tok::Ident(s)) => match local_named(s) {
                Some(l) => {
                    self.pos += 1;
                    Ok(l)
                }
                None => Err(self.error("a local")),
            },
            _ => Err(self.error("a local")),
        }
    }

    fn block(&mut self) -> Parsed<BasicBlock> {
        match self.peek() {
            Some(Tok::Ident(s)) => match s.strip_prefix("bb").and_then(|d| d.parse().ok()) {
                Some(n) => {
                    self.pos += 1;
                    Ok(BasicBlock(n))
                }
                None => Err(self.error("a block")),
            },
            _ => Err(self.error("a block")),
        }
    }

    /// The text of the tokens from `from` up to here.
    fn text_from(&self, from: usize) -> &str {
        let start = self.toks[from].start;
        let end = self.toks[self.pos - 1].end;
        &self.text[start..end]
    }

    /// The kind of a cast, from the tokens from `from` up to the closing `)` the parser stands at.
    fn cast_kind(&self, from: usize) -> CastKind {
        let toks: Vec<&Tok> = self.toks[from..self.pos].iter().map(|t| &t.tok).collect();
        let known = match toks.as_slice() {
            [Tok::Ident(name)] => CastKind::from_name(name),
            [Tok::Ident(c), Tok::Punct("("), Tok::Ident(u), Tok::Punct(","), Tok::Ident(source), Tok::Punct(")")]
                if c == "PointerCoercion" && u == "Unsize" =>
            {
                CoercionSource::from_name(source).map(CastKind::Unsize)
            }
            _ => None,
        };
        known.unwrap_or_else(|| {
            if toks.is_empty() {
                CastKind::Other(String::new())
            } else {
                CastKind::Other(self.text_from(from).to_string())
            }
        })
    }

    // ---- lines ----

    fn line(&mut self) -> Parsed<Line> {
        if self.eat_ident("goto") {
            self.punct("->")?;
            let b = self.block()?;
            self.punct(";")?;
            return Ok(Line::Terminator(Terminator::Goto(b)));
        }
        if self.is_ident("return") && self.peek_at(1) == Some(&Tok::Punct(";")) {
            self.pos += 2;
            return Ok(Line::Terminator(Terminator::Return));
        }
        if self.is_ident("unreachable") && self.peek_at(1) == Some(&Tok::Punct(";")) {
            self.pos += 2;
            return Ok(Line::Terminator(Terminator::Unreachable));
        }
        if self.is_ident("nop") && self.peek_at(1) == Some(&Tok::Punct(";")) {
            self.pos += 2;
            return Ok(Line::Statement(Statement::Nop));
        }
        if self.is_ident("switchInt") && self.peek_at(1) == Some(&Tok::Punct("(")) {
            return self.switch_int().map(Line::Terminator);
        }
        if self.is_ident("assert") && self.peek_at(1) == Some(&Tok::Punct("(")) {
            return self.assert().map(Line::Terminator);
        }
        if self.is_ident("drop") && self.peek_at(1) == Some(&Tok::Punct("(")) {
            return self.drop_line().map(Line::Terminator);
        }
        if self.is_ident("copy_nonoverlapping") && self.peek_at(1) == Some(&Tok::Punct("(")) {
            self.pos += 2;
            let arg = |p: &mut Self, name: &str, last: bool| -> Parsed<Operand> {
                p.keyword(name)?;
                p.punct("=")?;
                let o = p.operand()?;
                p.punct(if last { ")" } else { "," })?;
                Ok(o)
            };
            let dst = arg(self, "dst", false)?;
            let src = arg(self, "src", false)?;
            let count = arg(self, "count", true)?;
            self.punct(";")?;
            return Statement::copy_one(src, dst, count)
                .map(Line::Statement)
                .ok_or_else(|| self.error("a copy of one element through two pointers"));
        }
        if self.is_ident("assume") && self.peek_at(1) == Some(&Tok::Punct("(")) {
            self.pos += 2;
            let o = self.operand()?;
            self.punct(")")?;
            self.punct(";")?;
            return Ok(Line::Statement(Statement::Assume(o)));
        }
        for (name, make) in [
            ("StorageLive", Statement::StorageLive as fn(Local) -> Statement),
            ("StorageDead", Statement::StorageDead),
        ] {
            if self.is_ident(name) && self.peek_at(1) == Some(&Tok::Punct("(")) {
                self.pos += 2;
                let l = self.local()?;
                self.punct(")")?;
                self.punct(";")?;
                return Ok(Line::Statement(make(l)));
            }
        }
        let dest = self.place()?;
        self.punct("=")?;
        if let Some(call) = self.try_call(&dest)? {
            return Ok(Line::Terminator(call));
        }
        let r = self.rvalue()?;
        self.punct(";")?;
        Ok(Line::Statement(Statement::Assign(dest, r)))
    }

    fn switch_int(&mut self) -> Parsed<Terminator> {
        self.keyword("switchInt")?;
        self.punct("(")?;
        let discr = self.operand()?;
        self.punct(")")?;
        self.punct("->")?;
        self.punct("[")?;
        let (mut arms, mut otherwise) = (vec![], None);
        loop {
            if self.eat_ident("otherwise") {
                self.punct(":")?;
                otherwise = Some(self.block()?);
            } else {
                // the bits, as MIR prints them, until the front end reads them as the operand's type
                let v = self.number()? as i128;
                self.punct(":")?;
                arms.push((v, self.block()?));
            }
            if !self.eat_punct(",") {
                break;
            }
        }
        self.punct("]")?;
        self.punct(";")?;
        Ok(Terminator::SwitchInt { discr, arms, otherwise })
    }

    fn assert(&mut self) -> Parsed<Terminator> {
        self.keyword("assert")?;
        self.punct("(")?;
        let expected = !self.eat_punct("!");
        let cond = self.operand()?;
        self.punct(",")?;
        let msg = match self.peek() {
            Some(Tok::Str(s)) => s.clone(),
            _ => return Err(self.error("the assert's message")),
        };
        self.pos += 1;
        let mut msg_args = vec![];
        while self.eat_punct(",") {
            msg_args.push(self.operand()?);
        }
        self.punct(")")?;
        self.punct("->")?;
        self.punct("[")?;
        self.keyword("success")?;
        self.punct(":")?;
        let target = self.block()?;
        self.punct(",")?;
        let unwind = self.unwind()?;
        self.punct("]")?;
        self.punct(";")?;
        Ok(Terminator::Assert {
            cond,
            expected,
            msg,
            msg_args,
            target,
            unwind,
        })
    }

    fn drop_line(&mut self) -> Parsed<Terminator> {
        self.keyword("drop")?;
        self.punct("(")?;
        let place = self.place()?;
        self.punct(")")?;
        self.punct("->")?;
        self.punct("[")?;
        self.keyword("return")?;
        self.punct(":")?;
        let target = self.block()?;
        self.punct(",")?;
        let unwind = self.unwind()?;
        self.punct("]")?;
        self.punct(";")?;
        Ok(Terminator::Drop { place, target, unwind })
    }

    fn unwind(&mut self) -> Parsed<Unwind> {
        self.keyword("unwind")?;
        if self.eat_punct(":") {
            return Ok(Unwind::Cleanup(self.block()?));
        }
        if self.eat_ident("continue") {
            return Ok(Unwind::Continue);
        }
        if self.eat_ident("unreachable") {
            return Ok(Unwind::Unreachable);
        }
        self.keyword("terminate")?;
        self.punct("(")?;
        let why = self.ident()?;
        self.punct(")")?;
        Ok(Unwind::Terminate(why))
    }

    /// `f(args) -> [return: bbN, unwind ...];` or `f(args) -> unwind ...;`, if that is what follows.
    fn try_call(&mut self, dest: &Place) -> Parsed<Option<Terminator>> {
        let back = self.pos;
        let Ok(func) = self.path(true) else {
            self.pos = back;
            return Ok(None);
        };
        if !self.eat_punct("(") {
            self.pos = back;
            return Ok(None);
        }
        let mut args = vec![];
        if !self.eat_punct(")") {
            loop {
                let Ok(a) = self.operand() else {
                    self.pos = back;
                    return Ok(None);
                };
                args.push(a);
                if !self.eat_punct(",") {
                    break;
                }
            }
            if !self.eat_punct(")") {
                self.pos = back;
                return Ok(None);
            }
        }
        if !self.eat_punct("->") {
            self.pos = back;
            return Ok(None);
        }
        let (target, unwind) = if self.eat_punct("[") {
            self.keyword("return")?;
            self.punct(":")?;
            let t = self.block()?;
            self.punct(",")?;
            let u = self.unwind()?;
            self.punct("]")?;
            (Some(t), u)
        } else {
            (None, self.unwind()?)
        };
        self.punct(";")?;
        Ok(Some(Terminator::Call {
            dest: dest.clone(),
            func,
            args,
            target,
            unwind,
        }))
    }

    // ---- places and operands ----

    fn place(&mut self) -> Parsed<Place> {
        let mut place = if self.eat_punct("(") {
            if self.eat_punct("*") {
                let mut inner = self.place()?;
                self.punct(")")?;
                inner.projection.push(Projection::Deref);
                inner
            } else {
                let mut inner = self.place()?;
                if self.eat_punct(".") {
                    let k = self.number()? as u32;
                    self.punct(":")?;
                    let t = self.ty()?;
                    inner.projection.push(Projection::Field(k, t));
                } else {
                    self.keyword("as")?;
                    let v = self.ident()?;
                    inner.projection.push(Projection::Downcast(v));
                }
                self.punct(")")?;
                inner
            }
        } else if self.eat_punct("@") {
            let func = self.ident()?;
            self.punct(".")?;
            let local = self.number()? as u32;
            Place {
                base: PlaceBase::Result { func, local },
                projection: vec![],
            }
        } else {
            Place::local(self.local()?)
        };
        while self.eat_punct("[") {
            if self.is_punct("(") {
                let index = self.place()?;
                place.projection.push(Projection::IndexPlace(Box::new(index)));
            } else if matches!(self.peek(), Some(Tok::Int { .. })) {
                let offset = self.number()? as u64;
                if self.eat_punct(".") {
                    self.punct(".")?;
                    let to = self.number()? as u64;
                    place.projection.push(Projection::Subslice { from: offset, to });
                } else {
                    self.keyword("of")?;
                    let min_length = self.number()? as u64;
                    place.projection.push(Projection::ConstantIndex { offset, min_length });
                }
            } else {
                let i = self.local()?;
                if self.eat_punct("+") {
                    let k = self.number()? as u32;
                    place.projection.push(Projection::IndexPlus(i, k));
                } else {
                    place.projection.push(Projection::Index(i));
                }
            }
            self.punct("]")?;
        }
        Ok(place)
    }

    fn operand(&mut self) -> Parsed<Operand> {
        if self.eat_ident("copy") {
            return Ok(Operand::Copy(self.place()?));
        }
        if self.eat_ident("move") {
            return Ok(Operand::Move(self.place()?));
        }
        if self.eat_ident("const") {
            return Ok(Operand::Constant(self.constant()?));
        }
        // a struct value written out: a path, then its fields in braces
        if matches!(self.peek(), Some(Tok::Ident(s)) if local_named(s).is_none()) {
            let path = self.path(true)?;
            self.punct("{")?;
            let mut fields = vec![];
            loop {
                let name = self.ident()?;
                self.punct(":")?;
                fields.push((name, self.operand()?));
                if !self.eat_punct(",") {
                    break;
                }
            }
            self.punct("}")?;
            return Ok(Operand::Aggregate(path, fields));
        }
        Ok(Operand::Place(self.place()?))
    }

    fn constant(&mut self) -> Parsed<Const> {
        let negative = self.eat_punct("-");
        if let Some(Tok::Int { digits, suffix }) = self.peek() {
            let ty = suffix
                .as_deref()
                .and_then(IntTy::from_name)
                .ok_or_else(|| self.error("an integer's type"))?;
            let v: i128 = digits.parse().map_err(|_| self.error("an integer that fits"))?;
            self.pos += 1;
            return Ok(Const::Int {
                value: if negative { -v } else { v },
                ty,
            });
        }
        if negative {
            return Err(self.error("a number"));
        }
        // a value of a type with no bytes (a closure that captures nothing): `ZeroSized: {closure@...}`
        if self.is_ident("ZeroSized") && self.peek_at(1) == Some(&Tok::Punct(":")) {
            self.pos += 2;
            self.ty()?;
            return Ok(Const::Unit);
        }
        if self.eat_ident("true") {
            return Ok(Const::Bool(true));
        }
        if self.eat_ident("false") {
            return Ok(Const::Bool(false));
        }
        if self.is_punct("(") && self.peek_at(1) == Some(&Tok::Punct(")")) {
            self.pos += 2;
            return Ok(Const::Unit);
        }
        if let Some(Tok::Str(s)) = self.peek() {
            let s = s.clone();
            self.pos += 1;
            return Ok(Const::Str(s));
        }
        // a pointer into an allocation: `{alloc1: &[i16; 4]}`
        if let (true, Some(Tok::Ident(name)), Some(Tok::Punct(":"))) =
            (self.is_punct("{"), self.peek_at(1), self.peek_at(2))
        {
            if name.starts_with("alloc") {
                let name = name.clone();
                self.pos += 3;
                let ty = self.ty()?;
                self.punct("}")?;
                return Ok(Const::Alloc(name, ty));
            }
        }
        if let Some(Tok::Bytes(bs)) = self.peek() {
            let bs = bs.clone();
            self.pos += 1;
            return Ok(Const::Bytes(bs));
        }
        if let Some(&Tok::Char(c)) = self.peek() {
            self.pos += 1;
            return Ok(Const::Char(c));
        }
        let path = self.path(true)?;
        // a struct's value: `Path {{ field: value, .. }}`
        if self.is_punct("{") && self.peek_at(1) == Some(&Tok::Punct("{")) {
            self.pos += 2;
            let mut fields = vec![];
            loop {
                let name = self.ident()?;
                self.punct(":")?;
                fields.push((name, self.constant()?));
                if !self.eat_punct(",") {
                    break;
                }
            }
            self.punct("}")?;
            self.punct("}")?;
            return Ok(Const::Adt(path, fields));
        }
        // a variant's value: `Result::<i16, i16>::Err(-9_i16)`
        if self.eat_punct("(") {
            let mut values = vec![];
            if !self.eat_punct(")") {
                loop {
                    values.push(self.constant()?);
                    if !self.eat_punct(",") {
                        break;
                    }
                }
                self.punct(")")?;
            }
            return Ok(Const::Variant(path, values));
        }
        // a promoted constant: `main::promoted[1]`
        if path.name() == "promoted" && self.is_punct("[") {
            if let Some(Tok::Int { digits, suffix: None }) = self.peek_at(1) {
                let n: u32 = digits.parse().map_err(|_| self.error("a promoted's number"))?;
                if self.peek_at(2) == Some(&Tok::Punct("]")) {
                    self.pos += 3;
                    let mut function = path;
                    function.segments.pop();
                    return Ok(Const::Promoted(function, n));
                }
            }
        }
        if let Some(c) = limit_on_a_card(&path) {
            return Ok(c);
        }
        Ok(Const::Path(path))
    }

    // ---- rvalues ----

    fn rvalue(&mut self) -> Parsed<Rvalue> {
        if self.eat_punct("&") {
            let kind = if self.eat_ident("raw") {
                if self.eat_ident("mut") {
                    BorrowKind::RawMut
                } else {
                    self.keyword("const")?;
                    BorrowKind::RawConst
                }
            } else if self.eat_ident("mut") {
                BorrowKind::Mut
            } else {
                BorrowKind::Shared
            };
            return Ok(Rvalue::Ref(kind, self.place()?));
        }
        // `*mut [T] from (p, n)`
        if self.is_punct("*") {
            let back = self.pos;
            if let Ok(Ty::Ptr { mutable, to }) = self.ty() {
                if self.eat_ident("from") {
                    self.punct("(")?;
                    let p = self.operand()?;
                    self.punct(",")?;
                    let n = self.operand()?;
                    self.punct(")")?;
                    return Rvalue::slice_from_parts(p, *to, mutable, &n)
                        .ok_or_else(|| self.error("a pointer to a slice, made of its parts"));
                }
            }
            self.pos = back;
        }
        if self.eat_punct("[") {
            let first = self.operand()?;
            if self.eat_punct(";") {
                let n = self.number()? as u64;
                self.punct("]")?;
                return Ok(Rvalue::Repeat(first, n));
            }
            let mut items = vec![first];
            while self.eat_punct(",") {
                items.push(self.operand()?);
            }
            self.punct("]")?;
            return Ok(Rvalue::Array(items));
        }
        let starts_operand =
            |p: &Parser| matches!(p.peek(), Some(Tok::Ident(s)) if s == "copy" || s == "move" || s == "const");
        if self.is_punct("(")
            && matches!(self.peek_at(1), Some(Tok::Ident(s)) if s == "copy" || s == "move" || s == "const")
        {
            self.pos += 1;
            let mut items = vec![self.operand()?];
            while self.eat_punct(",") {
                items.push(self.operand()?);
            }
            self.punct(")")?;
            return Ok(Rvalue::Tuple(items));
        }
        if starts_operand(self)
            || self.is_punct("(")
            || self.is_punct("@")
            || matches!(self.peek(), Some(Tok::Ident(s)) if local_named(s).is_some())
        {
            let o = self.operand()?;
            if self.eat_ident("as") {
                let t = self.ty()?;
                self.punct("(")?;
                let from = self.pos;
                let mut depth = 1;
                while depth > 0 {
                    match self.peek() {
                        Some(Tok::Punct("(")) => depth += 1,
                        Some(Tok::Punct(")")) => depth -= 1,
                        None => return Err(self.error("`)`")),
                        _ => {}
                    }
                    self.pos += 1;
                }
                self.pos -= 1;
                let kind = self.cast_kind(from);
                self.pos += 1;
                return Ok(Rvalue::Cast(o, t, kind));
            }
            return Ok(Rvalue::Use(o));
        }
        // a name with arguments: an operation, or an enum variant built
        if self.is_ident("Lin") && self.peek_at(1) == Some(&Tok::Punct("(")) {
            self.pos += 2;
            let mut terms = vec![];
            loop {
                let sign = if self.eat_punct("-") {
                    -1
                } else {
                    self.punct("+")?;
                    1
                };
                let k = self.number()? as i64;
                self.punct("*")?;
                terms.push((sign * k, self.operand()?));
                if !self.eat_punct(",") {
                    break;
                }
            }
            self.punct(")")?;
            return Ok(Rvalue::Lin(terms));
        }
        if self.is_ident("SumWithOverflow") && self.peek_at(1) == Some(&Tok::Punct("(")) {
            self.pos += 2;
            let mut terms = vec![];
            loop {
                let sign = if self.eat_punct("-") {
                    Sign::Minus
                } else {
                    self.punct("+")?;
                    Sign::Plus
                };
                terms.push((sign, self.operand()?));
                if !self.eat_punct(",") {
                    break;
                }
            }
            self.punct(")")?;
            return Ok(Rvalue::Sum(terms));
        }
        if self.is_ident("discriminant") && self.peek_at(1) == Some(&Tok::Punct("(")) {
            self.pos += 2;
            let p = self.place()?;
            self.punct(")")?;
            return Ok(Rvalue::Discriminant(p));
        }
        let path = self.path(true)?;
        if self.eat_punct("{") {
            let mut fields = vec![];
            loop {
                let name = self.ident()?;
                self.punct(":")?;
                fields.push((name, self.operand()?));
                if !self.eat_punct(",") {
                    break;
                }
            }
            self.punct("}")?;
            return Ok(Rvalue::Struct(path, fields));
        }
        // a unit variant, `Shape::Dot`
        if !self.eat_punct("(") {
            return Ok(Rvalue::Variant(path, vec![]));
        }
        let mut args = vec![];
        if !self.eat_punct(")") {
            loop {
                args.push(self.operand()?);
                if !self.eat_punct(",") {
                    break;
                }
            }
            self.punct(")")?;
        }
        let single = path.qself.is_none()
            && path.segments.len() == 1
            && matches!(&path.segments[0], Segment::Name { args: None, .. });
        if single {
            let name = path.name();
            if let (Some(op), 2) = (BinOp::from_name(name), args.len()) {
                let b = args.pop().unwrap();
                let a = args.pop().unwrap();
                return Ok(Rvalue::BinaryOp(op, a, b));
            }
            if args.len() == 1 && (name == "Neg" || name == "Not") {
                let op = if name == "Neg" { UnOp::Neg } else { UnOp::Not };
                return Ok(Rvalue::UnaryOp(op, args.pop().unwrap()));
            }
        }
        Ok(Rvalue::Variant(path, args))
    }

    // ---- paths and types ----

    /// A path; in an expression (`expr`), generic arguments come after `::`.
    fn path(&mut self, expr: bool) -> Parsed<Path> {
        let mut qself = None;
        let mut segments = vec![];
        if self.is_punct("<") && !(self.peek_at(1) == Some(&Tok::Ident("impl".into()))) {
            self.pos += 1;
            let ty = self.ty()?;
            let as_trait = if self.eat_ident("as") {
                Some(self.path(false)?)
            } else {
                None
            };
            self.punct(">")?;
            qself = Some(Box::new(QSelf { ty, as_trait }));
            self.punct("::")?;
        }
        loop {
            if self.is_punct("<") && self.peek_at(1) == Some(&Tok::Ident("impl".into())) {
                self.pos += 2;
                let t = self.ty()?;
                self.punct(">")?;
                segments.push(Segment::Impl(t));
            } else {
                let name = self.ident()?;
                let mut args = None;
                if expr
                    && self.is_punct("::")
                    && self.peek_at(1) == Some(&Tok::Punct("<"))
                    && self.peek_at(2) != Some(&Tok::Ident("impl".into()))
                {
                    self.pos += 2;
                    args = Some(GenericArgs {
                        turbofish: true,
                        args: self.generic_args()?,
                    });
                } else if !expr && self.eat_punct("<") {
                    args = Some(GenericArgs {
                        turbofish: false,
                        args: self.generic_args()?,
                    });
                }
                segments.push(Segment::Name { name, args });
            }
            if self.is_punct("::") && matches!(self.peek_at(1), Some(Tok::Ident(_)) | Some(Tok::Punct("<"))) {
                self.pos += 1;
            } else {
                break;
            }
        }
        Ok(Path { qself, segments })
    }

    /// The arguments after `<`, and the closing `>`.
    fn generic_args(&mut self) -> Parsed<Vec<GenericArg>> {
        let mut args = vec![];
        loop {
            match self.peek() {
                Some(Tok::Lifetime(l)) => {
                    args.push(GenericArg::Lifetime(l.clone()));
                    self.pos += 1;
                }
                Some(Tok::Int { suffix: None, .. }) => args.push(GenericArg::Const(self.number()? as u64)),
                _ => args.push(GenericArg::Ty(self.ty()?)),
            }
            if !self.eat_punct(",") {
                break;
            }
        }
        self.punct(">")?;
        Ok(args)
    }

    fn ty(&mut self) -> Parsed<Ty> {
        // a closure's type, which the IR doesn't model: its text, as rustc wrote it, which is also how a closure's
        // own function gives it for its first parameter
        if let Some(Tok::Closure(t)) = self.peek() {
            let t = t.clone();
            self.pos += 1;
            return Ok(Ty::Other(t));
        }
        if self.eat_punct("(") {
            if self.eat_punct(")") {
                return Ok(Ty::Unit);
            }
            let mut ts = vec![self.ty()?];
            while self.eat_punct(",") {
                if self.is_punct(")") {
                    break;
                }
                ts.push(self.ty()?);
            }
            self.punct(")")?;
            return Ok(Ty::Tuple(ts));
        }
        if self.eat_punct("[") {
            let t = self.ty()?;
            if self.eat_punct(";") {
                let n = match self.peek() {
                    Some(Tok::Int { suffix: None, .. }) => ArrayLen::Known(self.number()? as u64),
                    _ => ArrayLen::Param(self.ident()?),
                };
                self.punct("]")?;
                return Ok(Ty::Array(Box::new(t), n));
            }
            self.punct("]")?;
            return Ok(Ty::Slice(Box::new(t)));
        }
        if self.eat_punct("&") {
            let lifetime = match self.peek() {
                Some(Tok::Lifetime(l)) => {
                    let l = l.clone();
                    self.pos += 1;
                    Some(l)
                }
                _ => None,
            };
            let mutable = self.eat_ident("mut");
            return Ok(Ty::Ref {
                lifetime,
                mutable,
                to: Box::new(self.ty()?),
            });
        }
        if self.eat_punct("*") {
            let mutable = if self.eat_ident("mut") {
                true
            } else {
                self.keyword("const")?;
                false
            };
            return Ok(Ty::Ptr {
                mutable,
                to: Box::new(self.ty()?),
            });
        }
        if self.eat_punct("!") {
            return Ok(Ty::Never);
        }
        if self.eat_ident("dyn") {
            return Ok(Ty::Dyn(self.path(false)?));
        }
        let path = self.path(false)?;
        let simple = path.qself.is_none()
            && path.segments.len() == 1
            && matches!(&path.segments[0], Segment::Name { args: None, .. });
        if simple {
            let name = path.name();
            if let Some(t) = IntTy::from_name(name) {
                return Ok(Ty::Int(t));
            }
            match name {
                "bool" => return Ok(Ty::Bool),
                "char" => return Ok(Ty::Char),
                "str" => return Ok(Ty::Str),
                _ => {}
            }
        }
        Ok(Ty::Path(path))
    }
}

/// rustc prints a literal at its type's limit by name: `255_u8` is `u8::MAX` (in an array, an operand, an argument,
/// a field), `-128_i8` is `i8::MIN`. One a card holds is read as the number it is (`u8::MAX`, `i8::MIN`), and is
/// then a constant like any other. The others stay names, as before: past a card, they are refused where they
/// are used, but for a comparison with `i16::MIN` or `i16::MAX`, which the card settles
/// (`compile::settled_by_the_card`), and an index of `usize::MAX` that only a failing bounds check reads
/// (`blocks::settled_checks`).
pub fn limit_on_a_card(path: &Path) -> Option<Const> {
    let [Segment::Name { name: ty, .. }, Segment::Name { name: which, .. }] = path.segments.as_slice() else {
        return None;
    };
    let ty = IntTy::from_name(ty).filter(|_| path.is_plain())?;
    let bits = match ty {
        IntTy::I8 | IntTy::U8 => 8,
        IntTy::I16 | IntTy::U16 => 16,
        IntTy::I32 | IntTy::U32 => 32,
        IntTy::I64 | IntTy::U64 | IntTy::Isize | IntTy::Usize => 64,
        // u128::MAX is past an i128; neither limit is anywhere near a card
        IntTy::I128 | IntTy::U128 => return None,
    };
    let signed = matches!(ty, IntTy::I8 | IntTy::I16 | IntTy::I32 | IntTy::I64 | IntTy::Isize);
    let value: i128 = match (which.as_str(), signed) {
        ("MAX", true) => (1 << (bits - 1)) - 1,
        ("MIN", true) => -(1 << (bits - 1)),
        ("MAX", false) => (1 << bits) - 1,
        ("MIN", false) => 0,
        _ => return None,
    };
    (-999..=999).contains(&value).then_some(Const::Int { value, ty })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cast_kind(line: &str) -> CastKind {
        match parse_line(line) {
            Ok(Line::Statement(Statement::Assign(_, Rvalue::Cast(_, _, kind)))) => kind,
            other => panic!("not a cast: {other:?}"),
        }
    }

    fn constant(line: &str) -> Const {
        match parse_line(line) {
            Ok(Line::Statement(Statement::Assign(_, Rvalue::Use(Operand::Constant(c))))) => c,
            other => panic!("not a constant: {other:?}"),
        }
    }

    #[test]
    fn a_limit_a_card_holds_is_its_number() {
        assert_eq!(constant("_1 = const u8::MAX;").to_string(), "255_u8");
        assert_eq!(constant("_1 = const i8::MIN;").to_string(), "-128_i8");
        for past in [
            "i16::MIN",
            "i16::MAX",
            "usize::MAX",
            "u16::MAX",
            "i32::MIN",
            "u128::MAX",
            "N::MAX",
        ] {
            assert!(
                matches!(constant(&format!("_1 = const {past};")), Const::Path(_)),
                "{past}"
            );
        }
    }

    #[test]
    fn a_switch_arm_is_its_bits_read_as_the_switch_type() {
        let Ok(Line::Terminator(Terminator::SwitchInt { arms, .. })) =
            parse_line("switchInt(copy _4) -> [65535: bb3, 4294967295: bb4, otherwise: bb5];")
        else {
            panic!("not a switchInt")
        };
        assert_eq!(arms.iter().map(|a| a.0).collect::<Vec<_>>(), [65535, 4294967295]);
        assert_eq!(IntTy::I16.from_bits(65535), -1);
        assert_eq!(IntTy::I32.from_bits(4294967295), -1);
        assert_eq!(IntTy::I32.from_bits(4294966297), -999);
        assert_eq!(IntTy::I16.from_bits(32767), 32767);
        assert_eq!(IntTy::U16.from_bits(65535), 65535);
        assert_eq!(IntTy::U8.from_bits(255), 255);
        assert_eq!(IntTy::Isize.from_bits(u64::MAX as i128), -1);
        assert_eq!(IntTy::I128.from_bits(-1), -1);
    }

    #[test]
    fn cast_kinds_read_and_print_back_as_rustc_writes_them() {
        let lines = [
            ("_11 = move _1 as i16 (IntToInt);", CastKind::IntToInt),
            ("_2 = copy _1 as *const u8 (PtrToPtr);", CastKind::PtrToPtr),
            ("_2 = copy _1 as u16 (Transmute);", CastKind::Transmute),
            (
                "_3 = copy _4 as &dyn core::fmt::Debug (PointerCoercion(Unsize, Implicit));",
                CastKind::Unsize(CoercionSource::Implicit),
            ),
            (
                "_3 = copy _4 as *const u8 (PointerCoercion(ReifyFnPointer, AsCast));",
                CastKind::Other("PointerCoercion(ReifyFnPointer, AsCast)".into()),
            ),
            ("_2 = copy _1 as u16 (Novel);", CastKind::Other("Novel".into())),
            ("_2 = copy _1 as u16 ();", CastKind::Other(String::new())),
        ];
        for (line, kind) in lines {
            assert_eq!(cast_kind(line), kind);
            assert_eq!(parse_line(line).unwrap().to_string(), line);
        }
    }
}
