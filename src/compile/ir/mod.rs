//! The compiler's intermediate representation: a function's MIR as typed values rather than text.
//!
//! The shapes and names follow rustc's own MIR (`Place`, `Operand`, `Rvalue`, `Statement`, `Terminator`), and
//! `Display` prints each one in the syntax `rustc --emit=mir` uses, so a parsed statement prints back exactly as
//! rustc wrote it. A few variants are this compiler's own, made by its passes rather than by rustc:
//! `Rvalue::Lin` and `Rvalue::Sum` (sums worked out in the accumulator), the unsigned `BinOp::UDiv` and
//! `BinOp::URem`, `Projection::IndexPlus` (`xs[i+2]`), `PlaceBase::Result` (a callee's result cell, `@f.2`), and
//! `Statement::Nop`. They print in a syntax of the same style.
//!
//! Only what the compiler can compile is represented in full. A statement or terminator outside that subset is
//! kept as `Unsupported`, with its text and the locals it names, so the passes still see every use of a local and
//! the compiler can reject it with the line that rustc wrote.
use std::fmt;

mod body;
mod file;
mod function;
mod lex;
mod parse;
mod visit;
pub mod visit_mut;

pub use body::{blocks_naming, find, lines_naming, occurrences, predecessors};
pub use file::{parse_file, parse_item_path, ConstItem, ConstValue, File, FnItem, Item, ItemPath};
pub use function::{parse_body, parse_params, Body, Debug, Decl};
pub use parse::{limit_on_a_card, parse_line, parse_operand, parse_path, parse_ty, ParseError};
pub use visit::Locals;

/// A basic block: its statements, then the terminator that ends it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub name: BasicBlock,
    pub statements: Vec<Statement>,
    pub terminator: Terminator,
}

impl Local {
    /// `_5` as a local.
    pub fn parse(s: &str) -> Option<Local> {
        parse::local_named(s)
    }
}

/// A local, `_N`. The struct pass adds one of its own, `_unit`, to receive a call's `()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Local(pub u32);

impl Local {
    pub const UNIT: Local = Local(u32::MAX);
}

/// A basic block, `bbN`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BasicBlock(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IntTy {
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
}

impl IntTy {
    pub const ALL: [IntTy; 12] = [
        IntTy::I8,
        IntTy::I16,
        IntTy::I32,
        IntTy::I64,
        IntTy::I128,
        IntTy::Isize,
        IntTy::U8,
        IntTy::U16,
        IntTy::U32,
        IntTy::U64,
        IntTy::U128,
        IntTy::Usize,
    ];

    pub fn name(self) -> &'static str {
        match self {
            IntTy::I8 => "i8",
            IntTy::I16 => "i16",
            IntTy::I32 => "i32",
            IntTy::I64 => "i64",
            IntTy::I128 => "i128",
            IntTy::Isize => "isize",
            IntTy::U8 => "u8",
            IntTy::U16 => "u16",
            IntTy::U32 => "u32",
            IntTy::U64 => "u64",
            IntTy::U128 => "u128",
            IntTy::Usize => "usize",
        }
    }

    pub fn from_name(s: &str) -> Option<IntTy> {
        IntTy::ALL.into_iter().find(|t| t.name() == s)
    }

    /// The value a bit pattern is in this type, as MIR prints a switch arm: 65535 is -1 as an i16, 255 as a u8. The
    /// pattern is a u128's bits (an i128 that wrapped); isize and usize are the host's 64 bits, as rustc builds.
    pub fn from_bits(self, v: i128) -> i128 {
        let bits = match self {
            IntTy::I8 | IntTy::U8 => 8,
            IntTy::I16 | IntTy::U16 => 16,
            IntTy::I32 | IntTy::U32 => 32,
            IntTy::I64 | IntTy::U64 | IntTy::Isize | IntTy::Usize => 64,
            IntTy::I128 | IntTy::U128 => return v,
        };
        let signed = matches!(self, IntTy::I8 | IntTy::I16 | IntTy::I32 | IntTy::I64 | IntTy::Isize);
        let low = v & ((1 << bits) - 1);
        if signed && low >= 1 << (bits - 1) {
            low - (1 << bits)
        } else {
            low
        }
    }
}

