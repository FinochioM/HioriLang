use hiori_diagnostics::{Diagnostic, Span};
use crate::token::{Token, TokenKind};

pub struct Lexer<'src> {
    source: &'src str,
    chars: std::str::CharIndices<'src>,
    current: Option<(usize, char)>,
    diagnostics: Vec<Diagnostic>,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Self {
        let mut chars = source.char_indices();
        let current = chars.next();
        Self { source, chars, current, diagnostics: Vec::new() }
    }

    fn peek(&self) -> Option<char> {
        self.current.map(|(_, c)| c)
    }

    fn pos(&self) -> usize {
        self.current.map(|(i, _)| i).unwrap_or(self.source.len())
    }

    fn advance(&mut self) -> Option<(usize, char)> {
        let prev = self.current;
        self.current = self.chars.next();
        prev
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.advance();
        }
    }

    fn read_integer(&mut self, start: usize, first: char) -> Token {
        let mut text = String::from(first);

        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            text.push(self.advance().unwrap().1);
        }

        let end = self.pos();

        match text.parse::<i64>() {
            Ok(value) => Token::new(TokenKind::Integer(value), start, end),
            Err(_) => {
                self.diagnostics.push(Diagnostic::error(
                    format!("integer literal '{}' overflows i64", text),
                    Span::new(start, end),
                ));

                Token::new(TokenKind::Integer(0), start, end)
            }
        }
    }

    fn read_ident(&mut self, start: usize, first: char) -> Token {
        let mut text = String::from(first);

        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') {
            text.push(self.advance().unwrap().1);
        }

        Token::new(TokenKind::Ident(text), start, self.pos())
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        let Some((start, c)) = self.advance() else {
            let end = self.source.len();
            return Token::new(TokenKind::Eof, end, end);
        };

        match c {
            '+' => Token::new(TokenKind::Plus,   start, start + 1),
            '-' => Token::new(TokenKind::Minus,  start, start + 1),
            '*' => Token::new(TokenKind::Star,   start, start + 1),
            '/' => Token::new(TokenKind::Slash,  start, start + 1),
            '(' => Token::new(TokenKind::LParen, start, start + 1),
            ')' => Token::new(TokenKind::RParen, start, start + 1),

            c if c.is_ascii_digit()            => self.read_integer(start, c),
            c if c.is_alphabetic() || c == '_' => self.read_ident(start, c),

            c => {
                let end = start + c.len_utf8();
                self.diagnostics.push(Diagnostic::error(
                    format!("unknown character '{}'", c),
                    Span::new(start, end),
                ));
                Token::new(TokenKind::Unknown(c), start, end)
            }
        }
    }

    pub fn tokenize(mut self) -> (Vec<Token>, Vec<Diagnostic>) {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            let done = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if done {
                break;
            }
        }
        (tokens, self.diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::TokenKind;

    fn tokenize(source: &str) -> (Vec<TokenKind>, Vec<Diagnostic>) {
        let (tokens, diags) = Lexer::new(source).tokenize();
        let kinds = tokens.into_iter().map(|t| t.kind).collect();
        (kinds, diags)
    }

    fn kinds(source: &str) -> Vec<TokenKind> {
        tokenize(source).0
    }

    #[test]
    fn empty_source_gives_eof() {
        assert_eq!(kinds(""), vec![TokenKind::Eof]);
    }

    #[test]
    fn single_integer() {
        assert_eq!(kinds("42"), vec![TokenKind::Integer(42), TokenKind::Eof]);
    }

    #[test]
    fn operators() {
        assert_eq!(
            kinds("+ - * /"),
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn identifier() {
        assert_eq!(
            kinds("foo"),
            vec![TokenKind::Ident("foo".to_string()), TokenKind::Eof]
        );
    }

    #[test]
    fn expression_tokens() {
        assert_eq!(
            kinds("1 + 2 * 3"),
            vec![
                TokenKind::Integer(1),
                TokenKind::Plus,
                TokenKind::Integer(2),
                TokenKind::Star,
                TokenKind::Integer(3),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn spans_are_correct() {
        let (tokens, _) = Lexer::new("12 + 5").tokenize();
        assert_eq!(tokens[0].span.start, 0);
        assert_eq!(tokens[0].span.end, 2);
        assert_eq!(tokens[1].span.start, 3);
        assert_eq!(tokens[2].span.start, 5);
    }

    #[test]
    fn unknown_character_produces_diagnostic() {
        let (_, diags) = tokenize("@");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains('@'));
    }

    #[test]
    fn unknown_character_after_expression_produces_diagnostic() {
        let (_, diags) = tokenize("1 + 2 @");
        assert_eq!(diags.len(), 1);
    }
}