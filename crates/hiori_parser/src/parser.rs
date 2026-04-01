use hiori_diagnostics::{Diagnostic, Span};
use hiori_lexer::{Token, TokenKind};

use crate::ast::{BinOp, Expr, Node};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        assert!(!tokens.is_empty(), "token list must contain at least Eof");
        Self { tokens, pos: 0, diagnostics: Vec::new() }
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.tokens[self.pos.min(self.tokens.len() - 1)].kind
    }

    fn current_span(&self) -> Span {
        self.tokens[self.pos.min(self.tokens.len() - 1)].span.clone()
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos.min(self.tokens.len() - 1)].clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic::error(message, span));
    }

    pub fn parse(&mut self) -> Option<Node<Expr>> {
        let result = self.parse_expr();
        if !matches!(self.peek_kind(), TokenKind::Eof) {
            let span = self.current_span();
            self.error(
                format!("unexpected token after expression: {:?}", self.peek_kind()),
                span,
            );
        }
        result
    }

    fn parse_expr(&mut self) -> Option<Node<Expr>> {
        self.parse_additive()
    }

    fn parse_additive(&mut self) -> Option<Node<Expr>> {
        let mut left = self.parse_multiplicative()?;

        loop {
            let op = match self.peek_kind() {
                TokenKind::Plus  => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            let span = Span::new(left.span.start, right.span.end);
            left = Node::new(
                Expr::Binary { op, left: Box::new(left), right: Box::new(right) },
                span,
            );
        }

        Some(left)
    }

    fn parse_multiplicative(&mut self) -> Option<Node<Expr>> {
        let mut left = self.parse_primary()?;

        loop {
            let op = match self.peek_kind() {
                TokenKind::Star  => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                _ => break,
            };
            self.advance();
            let right = self.parse_primary()?;
            let span = Span::new(left.span.start, right.span.end);
            left = Node::new(
                Expr::Binary { op, left: Box::new(left), right: Box::new(right) },
                span,
            );
        }

        Some(left)
    }

    fn parse_primary(&mut self) -> Option<Node<Expr>> {
        let kind = self.peek_kind().clone();
        let span = self.current_span();

        match kind {
            TokenKind::Integer(n) => {
                self.advance();
                Some(Node::new(Expr::Integer(n), span))
            }

            TokenKind::Ident(name) => {
                self.advance();
                Some(Node::new(Expr::Ident(name), span))
            }

            TokenKind::LParen => {
                self.advance();
                let inner = self.parse_expr();
                if matches!(self.peek_kind(), TokenKind::RParen) {
                    self.advance();
                } else {
                    self.error("expected ')'", self.current_span());
                }
                inner
            }

            TokenKind::Unknown(_) => {
                self.advance();
                None
            }

            TokenKind::Eof => {
                self.error("unexpected end of input", span);
                None
            }

            other => {
                self.error(format!("unexpected token: {:?}", other), span);
                self.advance();
                None
            }
        }
    }

    pub fn finish(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hiori_lexer::Lexer;

    fn parse(source: &str) -> (Option<Node<Expr>>, Vec<Diagnostic>) {
        let (tokens, mut lex_diags) = Lexer::new(source).tokenize();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse();
        lex_diags.extend(parser.finish());
        (expr, lex_diags)
    }

    #[test]
    fn parse_integer() {
        let (expr, diags) = parse("42");
        assert!(diags.is_empty());
        assert!(matches!(expr.unwrap().inner, Expr::Integer(42)));
    }

    #[test]
    fn parse_addition() {
        let (expr, diags) = parse("1 + 2");
        assert!(diags.is_empty());
        assert!(matches!(expr.unwrap().inner, Expr::Binary { op: BinOp::Add, .. }));
    }

    #[test]
    fn operator_precedence() {
        let (expr, diags) = parse("1 + 2 * 3");
        assert!(diags.is_empty());
        let node = expr.unwrap();
        match node.inner {
            Expr::Binary { op: BinOp::Add, right, .. } => {
                assert!(matches!(right.inner, Expr::Binary { op: BinOp::Mul, .. }));
            }
            _ => panic!("expected Add at root"),
        }
    }

    #[test]
    fn parentheses_override_precedence() {
        let (expr, diags) = parse("(1 + 2) * 3");
        assert!(diags.is_empty());
        let node = expr.unwrap();
        match node.inner {
            Expr::Binary { op: BinOp::Mul, left, .. } => {
                assert!(matches!(left.inner, Expr::Binary { op: BinOp::Add, .. }));
            }
            _ => panic!("expected Mul at root"),
        }
    }

    #[test]
    fn unknown_character_produces_diagnostic() {
        let (_, diags) = parse("@");
        assert!(!diags.is_empty());
    }

    #[test]
    fn trailing_token_produces_diagnostic() {
        let (_, diags) = parse("1 + 2 @");
        assert!(!diags.is_empty());
    }

    #[test]
    fn missing_closing_paren_produces_diagnostic() {
        let (_, diags) = parse("(1 + 2");
        assert!(!diags.is_empty());
    }

    #[test]
    fn unexpected_eof_produces_diagnostic() {
        let (_, diags) = parse("1 +");
        assert!(!diags.is_empty());
    }
}