/// A type as MIR prints it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ty {
    Int(IntTy),
    Bool,
    Char,
    Str,
    Unit,
    Never,
    Tuple(Vec<Ty>),
    Array(Box<Ty>, ArrayLen),
    Slice(Box<Ty>),
    Ref {
        lifetime: Option<String>,
        mutable: bool,
        to: Box<Ty>,
    },
    Ptr {
        mutable: bool,
        to: Box<Ty>,
    },
    /// A named type: a struct, an enum, a type from core (`core::ops::Range<i16>`).
    Path(Path),
    Dyn(Path),
    /// A type the IR does not model (a closure's, a function pointer's), as rustc wrote it.
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ArrayLen {
    Known(u64),
    /// A const generic parameter, `[i16; N]`.
    Param(String),
}

/// A path: `Point`, `core::ops::Range::<i16>`, `<Rev<core::ops::Range<i16>> as Iterator>::next`,
/// `core::ptr::mut_ptr::<impl *mut u8>::add`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Path {
    pub qself: Option<Box<QSelf>>,
    pub segments: Vec<Segment>,
}

/// `<T as Trait>::`, or `<T>::` without a trait.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct QSelf {
    pub ty: Ty,
    pub as_trait: Option<Path>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Segment {
    Name {
        name: String,
        args: Option<GenericArgs>,
    },
    /// `<impl *mut u8>`: an inherent impl, named by its type.
    Impl(Ty),
}

/// `<i16>` in a type, `::<i16>` in an expression.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GenericArgs {
    pub turbofish: bool,
    pub args: Vec<GenericArg>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GenericArg {
    Ty(Ty),
    Lifetime(String),
    Const(u64),
}

impl Path {
    /// The last segment's name: `next` for `<Range<i16> as Iterator>::next`.
    pub fn name(&self) -> &str {
        match self.segments.last() {
            Some(Segment::Name { name, .. }) => name,
            _ => "",
        }
    }

    /// A path of names only: no `<T as Trait>::`, no generic arguments, no `<impl T>`.
    pub fn is_plain(&self) -> bool {
        self.qself.is_none()
            && self
                .segments
                .iter()
                .all(|s| matches!(s, Segment::Name { args: None, .. }))
    }

    /// Is it the plain path of these names (`["core", "intrinsics", "cold_path"]`)?
    pub fn is_path(&self, names: &[&str]) -> bool {
        self.is_plain()
            && self.segments.len() == names.len()
            && self
                .segments
                .iter()
                .zip(names)
                .all(|(s, n)| matches!(s, Segment::Name { name, .. } if name == n))
    }
}

