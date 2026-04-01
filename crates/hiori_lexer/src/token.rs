use hiori_diagnostics::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Integer(i64),
    Ident(String),

    // Arithmetic operators
    Plus,
    Minus,
    Star,
    Slash,

    // Delimiters
    LParen,
    RParen,

    // Signals end of input — always the last token
    Eof,

    // A character the lexer does not recognize.
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