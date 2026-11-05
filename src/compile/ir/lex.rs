//! Tokens of MIR text.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tok {
    /// A name: `_5`, `bb3`, `copy`, `core`, `i16`.
    Ident(String),
    /// An integer, with its type suffix if it has one: `5_i16`, `16`.
    Int { digits: String, suffix: Option<String> },
    /// A string literal's text, between the quotes, as written.
    Str(String),
    /// `'_`, `'static`
    Lifetime(String),
    /// Punctuation: `( ) [ ] { } < > , ; : :: = -> & * ! . + - @ #`
    Punct(&'static str),
    /// A closure's type, as rustc writes it, whatever its file's path holds: `{closure@src/main.rs:10:25: 10:28}`
    Closure(String),
}

/// A token and the bytes of the line it came from.
#[derive(Clone, Debug)]
pub struct Token {
    pub tok: Tok,
    pub start: usize,
    pub end: usize,
}

const PUNCT: [&str; 22] = [
    "::", "->", "(", ")", "[", "]", "{", "}", "<", ">", ",", ";", ":", "=", "&", "*", "!", ".", "+", "-", "@", "#",
];

/// The tokens of one line, or the byte where a character no token starts with appears.
pub fn tokens(text: &str) -> Result<Vec<Token>, usize> {
    let b = text.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let start = i;
        let tok = if c.is_ascii_whitespace() {
            i += 1;
            continue;
        } else if b[i..].starts_with(b"{closure@") {
            let Some(end) = text[i..].find('}') else {
                return Err(start);
            };
            i += end + 1;
            Tok::Closure(text[start..i].to_string())
        } else if c.is_ascii_alphabetic() || c == b'_' {
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            Tok::Ident(text[start..i].to_string())
        } else if c.is_ascii_digit() {
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            let digits = text[start..i].to_string();
            // a type suffix follows an underscore: 5_i16
            let mut suffix = None;
            if i + 1 < b.len() && b[i] == b'_' && b[i + 1].is_ascii_alphabetic() {
                let s = i + 1;
                i = s;
                while i < b.len() && b[i].is_ascii_alphanumeric() {
                    i += 1;
                }
                suffix = Some(text[s..i].to_string());
            }
            Tok::Int { digits, suffix }
        } else if c == b'"' {
            i += 1;
            while i < b.len() && b[i] != b'"' {
                if b[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            if i >= b.len() {
                return Err(start);
            }
            i += 1;
            Tok::Str(text[start + 1..i - 1].to_string())
        } else if c == b'\'' {
            i += 1;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            Tok::Lifetime(text[start..i].to_string())
        } else if let Some(p) = PUNCT.iter().find(|p| b[i..].starts_with(p.as_bytes())) {
            i += p.len();
            Tok::Punct(p)
        } else {
            return Err(i);
        };
        out.push(Token { tok, start, end: i });
    }
    Ok(out)
}