/// A place: a local, or a callee's result cell, and the projections applied to it in order.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Place {
    pub base: PlaceBase,
    pub projection: Vec<Projection>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PlaceBase {
    Local(Local),
    /// `@f.N`: local `_N` of function `f`, read by the caller after `f` returns (a struct result's fields).
    Result {
        func: String,
        local: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Projection {
    Deref,
    Field(u32, Ty),
    Index(Local),
    /// `xs[i+k]`: an index and a constant folded into the address.
    IndexPlus(Local, u32),
    /// `xs[k of n]`: a constant index into an array of length n.
    ConstantIndex {
        offset: u64,
        min_length: u64,
    },
    /// `xs[(r.0: usize)]`: an index read from a struct's field, as the for-loop pass writes it, until the struct
    /// pass makes the field a local of its own.
    IndexPlace(Box<Place>),
    Downcast(String),
}

impl Place {
    pub fn local(l: Local) -> Place {
        Place {
            base: PlaceBase::Local(l),
            projection: vec![],
        }
    }

    /// The local the place starts from, if it is a local's.
    pub fn base_local(&self) -> Option<Local> {
        match self.base {
            PlaceBase::Local(l) => Some(l),
            PlaceBase::Result { .. } => None,
        }
    }

    /// The local itself, when the place is a whole local.
    pub fn as_local(&self) -> Option<Local> {
        if self.projection.is_empty() {
            self.base_local()
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Operand {
    Copy(Place),
    Move(Place),
    Constant(Const),
    /// A place read where an instruction names it directly: `out(xs[i])`.
    Place(Place),
    /// A struct value written out field by field, `Point { x: const 0_i16, y: const 2_i16 }`: a struct constant,
    /// inlined where it is used.
    Aggregate(Path, Vec<(String, Operand)>),
}

/// A constant operand, `const ...`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Const {
    Int {
        value: i128,
        ty: IntTy,
    },
    Bool(bool),
    Unit,
    /// A string literal, as written (escapes and all).
    Str(String),
    /// A named constant: `N`, `tables::SQUARES`, `i16::MIN`.
    Path(Path),
    /// A struct's value, field by field: `core::ops::Range::<i16> {{ start: 1_i16, end: 5_i16 }}`.
    Adt(Path, Vec<(String, Const)>),
    /// An enum's variant with its fields' values: `Result::<i16, i16>::Err(-9_i16)`.
    Variant(Path, Vec<Const>),
    /// A reference to a value rustc promoted out of a function into a constant of its own, `main::promoted[1]`:
    /// the function's path (`main`), and the promoted's number.
    Promoted(Path, u32),
}

impl Const {
    /// A constant of one word: an integer, a bool, or a named constant (`N`, `i16::MAX`).
    pub fn is_scalar(&self) -> bool {
        match self {
            Const::Int { .. } | Const::Bool(_) => true,
            Const::Path(p) => p.is_plain(),
            Const::Unit | Const::Str(_) | Const::Adt(..) | Const::Variant(..) | Const::Promoted(..) => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    AddWithOverflow,
    SubWithOverflow,
    MulWithOverflow,
    AddUnchecked,
    SubUnchecked,
    MulUnchecked,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Offset,
    /// Division of values known not to be negative.
    UDiv,
    URem,
    /// Rust's wrapping +, - and * on a u8 (with overflow checks off, or `wrapping_add` and its kin inlined), as the
    /// compiler marks it until the range pass shows the result stays within 0..=255 (see `ranges::mark_wrapping`).
    AddWrapping,
    SubWrapping,
    MulWrapping,
}

impl BinOp {
    pub const ALL: [BinOp; 28] = [
        BinOp::Add,
        BinOp::Sub,
        BinOp::Mul,
        BinOp::Div,
        BinOp::Rem,
        BinOp::AddWithOverflow,
        BinOp::SubWithOverflow,
        BinOp::MulWithOverflow,
        BinOp::AddUnchecked,
        BinOp::SubUnchecked,
        BinOp::MulUnchecked,
        BinOp::BitAnd,
        BinOp::BitOr,
        BinOp::BitXor,
        BinOp::Shl,
        BinOp::Shr,
        BinOp::Eq,
        BinOp::Ne,
        BinOp::Lt,
        BinOp::Le,
        BinOp::Gt,
        BinOp::Ge,
        BinOp::Offset,
        BinOp::UDiv,
        BinOp::URem,
        BinOp::AddWrapping,
        BinOp::SubWrapping,
        BinOp::MulWrapping,
    ];

    pub fn name(self) -> &'static str {
        match self {
            BinOp::Add => "Add",
            BinOp::Sub => "Sub",
            BinOp::Mul => "Mul",
            BinOp::Div => "Div",
            BinOp::Rem => "Rem",
            BinOp::AddWithOverflow => "AddWithOverflow",
            BinOp::SubWithOverflow => "SubWithOverflow",
            BinOp::MulWithOverflow => "MulWithOverflow",
            BinOp::AddUnchecked => "AddUnchecked",
            BinOp::SubUnchecked => "SubUnchecked",
            BinOp::MulUnchecked => "MulUnchecked",
            BinOp::BitAnd => "BitAnd",
            BinOp::BitOr => "BitOr",
            BinOp::BitXor => "BitXor",
            BinOp::Shl => "Shl",
            BinOp::Shr => "Shr",
            BinOp::Eq => "Eq",
            BinOp::Ne => "Ne",
            BinOp::Lt => "Lt",
            BinOp::Le => "Le",
            BinOp::Gt => "Gt",
            BinOp::Ge => "Ge",
            BinOp::Offset => "Offset",
            BinOp::UDiv => "UDiv",
            BinOp::URem => "URem",
            BinOp::AddWrapping => "AddWrapping",
            BinOp::SubWrapping => "SubWrapping",
            BinOp::MulWrapping => "MulWrapping",
        }
    }

    pub fn from_name(s: &str) -> Option<BinOp> {
        BinOp::ALL.into_iter().find(|o| o.name() == s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BorrowKind {
    Shared,
    Mut,
    RawConst,
    RawMut,
}

/// The sign of a term in a sum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sign {
    Plus,
    Minus,
}

/// A cast's kind as MIR names it: `IntToInt`, `PtrToPtr`, `PointerCoercion(Unsize, Implicit)`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CastKind {
    IntToInt,
    IntToFloat,
    FloatToInt,
    FloatToFloat,
    PtrToPtr,
    FnPtrToPtr,
    Transmute,
    Subtype,
    PointerExposeProvenance,
    PointerWithExposedProvenance,
    /// `PointerCoercion(Unsize, Implicit)`: a pointer to an array made a pointer to a slice.
    Unsize(CoercionSource),
    /// A kind the IR does not model (another pointer coercion, say), as rustc wrote it.
    Other(String),
}

impl CastKind {
    /// The kinds named by one word.
    const WORDS: [CastKind; 10] = [
        CastKind::IntToInt,
        CastKind::IntToFloat,
        CastKind::FloatToInt,
        CastKind::FloatToFloat,
        CastKind::PtrToPtr,
        CastKind::FnPtrToPtr,
        CastKind::Transmute,
        CastKind::Subtype,
        CastKind::PointerExposeProvenance,
        CastKind::PointerWithExposedProvenance,
    ];

    /// The one-word kind of this name.
    pub fn from_name(s: &str) -> Option<CastKind> {
        CastKind::WORDS.into_iter().find(|k| k.to_string() == s)
    }
}

impl fmt::Display for CastKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            CastKind::IntToInt => "IntToInt",
            CastKind::IntToFloat => "IntToFloat",
            CastKind::FloatToInt => "FloatToInt",
            CastKind::FloatToFloat => "FloatToFloat",
            CastKind::PtrToPtr => "PtrToPtr",
            CastKind::FnPtrToPtr => "FnPtrToPtr",
            CastKind::Transmute => "Transmute",
            CastKind::Subtype => "Subtype",
            CastKind::PointerExposeProvenance => "PointerExposeProvenance",
            CastKind::PointerWithExposedProvenance => "PointerWithExposedProvenance",
            CastKind::Unsize(source) => return write!(f, "PointerCoercion(Unsize, {source})"),
            CastKind::Other(text) => text,
        };
        f.write_str(name)
    }
}

/// Where a pointer coercion comes from: the compiler put it in, or the program wrote `as`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CoercionSource {
    Implicit,
    AsCast,
}

impl CoercionSource {
    pub fn from_name(s: &str) -> Option<CoercionSource> {
        match s {
            "Implicit" => Some(CoercionSource::Implicit),
            "AsCast" => Some(CoercionSource::AsCast),
            _ => None,
        }
    }
}

impl fmt::Display for CoercionSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            CoercionSource::Implicit => "Implicit",
            CoercionSource::AsCast => "AsCast",
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Rvalue {
    Use(Operand),
    BinaryOp(BinOp, Operand, Operand),
    UnaryOp(UnOp, Operand),
    /// `copy _1 as usize (IntToInt)`: the operand, the type, and the cast's kind.
    Cast(Operand, Ty, CastKind),
    Ref(BorrowKind, Place),
    Discriminant(Place),
    /// `[v; n]`
    Repeat(Operand, u64),
    /// `[a, b, c]`
    Array(Vec<Operand>),
    /// `(a, b)`
    Tuple(Vec<Operand>),
    /// `Point { x: a, y: b }`
    Struct(Path, Vec<(String, Operand)>),
    /// `Option::<i16>::Some(a)`: an enum variant built from its fields.
    Variant(Path, Vec<Operand>),
    /// An overflow-checked sum of signed terms, worked out in the accumulator and checked once at the end:
    /// `SumWithOverflow(+copy _1, -copy _2, +const 5_i16)`.
    Sum(Vec<(Sign, Operand)>),
    /// A linear combination with small coefficients, worked out in the accumulator: `Lin(+3*copy _1, -2*copy _2)`.
    Lin(Vec<(i64, Operand)>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Statement {
    Assign(Place, Rvalue),
    /// `assume(b)`: what `core::hint::assert_unchecked` becomes once inlined.
    Assume(Operand),
    StorageLive(Local),
    StorageDead(Local),
    Nop,
    Unsupported(Unsupported),
}

/// A line the IR does not represent: its text, and the locals it names.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Unsupported {
    pub text: String,
    pub locals: Vec<Local>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Unwind {
    Continue,
    Unreachable,
    Terminate(String),
    Cleanup(BasicBlock),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Terminator {
    Goto(BasicBlock),
    Return,
    Unreachable,
    /// `switchInt(v) -> [0: bb1, 1: bb2, otherwise: bb3]`. MIR prints an arm as its bit pattern (-1 of an i16 is
    /// 65535); the front end reads each as a value of v's type (`front::switch_values`).
    SwitchInt {
        discr: Operand,
        arms: Vec<(i128, BasicBlock)>,
        otherwise: Option<BasicBlock>,
    },
    /// `dest = func(args) -> [return: target, unwind ...]`; a call that never returns has no target.
    Call {
        dest: Place,
        func: Path,
        args: Vec<Operand>,
        target: Option<BasicBlock>,
        unwind: Unwind,
    },
    /// `assert(cond, "message", args) -> [success: target, unwind ...]`: panics unless cond is `expected`.
    Assert {
        cond: Operand,
        expected: bool,
        msg: String,
        msg_args: Vec<Operand>,
        target: BasicBlock,
        unwind: Unwind,
    },
    /// `drop(_4) -> [return: bb7, unwind ...]`: the value's drop glue run, then on to the target. Only a drop the
    /// passes know does nothing gets past them (`slices`, an array's `IntoIter`); any other is refused.
    Drop {
        place: Place,
        target: BasicBlock,
        unwind: Unwind,
    },
    Unsupported(Unsupported),
}

/// One line of a block, as the parser reads it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Line {
    Statement(Statement),
    Terminator(Terminator),
}

// ---- printing, in rustc's MIR syntax ----

fn comma_list<T: fmt::Display>(f: &mut fmt::Formatter, items: &[T]) -> fmt::Result {
    for (i, x) in items.iter().enumerate() {
        if i > 0 {
            f.write_str(", ")?;
        }
        write!(f, "{x}")?;
    }
    Ok(())
}

impl fmt::Display for Local {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if *self == Local::UNIT {
            f.write_str("_unit")
        } else {
            write!(f, "_{}", self.0)
        }
    }
}

impl fmt::Display for BasicBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

impl fmt::Display for IntTy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Ty::Int(t) => write!(f, "{t}"),
            Ty::Bool => f.write_str("bool"),
            Ty::Char => f.write_str("char"),
            Ty::Str => f.write_str("str"),
            Ty::Unit => f.write_str("()"),
            Ty::Never => f.write_str("!"),
            Ty::Tuple(ts) => {
                f.write_str("(")?;
                comma_list(f, ts)?;
                if ts.len() == 1 {
                    f.write_str(",")?;
                }
                f.write_str(")")
            }
            Ty::Array(t, n) => write!(f, "[{t}; {n}]"),
            Ty::Slice(t) => write!(f, "[{t}]"),
            Ty::Ref { lifetime, mutable, to } => {
                f.write_str("&")?;
                if let Some(l) = lifetime {
                    write!(f, "{l} ")?;
                }
                if *mutable {
                    f.write_str("mut ")?;
                }
                write!(f, "{to}")
            }
            Ty::Ptr { mutable, to } => write!(f, "*{} {to}", if *mutable { "mut" } else { "const" }),
            Ty::Path(p) => write!(f, "{p}"),
            Ty::Dyn(p) => write!(f, "dyn {p}"),
            Ty::Other(t) => f.write_str(t),
        }
    }
}

