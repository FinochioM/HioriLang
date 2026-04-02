use hiori_diagnostics::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Integer(i64),
    Ident(String),
    Let,
    Plus,
    Minus,
    Star,
    Slash,
    Eq,
    LParen,
    RParen,
    Semicolon,
    Eof,
    Unknown(char),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, start: usize, end: usize) -> Self {
        Self {
            kind,
            span: Span::new(start, end),
        }
    }
}