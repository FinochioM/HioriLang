use hiori_diagnostics::{Diagnostic, Span};
use hiori_lexer::{Token, TokenKind};

use crate::ast::{BinOp, Expr, Node, Program, Stmt};

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

    fn expect(&mut self, kind: &TokenKind, message: &str) -> Option<Span> {
        if self.peek_kind() == kind {
            let span = self.current_span();
            self.advance();
            Some(span)
        } else {
            let span = self.current_span();
            self.error(message, span);
            None
        }
    }

    pub fn finish(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    pub fn parse_program(&mut self) -> Program {
        let mut stmts = Vec::new();

        while !matches!(self.peek_kind(), TokenKind::Eof) {
            match self.parse_stmt() {
                Some(stmt) => stmts.push(stmt),
                None => {
                    self.synchronize();
                }
            }
        }

        Program { stmts }
    }

    fn synchronize(&mut self) {
        loop {
            match self.peek_kind() {
                TokenKind::Eof => break,
                TokenKind::Semicolon => {
                    self.advance();
                    break;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn parse_stmt(&mut self) -> Option<Node<Stmt>> {
        match self.peek_kind() {
            TokenKind::Let => self.parse_let_stmt(),
            _              => self.parse_expr_stmt(),
        }
    }

    fn parse_let_stmt(&mut self) -> Option<Node<Stmt>> {
        let let_span = self.current_span();
        self.advance();

        let (name, name_span) = match self.peek_kind().clone() {
            TokenKind::Ident(n) => {
                let span = self.current_span();
                self.advance();
                (n, span)
            }
            _ => {
                let span = self.current_span();
                self.error("expected identifier after 'let'", span);
                return None;
            }
        };

        self.expect(&TokenKind::Eq, "expected '=' after binding name")?;

        let value = self.parse_expr()?;

        self.expect(&TokenKind::Semicolon, "expected ';' after 'let' statement")?;

        let span = Span::new(let_span.start, value.span.end);
        Some(Node::new(
            Stmt::Let { name, name_span, value: Box::new(value) },
            span,
        ))
    }

    fn parse_expr_stmt(&mut self) -> Option<Node<Stmt>> {
        let expr = self.parse_expr()?;

        self.expect(&TokenKind::Semicolon, "expected ';' after expression")?;

        let span = expr.span.clone();
        Some(Node::new(Stmt::Expr(expr), span))
    }

    #[cfg(test)]
    fn parse(&mut self) -> Option<Node<Expr>> {
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
        let mut left = self.parse_unary()?;

        loop {
            let op = match self.peek_kind() {
                TokenKind::Star  => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            let span = Span::new(left.span.start, right.span.end);
            left = Node::new(
                Expr::Binary { op, left: Box::new(left), right: Box::new(right) },
                span,
            );
        }

        Some(left)
    }

    fn parse_unary(&mut self) -> Option<Node<Expr>> {
        if matches!(self.peek_kind(), TokenKind::Minus) {
            let minus_span = self.current_span();
            self.advance();
            let operand = self.parse_unary()?;
            let span = Span::new(minus_span.start, operand.span.end);
            Some(Node::new(Expr::Neg(Box::new(operand)), span))
        } else {
            self.parse_primary()
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use hiori_lexer::Lexer;

    fn parse_program(source: &str) -> (Program, Vec<Diagnostic>) {
        let (tokens, mut lex_diags) = Lexer::new(source).tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program();
        lex_diags.extend(parser.finish());
        (program, lex_diags)
    }

    fn parse_expr(source: &str) -> (Option<Node<Expr>>, Vec<Diagnostic>) {
        let (tokens, mut lex_diags) = Lexer::new(source).tokenize();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse();
        lex_diags.extend(parser.finish());
        (expr, lex_diags)
    }

    #[test]
    fn parse_integer() {
        let (expr, diags) = parse_expr("42");
        assert!(diags.is_empty());
        assert!(matches!(expr.unwrap().inner, Expr::Integer(42)));
    }

    #[test]
    fn parse_addition() {
        let (expr, diags) = parse_expr("1 + 2");
        assert!(diags.is_empty());
        assert!(matches!(expr.unwrap().inner, Expr::Binary { op: BinOp::Add, .. }));
    }

    #[test]
    fn operator_precedence() {
        let (expr, diags) = parse_expr("1 + 2 * 3");
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
        let (expr, diags) = parse_expr("(1 + 2) * 3");
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
        let (_, diags) = parse_expr("@");
        assert!(!diags.is_empty());
    }

    #[test]
    fn trailing_token_produces_diagnostic() {
        let (_, diags) = parse_expr("1 + 2 @");
        assert!(!diags.is_empty());
    }

    #[test]
    fn missing_closing_paren_produces_diagnostic() {
        let (_, diags) = parse_expr("(1 + 2");
        assert!(!diags.is_empty());
    }

    #[test]
    fn unexpected_eof_produces_diagnostic() {
        let (_, diags) = parse_expr("1 +");
        assert!(!diags.is_empty());
    }

    #[test]
    fn negate_integer() {
        let (expr, diags) = parse_expr("-1");
        assert!(diags.is_empty());
        assert!(matches!(expr.unwrap().inner, Expr::Neg(_)));
    }

    #[test]
    fn negate_ident() {
        let (expr, diags) = parse_expr("-x");
        assert!(diags.is_empty());
        assert!(matches!(expr.unwrap().inner, Expr::Neg(_)));
    }

    #[test]
    fn negate_parenthesized() {
        let (expr, diags) = parse_expr("-(1 + 2)");
        assert!(diags.is_empty());
        let node = expr.unwrap();
        match node.inner {
            Expr::Neg(operand) => {
                assert!(matches!(operand.inner, Expr::Binary { op: BinOp::Add, .. }));
            }
            _ => panic!("expected Neg at root"),
        }
    }

    #[test]
    fn double_negation() {
        let (expr, diags) = parse_expr("--1");
        assert!(diags.is_empty());
        let node = expr.unwrap();
        match node.inner {
            Expr::Neg(inner) => {
                assert!(matches!(inner.inner, Expr::Neg(_)));
            }
            _ => panic!("expected Neg at root"),
        }
    }

    #[test]
    fn unary_binds_tighter_than_multiply() {
        let (expr, diags) = parse_expr("-2 * 3");
        assert!(diags.is_empty());
        let node = expr.unwrap();
        match node.inner {
            Expr::Binary { op: BinOp::Mul, left, .. } => {
                assert!(matches!(left.inner, Expr::Neg(_)));
            }
            _ => panic!("expected Mul at root"),
        }
    }

    #[test]
    fn unary_in_right_operand_of_addition() {
        let (expr, diags) = parse_expr("1 + -2");
        assert!(diags.is_empty());
        let node = expr.unwrap();
        match node.inner {
            Expr::Binary { op: BinOp::Add, right, .. } => {
                assert!(matches!(right.inner, Expr::Neg(_)));
            }
            _ => panic!("expected Add at root"),
        }
    }

    #[test]
    fn binary_minus_then_unary_minus() {
        let (expr, diags) = parse_expr("1 - -1");
        assert!(diags.is_empty());
        let node = expr.unwrap();
        match node.inner {
            Expr::Binary { op: BinOp::Sub, right, .. } => {
                assert!(matches!(right.inner, Expr::Neg(_)));
            }
            _ => panic!("expected Sub at root"),
        }
    }

    #[test]
    fn unary_minus_span() {
        let (tokens, mut lex_diags) = Lexer::new("-42").tokenize();
        let mut parser = Parser::new(tokens);
        let node = parser.parse().unwrap();

        lex_diags.extend(parser.finish());
        assert!(lex_diags.is_empty());

        assert_eq!(node.span.start, 0);
        assert_eq!(node.span.end, 3);

        match node.inner {
            Expr::Neg(operand) => {
                assert_eq!(operand.span.start, 1);
                assert_eq!(operand.span.end, 3);
            }
            _ => panic!("expected Neg"),
        }
    }

    #[test]
    fn unary_minus_alone_produces_diagnostic() {
        let (expr, diags) = parse_expr("-");
        assert!(!diags.is_empty());
        assert!(expr.is_none());
    }

    #[test]
    fn empty_program() {
        let (program, diags) = parse_program("");
        assert!(diags.is_empty());
        assert!(program.stmts.is_empty());
    }

    #[test]
    fn let_integer() {
        let (program, diags) = parse_program("let x = 42;");
        assert!(diags.is_empty());
        assert_eq!(program.stmts.len(), 1);
        match &program.stmts[0].inner {
            Stmt::Let { name, value, .. } => {
                assert_eq!(name, "x");
                assert!(matches!(value.inner, Expr::Integer(42)));
            }
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn let_expression_value() {
        let (program, diags) = parse_program("let result = 1 + 2 * 3;");
        assert!(diags.is_empty());
        assert_eq!(program.stmts.len(), 1);
        match &program.stmts[0].inner {
            Stmt::Let { name, value, .. } => {
                assert_eq!(name, "result");
                assert!(matches!(value.inner, Expr::Binary { op: BinOp::Add, .. }));
            }
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn let_ident_value() {
        let (program, diags) = parse_program("let y = x;");
        assert!(diags.is_empty());
        match &program.stmts[0].inner {
            Stmt::Let { name, value, .. } => {
                assert_eq!(name, "y");
                assert!(matches!(&value.inner, Expr::Ident(n) if n == "x"));
            }
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn let_negated_value() {
        let (program, diags) = parse_program("let a = -1;");
        assert!(diags.is_empty());
        match &program.stmts[0].inner {
            Stmt::Let { value, .. } => {
                assert!(matches!(value.inner, Expr::Neg(_)));
            }
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn multiple_let_statements() {
        let (program, diags) = parse_program("let x = 1;\nlet y = 2;\nlet z = 3;");
        assert!(diags.is_empty());
        assert_eq!(program.stmts.len(), 3);
    }

    #[test]
    fn expr_statement() {
        let (program, diags) = parse_program("1 + 2;");
        assert!(diags.is_empty());
        assert_eq!(program.stmts.len(), 1);
        assert!(matches!(program.stmts[0].inner, Stmt::Expr(_)));
    }

    #[test]
    fn mixed_statements() {
        let (program, diags) = parse_program("let x = 10;\nx + 1;");
        assert!(diags.is_empty());
        assert_eq!(program.stmts.len(), 2);
        assert!(matches!(program.stmts[0].inner, Stmt::Let { .. }));
        assert!(matches!(program.stmts[1].inner, Stmt::Expr(_)));
    }

    #[test]
    fn let_missing_semicolon() {
        let (_, diags) = parse_program("let x = 42");
        assert!(!diags.is_empty());
        assert!(diags.iter().any(|d| d.message.contains(';')));
    }

    #[test]
    fn let_missing_eq() {
        let (_, diags) = parse_program("let x 42;");
        assert!(!diags.is_empty());
        assert!(diags.iter().any(|d| d.message.contains('=')));
    }

    #[test]
    fn let_missing_name() {
        let (_, diags) = parse_program("let = 42;");
        assert!(!diags.is_empty());
        assert!(diags.iter().any(|d| d.message.contains("identifier")));
    }

    #[test]
    fn let_missing_value() {
        let (_, diags) = parse_program("let x = ;");
        assert!(!diags.is_empty());
    }

    #[test]
    fn expr_statement_missing_semicolon() {
        let (_, diags) = parse_program("1 + 2");
        assert!(!diags.is_empty());
        assert!(diags.iter().any(|d| d.message.contains(';')));
    }

    #[test]
    fn let_as_value_is_rejected() {
        let (_, diags) = parse_program("let x = let y = 1;");
        assert!(!diags.is_empty());
    }

    #[test]
    fn integer_as_binding_name_is_rejected() {
        let (_, diags) = parse_program("let 42 = x;");
        assert!(!diags.is_empty());
        assert!(diags.iter().any(|d| d.message.contains("identifier")));
    }

    #[test]
    fn second_statement_parses_after_first_fails() {
        let (program, diags) = parse_program("let = 42;\nlet y = 1;");
        assert!(!diags.is_empty());
        assert_eq!(program.stmts.len(), 1);
        assert!(matches!(program.stmts[0].inner, Stmt::Let { .. }));
    }

    #[test]
    fn let_span_covers_full_statement() {
        let (program, diags) = parse_program("let x = 5;");
        assert!(diags.is_empty());
        let node = &program.stmts[0];
        assert_eq!(node.span.start, 0);
        assert_eq!(node.span.end, 9);
    }

    #[test]
    fn let_name_span_is_precise() {
        let (program, diags) = parse_program("let x = 5;");
        assert!(diags.is_empty());
        match &program.stmts[0].inner {
            Stmt::Let { name_span, .. } => {
                assert_eq!(name_span.start, 4);
                assert_eq!(name_span.end,   5);
            }
            _ => panic!("expected Let"),
        }
    }
}