impl fmt::Display for ArrayLen {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ArrayLen::Known(n) => write!(f, "{n}"),
            ArrayLen::Param(p) => f.write_str(p),
        }
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(q) = &self.qself {
            match &q.as_trait {
                Some(t) => write!(f, "<{} as {t}>", q.ty)?,
                None => write!(f, "<{}>", q.ty)?,
            }
            for s in &self.segments {
                write!(f, "::{s}")?;
            }
            return Ok(());
        }
        for (i, s) in self.segments.iter().enumerate() {
            if i > 0 {
                f.write_str("::")?;
            }
            write!(f, "{s}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Segment::Name { name, args } => {
                f.write_str(name)?;
                if let Some(a) = args {
                    write!(f, "{a}")?;
                }
                Ok(())
            }
            Segment::Impl(t) => write!(f, "<impl {t}>"),
        }
    }
}

impl fmt::Display for GenericArgs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(if self.turbofish { "::<" } else { "<" })?;
        comma_list(f, &self.args)?;
        f.write_str(">")
    }
}

impl fmt::Display for GenericArg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GenericArg::Ty(t) => write!(f, "{t}"),
            GenericArg::Lifetime(l) => f.write_str(l),
            GenericArg::Const(n) => write!(f, "{n}"),
        }
    }
}

