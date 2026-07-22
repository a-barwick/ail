/// Half-open UTF-8 byte span in one source revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub const fn empty(at: usize) -> Self {
        Self { start: at, end: at }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Record,
    Variant,
    Fn,
    Let,
    Capability,
    Effects,
    If,
    Else,
    Match,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Whitespace,
    LineComment,
    BlockComment,
    Keyword(Keyword),
    Identifier,
    Text,
    Integer,
    LeftBrace,
    RightBrace,
    LeftParen,
    RightParen,
    Colon,
    Semicolon,
    Comma,
    Dot,
    Equal,
    Arrow,
    FatArrow,
    ColonColon,
    Invalid,
    Eof,
}

impl TokenKind {
    #[must_use]
    pub const fn is_trivia(&self) -> bool {
        matches!(
            self,
            Self::Whitespace | Self::LineComment | Self::BlockComment
        )
    }

    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Whitespace => "whitespace",
            Self::LineComment | Self::BlockComment => "comment",
            Self::Keyword(Keyword::Record) => "record",
            Self::Keyword(Keyword::Variant) => "variant",
            Self::Keyword(Keyword::Fn) => "fn",
            Self::Keyword(Keyword::Let) => "let",
            Self::Keyword(Keyword::Capability) => "capability",
            Self::Keyword(Keyword::Effects) => "effects",
            Self::Keyword(Keyword::If) => "if",
            Self::Keyword(Keyword::Else) => "else",
            Self::Keyword(Keyword::Match) => "match",
            Self::Identifier => "Identifier",
            Self::Text => "TextLiteral",
            Self::Integer => "IntLiteral",
            Self::LeftBrace => "{",
            Self::RightBrace => "}",
            Self::LeftParen => "(",
            Self::RightParen => ")",
            Self::Colon => ":",
            Self::Semicolon => ";",
            Self::Comma => ",",
            Self::Dot => ".",
            Self::Equal => "=",
            Self::Arrow => "->",
            Self::FatArrow => "=>",
            Self::ColonColon => "::",
            Self::Invalid => "invalid token",
            Self::Eof => "EOF",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub text: String,
}

/// Losslessly tokenize an AIL source unit.
#[must_use]
pub fn lex(source: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut cursor = 0;

    while cursor < source.len() {
        let start = cursor;
        let (kind, end) = scan_token(source, cursor);
        cursor = end;

        tokens.push(Token {
            kind,
            span: Span::new(start, cursor),
            text: source[start..cursor].to_owned(),
        });
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        span: Span::empty(source.len()),
        text: String::new(),
    });
    tokens
}

fn scan_token(source: &str, start: usize) -> (TokenKind, usize) {
    let rest = &source[start..];
    let Some(ch) = rest.chars().next() else {
        return (TokenKind::Eof, start);
    };
    let mut cursor = start;

    let kind = if ch.is_whitespace() {
        cursor += ch.len_utf8();
        while let Some(next) = source[cursor..].chars().next() {
            if !next.is_whitespace() {
                break;
            }
            cursor += next.len_utf8();
        }
        TokenKind::Whitespace
    } else if rest.starts_with("//") {
        cursor += 2;
        while cursor < source.len() && !source[cursor..].starts_with('\n') {
            let Some(next) = source[cursor..].chars().next() else {
                break;
            };
            cursor += next.len_utf8();
        }
        TokenKind::LineComment
    } else if rest.starts_with("/*") {
        cursor += 2;
        if let Some(offset) = source[cursor..].find("*/") {
            cursor += offset + 2;
        } else {
            cursor = source.len();
        }
        TokenKind::BlockComment
    } else if is_identifier_start(ch) {
        cursor += ch.len_utf8();
        while let Some(next) = source[cursor..].chars().next() {
            if !is_identifier_continue(next) {
                break;
            }
            cursor += next.len_utf8();
        }
        keyword_or_identifier(&source[start..cursor])
    } else if ch.is_ascii_digit() {
        cursor += ch.len_utf8();
        while cursor < source.len() && source.as_bytes()[cursor].is_ascii_digit() {
            cursor += 1;
        }
        TokenKind::Integer
    } else if ch == '"' {
        cursor += 1;
        let mut escaped = false;
        while let Some(next) = source[cursor..].chars().next() {
            cursor += next.len_utf8();
            if escaped {
                escaped = false;
            } else if next == '\\' {
                escaped = true;
            } else if next == '"' {
                break;
            }
        }
        TokenKind::Text
    } else if rest.starts_with("->") {
        cursor += 2;
        TokenKind::Arrow
    } else if rest.starts_with("=>") {
        cursor += 2;
        TokenKind::FatArrow
    } else if rest.starts_with("::") {
        cursor += 2;
        TokenKind::ColonColon
    } else {
        cursor += ch.len_utf8();
        match ch {
            '{' => TokenKind::LeftBrace,
            '}' => TokenKind::RightBrace,
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            ':' => TokenKind::Colon,
            ';' => TokenKind::Semicolon,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            '=' => TokenKind::Equal,
            _ => TokenKind::Invalid,
        }
    };
    (kind, cursor)
}

/// Reconstruct byte-identical source from the lossless token stream.
#[must_use]
pub fn reconstruct(tokens: &[Token]) -> String {
    tokens.iter().map(|token| token.text.as_str()).collect()
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

fn keyword_or_identifier(text: &str) -> TokenKind {
    match text {
        "record" => TokenKind::Keyword(Keyword::Record),
        "variant" => TokenKind::Keyword(Keyword::Variant),
        "fn" => TokenKind::Keyword(Keyword::Fn),
        "let" => TokenKind::Keyword(Keyword::Let),
        "capability" => TokenKind::Keyword(Keyword::Capability),
        "effects" => TokenKind::Keyword(Keyword::Effects),
        "if" => TokenKind::Keyword(Keyword::If),
        "else" => TokenKind::Keyword(Keyword::Else),
        "match" => TokenKind::Keyword(Keyword::Match),
        _ => TokenKind::Identifier,
    }
}