impl fmt::Display for Place {
    /// rustc's order: each projection wraps what came before it, `(*_1)`, `(_1.0: T)`, `(_1 as V)`, `_1[_2]`.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = match &self.base {
            PlaceBase::Local(l) => l.to_string(),
            PlaceBase::Result { func, local } => format!("@{func}.{local}"),
        };
        for p in &self.projection {
            s = match p {
                Projection::Deref => format!("(*{s})"),
                Projection::Field(k, t) => format!("({s}.{k}: {t})"),
                Projection::Index(i) => format!("{s}[{i}]"),
                Projection::IndexPlus(i, k) => format!("{s}[{i}+{k}]"),
                Projection::ConstantIndex { offset, min_length } => format!("{s}[{offset} of {min_length}]"),
                Projection::IndexPlace(p) => format!("{s}[{p}]"),
                Projection::Downcast(v) => format!("({s} as {v})"),
            };
        }
        f.write_str(&s)
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Operand::Copy(p) => write!(f, "copy {p}"),
            Operand::Move(p) => write!(f, "move {p}"),
            Operand::Constant(c) => write!(f, "const {c}"),
            Operand::Place(p) => write!(f, "{p}"),
            Operand::Aggregate(p, fields) => {
                write!(f, "{p} {{ ")?;
                for (i, (name, o)) in fields.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{name}: {o}")?;
                }
                f.write_str(" }")
            }
        }
    }
}

impl fmt::Display for Const {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Const::Int { value, ty } => write!(f, "{value}_{ty}"),
            Const::Bool(b) => write!(f, "{b}"),
            Const::Unit => f.write_str("()"),
            Const::Str(s) => write!(f, "\"{s}\""),
            Const::Path(p) => write!(f, "{p}"),
            Const::Adt(p, fields) => {
                write!(f, "{p} {{{{ ")?;
                for (i, (name, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{name}: {v}")?;
                }
                f.write_str(" }}")
            }
            Const::Variant(p, cs) => {
                write!(f, "{p}(")?;
                for (i, c) in cs.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{c}")?;
                }
                f.write_str(")")
            }
            Const::Promoted(p, n) => write!(f, "{p}::promoted[{n}]"),
        }
    }
}

impl fmt::Display for Sign {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Sign::Plus => "+",
            Sign::Minus => "-",
        })
    }
}

impl fmt::Display for Rvalue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Rvalue::Use(o) => write!(f, "{o}"),
            Rvalue::BinaryOp(op, a, b) => write!(f, "{}({a}, {b})", op.name()),
            Rvalue::UnaryOp(UnOp::Neg, a) => write!(f, "Neg({a})"),
            Rvalue::UnaryOp(UnOp::Not, a) => write!(f, "Not({a})"),
            Rvalue::Cast(o, t, kind) => write!(f, "{o} as {t} ({kind})"),
            Rvalue::Ref(k, p) => {
                let k = match k {
                    BorrowKind::Shared => "&",
                    BorrowKind::Mut => "&mut ",
                    BorrowKind::RawConst => "&raw const ",
                    BorrowKind::RawMut => "&raw mut ",
                };
                write!(f, "{k}{p}")
            }
            Rvalue::Discriminant(p) => write!(f, "discriminant({p})"),
            Rvalue::Repeat(o, n) => write!(f, "[{o}; {n}]"),
            Rvalue::Array(os) => {
                f.write_str("[")?;
                comma_list(f, os)?;
                f.write_str("]")
            }
            Rvalue::Tuple(os) => {
                f.write_str("(")?;
                comma_list(f, os)?;
                f.write_str(")")
            }
            Rvalue::Struct(p, fields) => {
                write!(f, "{p} {{ ")?;
                for (i, (name, o)) in fields.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{name}: {o}")?;
                }
                f.write_str(" }")
            }
            // a unit variant, `Shape::Dot`
            Rvalue::Variant(p, os) if os.is_empty() => write!(f, "{p}"),
            Rvalue::Variant(p, os) => {
                write!(f, "{p}(")?;
                comma_list(f, os)?;
                f.write_str(")")
            }
            Rvalue::Sum(terms) => {
                f.write_str("SumWithOverflow(")?;
                for (i, (s, o)) in terms.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{s}{o}")?;
                }
                f.write_str(")")
            }
            Rvalue::Lin(terms) => {
                f.write_str("Lin(")?;
                for (i, (k, o)) in terms.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{}{}*{o}", if *k < 0 { "-" } else { "+" }, k.abs())?;
                }
                f.write_str(")")
            }
        }
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Statement::Assign(p, r) => write!(f, "{p} = {r};"),
            Statement::Assume(o) => write!(f, "assume({o});"),
            Statement::StorageLive(l) => write!(f, "StorageLive({l});"),
            Statement::StorageDead(l) => write!(f, "StorageDead({l});"),
            Statement::Nop => f.write_str("nop;"),
            Statement::Unsupported(u) => f.write_str(&u.text),
        }
    }
}

impl fmt::Display for Unwind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Unwind::Continue => f.write_str("unwind continue"),
            Unwind::Unreachable => f.write_str("unwind unreachable"),
            Unwind::Terminate(why) => write!(f, "unwind terminate({why})"),
            Unwind::Cleanup(b) => write!(f, "unwind: {b}"),
        }
    }
}

impl fmt::Display for Terminator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Terminator::Goto(b) => write!(f, "goto -> {b};"),
            Terminator::Return => f.write_str("return;"),
            Terminator::Unreachable => f.write_str("unreachable;"),
            Terminator::SwitchInt { discr, arms, otherwise } => {
                write!(f, "switchInt({discr}) -> [")?;
                for (i, (v, b)) in arms.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{v}: {b}")?;
                }
                if let Some(o) = otherwise {
                    if !arms.is_empty() {
                        f.write_str(", ")?;
                    }
                    write!(f, "otherwise: {o}")?;
                }
                f.write_str("];")
            }
            Terminator::Call {
                dest,
                func,
                args,
                target,
                unwind,
            } => {
                write!(f, "{dest} = {func}(")?;
                comma_list(f, args)?;
                match target {
                    Some(t) => write!(f, ") -> [return: {t}, {unwind}];"),
                    None => write!(f, ") -> {unwind};"),
                }
            }
            Terminator::Assert {
                cond,
                expected,
                msg,
                msg_args,
                target,
                unwind,
            } => {
                write!(f, "assert({}{cond}, \"{msg}\"", if *expected { "" } else { "!" })?;
                for a in msg_args {
                    write!(f, ", {a}")?;
                }
                write!(f, ") -> [success: {target}, {unwind}];")
            }
            Terminator::Drop { place, target, unwind } => write!(f, "drop({place}) -> [return: {target}, {unwind}];"),
            Terminator::Unsupported(u) => f.write_str(&u.text),
        }
    }
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Line::Statement(s) => write!(f, "{s}"),
            Line::Terminator(t) => write!(f, "{t}"),
        }
    }
}
