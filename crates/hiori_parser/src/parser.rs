use hiori_diagnostics::{Diagnostic, Span};
use hiori_lexer::{Token, TokenKind};

use crate::ast::{BinOp, Block, CmpOp, Expr, Node, Program, Stmt};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        assert!(!tokens.is_empty(), "token list must contain at least Eof");
        Self {
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.tokens[self.pos.min(self.tokens.len() - 1)].kind
    }

    fn current_span(&self) -> Span {
        self.tokens[self.pos.min(self.tokens.len() - 1)]
            .span
            .clone()
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
                TokenKind::Eof | TokenKind::RBrace => break,
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
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::LBrace => self.parse_block_stmt(),
            _ => self.parse_expr_stmt(),
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
            Stmt::Let {
                name,
                name_span,
                value: Box::new(value),
            },
            span,
        ))
    }

    fn parse_if_stmt(&mut self) -> Option<Node<Stmt>> {
        let if_span = self.current_span();

        self.advance();

        let condition = self.parse_expr()?;
        let (then_block, mut end) = self.parse_block()?;

        let else_block = if matches!(self.peek_kind(), TokenKind::Else) {
            self.advance();
            let (block, else_end) = self.parse_block()?;
            end = else_end;
            Some(block)
        } else {
            None
        };

        let span = Span::new(if_span.start, end);
        Some(Node::new(
            Stmt::If {
                condition: Box::new(condition),
                then_block,
                else_block,
            },
            span,
        ))
    }

    fn parse_block(&mut self) -> Option<(Block, usize)> {
        if !matches!(self.peek_kind(), TokenKind::LBrace) {
            let span = self.current_span();
            self.error("expected '{'", span);
            return None;
        }

        self.advance();

        let mut stmts = Vec::new();

        while !matches!(self.peek_kind(), TokenKind::RBrace | TokenKind::Eof) {
            match self.parse_stmt() {
                Some(stmt) => stmts.push(stmt),
                None => self.synchronize(),
            }
        }

        if matches!(self.peek_kind(), TokenKind::RBrace) {
            let end = self.current_span().end;
            self.advance();
            Some((Block { stmts }, end))
        } else {
            let span = self.current_span();
            self.error("expected '}'", span);
            None
        }
    }

    fn parse_block_stmt(&mut self) -> Option<Node<Stmt>> {
        let start = self.current_span().start;
        let (block, end) = self.parse_block()?;

        Some(Node::new(Stmt::Block(block), Span::new(start, end)))
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
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Option<Node<Expr>> {
        let mut left = self.parse_additive()?;

        loop {
            let op = match self.peek_kind() {
                TokenKind::EqEq => CmpOp::Eq,
                TokenKind::BangEq => CmpOp::Ne,
                TokenKind::Lt => CmpOp::Lt,
                TokenKind::LtEq => CmpOp::Le,
                TokenKind::Gt => CmpOp::Gt,
                TokenKind::GtEq => CmpOp::Ge,
                _ => break,
            };

            let op_span = self.current_span();
            self.advance();
            let right = self.parse_additive()?;
            let span = Span::new(left.span.start, right.span.end);
            left = Node::new(
                Expr::Compare {
                    op,
                    op_span,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }

        Some(left)
    }

    fn parse_additive(&mut self) -> Option<Node<Expr>> {
        let mut left = self.parse_multiplicative()?;

        loop {
            let op = match self.peek_kind() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            let op_span = self.current_span();
            self.advance();
            let right = self.parse_multiplicative()?;
            let span = Span::new(left.span.start, right.span.end);
            left = Node::new(
                Expr::Binary {
                    op,
                    op_span,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }

        Some(left)
    }

    fn parse_multiplicative(&mut self) -> Option<Node<Expr>> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match self.peek_kind() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                _ => break,
            };
            let op_span = self.current_span();
            self.advance();
            let right = self.parse_unary()?;
            let span = Span::new(left.span.start, right.span.end);
            left = Node::new(
                Expr::Binary {
                    op,
                    op_span,
                    left: Box::new(left),
                    right: Box::new(right),
                },
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

            TokenKind::True => {
                self.advance();
                Some(Node::new(Expr::Bool(true), span))
            }

            TokenKind::False => {
                self.advance();
                Some(Node::new(Expr::Bool(false), span))
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
        assert!(matches!(
            expr.unwrap().inner,
            Expr::Binary { op: BinOp::Add, .. }
        ));
    }

    #[test]
    fn operator_precedence() {
        let (expr, diags) = parse_expr("1 + 2 * 3");
        assert!(diags.is_empty());
        let node = expr.unwrap();
        match node.inner {
            Expr::Binary {
                op: BinOp::Add,
                right,
                ..
            } => {
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
            Expr::Binary {
                op: BinOp::Mul,
                left,
                ..
            } => {
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
            Expr::Binary {
                op: BinOp::Mul,
                left,
                ..
            } => {
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
            Expr::Binary {
                op: BinOp::Add,
                right,
                ..
            } => {
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
            Expr::Binary {
                op: BinOp::Sub,
                right,
                ..
            } => {
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
                assert_eq!(name_span.end, 5);
            }
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn parse_true() {
        let (expr, diags) = parse_expr("true");
        assert!(diags.is_empty());
        assert!(matches!(expr.unwrap().inner, Expr::Bool(true)));
    }

    #[test]
    fn parse_false() {
        let (expr, diags) = parse_expr("false");
        assert!(diags.is_empty());
        assert!(matches!(expr.unwrap().inner, Expr::Bool(false)));
    }

    #[test]
    fn parse_less_than() {
        let (expr, diags) = parse_expr("1 < 2");
        assert!(diags.is_empty());
        assert!(matches!(
            expr.unwrap().inner,
            Expr::Compare { op: CmpOp::Lt, .. }
        ));
    }

    #[test]
    fn parse_equal_equal() {
        let (expr, diags) = parse_expr("1 == 2");
        assert!(diags.is_empty());
        assert!(matches!(
            expr.unwrap().inner,
            Expr::Compare { op: CmpOp::Eq, .. }
        ));
    }

    #[test]
    fn parse_not_equal() {
        let (expr, diags) = parse_expr("1 != 2");
        assert!(diags.is_empty());
        assert!(matches!(
            expr.unwrap().inner,
            Expr::Compare { op: CmpOp::Ne, .. }
        ));
    }

    #[test]
    fn parse_less_than_or_equal() {
        let (expr, diags) = parse_expr("1 <= 2");
        assert!(diags.is_empty());
        assert!(matches!(
            expr.unwrap().inner,
            Expr::Compare { op: CmpOp::Le, .. }
        ));
    }

    #[test]
    fn parse_greater_than() {
        let (expr, diags) = parse_expr("1 > 2");
        assert!(diags.is_empty());
        assert!(matches!(
            expr.unwrap().inner,
            Expr::Compare { op: CmpOp::Gt, .. }
        ));
    }

    #[test]
    fn parse_greater_than_or_equal() {
        let (expr, diags) = parse_expr("1 >= 2");
        assert!(diags.is_empty());
        assert!(matches!(
            expr.unwrap().inner,
            Expr::Compare { op: CmpOp::Ge, .. }
        ));
    }

    #[test]
    fn comparison_lower_precedence_than_addition() {
        let (expr, diags) = parse_expr("1 + 2 < 3 + 4");
        assert!(diags.is_empty());
        let node = expr.unwrap();
        match node.inner {
            Expr::Compare {
                op: CmpOp::Lt,
                ref left,
                ref right,
                ..
            } => {
                assert!(matches!(left.inner, Expr::Binary { op: BinOp::Add, .. }));
                assert!(matches!(right.inner, Expr::Binary { op: BinOp::Add, .. }));
            }
            _ => panic!("expected Compare at root"),
        }
    }

    #[test]
    fn chained_comparison_parses_left_associatively() {
        let (expr, diags) = parse_expr("1 < 2 < 3");
        assert!(diags.is_empty());
        let node = expr.unwrap();
        match node.inner {
            Expr::Compare {
                op: CmpOp::Lt,
                ref left,
                ..
            } => {
                assert!(matches!(left.inner, Expr::Compare { op: CmpOp::Lt, .. }));
            }
            _ => panic!("expected Compare at root"),
        }
    }

    #[test]
    fn let_with_bool_literal() {
        let (program, diags) = parse_program("let b = true;");
        assert!(diags.is_empty());
        assert_eq!(program.stmts.len(), 1);
        match &program.stmts[0].inner {
            Stmt::Let { name, value, .. } => {
                assert_eq!(name, "b");
                assert!(matches!(value.inner, Expr::Bool(true)));
            }
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn compare_span() {
        let (tokens, mut lex_diags) = Lexer::new("1 < 2").tokenize();
        let mut parser = Parser::new(tokens);
        let node = parser.parse().unwrap();
        lex_diags.extend(parser.finish());
        assert!(lex_diags.is_empty());
        assert_eq!(node.span.start, 0);
        assert_eq!(node.span.end, 5);
    }

    #[test]
    fn parse_if_empty_block() {
        let (program, diags) = parse_program("if true { }");
        assert!(diags.is_empty());
        assert_eq!(program.stmts.len(), 1);
        assert!(matches!(program.stmts[0].inner, Stmt::If { .. }));
    }

    #[test]
    fn parse_if_with_body() {
        let (program, diags) = parse_program("if true { 1; }");
        assert!(diags.is_empty());
        match &program.stmts[0].inner {
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                assert_eq!(then_block.stmts.len(), 1);
                assert!(else_block.is_none());
            }
            _ => panic!("expected If"),
        }
    }

    #[test]
    fn parse_if_else() {
        let (program, diags) = parse_program("if true { 1; } else { 2; }");
        assert!(diags.is_empty());
        match &program.stmts[0].inner {
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                assert_eq!(then_block.stmts.len(), 1);
                assert!(else_block.is_some());
                assert_eq!(else_block.as_ref().unwrap().stmts.len(), 1);
            }
            _ => panic!("expected If"),
        }
    }

    #[test]
    fn parse_if_condition_is_comparison() {
        let (program, diags) = parse_program("let x = 1;\nif x < 10 { }");
        assert!(diags.is_empty());
        match &program.stmts[1].inner {
            Stmt::If { condition, .. } => {
                assert!(matches!(condition.inner, Expr::Compare { .. }));
            }
            _ => panic!("expected If"),
        }
    }

    #[test]
    fn parse_nested_if() {
        let (program, diags) = parse_program("if true { if false { 1; } }");
        assert!(diags.is_empty());
        match &program.stmts[0].inner {
            Stmt::If { then_block, .. } => {
                assert_eq!(then_block.stmts.len(), 1);
                assert!(matches!(then_block.stmts[0].inner, Stmt::If { .. }));
            }
            _ => panic!("expected If"),
        }
    }

    #[test]
    fn parse_else_if_pattern() {
        let (program, diags) = parse_program("if true { 1; } else { if false { 2; } else { 3; } }");
        assert!(diags.is_empty());
        match &program.stmts[0].inner {
            Stmt::If { else_block, .. } => {
                let eb = else_block.as_ref().unwrap();
                assert_eq!(eb.stmts.len(), 1);
                assert!(matches!(eb.stmts[0].inner, Stmt::If { .. }));
            }
            _ => panic!("expected If"),
        }
    }

    #[test]
    fn parse_if_missing_lbrace_is_error() {
        let (_, diags) = parse_program("if true 1;");
        assert!(!diags.is_empty());
        assert!(diags.iter().any(|d| d.message.contains('{')));
    }

    #[test]
    fn parse_if_missing_rbrace_is_error() {
        let (_, diags) = parse_program("if true { 1;");
        assert!(!diags.is_empty());
        assert!(diags.iter().any(|d| d.message.contains('}')));
    }

    #[test]
    fn parse_if_no_condition_is_error() {
        let (_, diags) = parse_program("if { }");
        assert!(!diags.is_empty());
    }

    #[test]
    fn parse_if_span_covers_full_statement() {
        let (program, diags) = parse_program("if true { }");
        assert!(diags.is_empty());
        let node = &program.stmts[0];
        assert_eq!(node.span.start, 0);
        assert_eq!(node.span.end, 11);
    }

    #[test]
    fn parse_if_else_span_covers_else_block() {
        let (program, diags) = parse_program("if true { } else { }");
        assert!(diags.is_empty());
        let node = &program.stmts[0];
        assert_eq!(node.span.start, 0);
        assert_eq!(node.span.end, 20);
    }

    #[test]
    fn parse_error_inside_block_recovers() {
        let (program, diags) = parse_program("if true { let = 1; }\nlet x = 2;");
        assert!(!diags.is_empty());
        assert!(program.stmts.iter().any(|s| matches!(&s.inner,
            Stmt::Let { name, .. } if name == "x"
        )));
    }

    #[test]
    fn if_stmt_followed_by_let_stmt() {
        let (program, diags) = parse_program("if true { }\nlet x = 1;");
        assert!(diags.is_empty());
        assert_eq!(program.stmts.len(), 2);
        assert!(matches!(program.stmts[0].inner, Stmt::If { .. }));
        assert!(matches!(program.stmts[1].inner, Stmt::Let { .. }));
    }

    #[test]
    fn parse_empty_block_stmt() {
        let (program, diags) = parse_program("{ }");
        assert!(diags.is_empty());
        assert_eq!(program.stmts.len(), 1);
        assert!(matches!(program.stmts[0].inner, Stmt::Block(_)));
    }

    #[test]
    fn parse_block_stmt_with_body() {
        let (program, diags) = parse_program("{ 1; }");
        assert!(diags.is_empty());
        match &program.stmts[0].inner {
            Stmt::Block(block) => assert_eq!(block.stmts.len(), 1),
            _ => panic!("expected Block"),
        }
    }

    #[test]
    fn parse_block_stmt_followed_by_let() {
        let (program, diags) = parse_program("{ 1; }\nlet x = 2;");
        assert!(diags.is_empty());
        assert_eq!(program.stmts.len(), 2);
        assert!(matches!(program.stmts[0].inner, Stmt::Block(_)));
        assert!(matches!(program.stmts[1].inner, Stmt::Let { .. }));
    }

    #[test]
    fn parse_two_consecutive_block_stmts() {
        let (program, diags) = parse_program("{ 1; }\n{ 2; }");
        assert!(diags.is_empty());
        assert_eq!(program.stmts.len(), 2);
        assert!(matches!(program.stmts[0].inner, Stmt::Block(_)));
        assert!(matches!(program.stmts[1].inner, Stmt::Block(_)));
    }

    #[test]
    fn parse_nested_block_stmt() {
        let (program, diags) = parse_program("{ { 1; } }");
        assert!(diags.is_empty());
        match &program.stmts[0].inner {
            Stmt::Block(outer) => {
                assert_eq!(outer.stmts.len(), 1);
                assert!(matches!(outer.stmts[0].inner, Stmt::Block(_)));
            }
            _ => panic!("expected Block"),
        }
    }

    #[test]
    fn parse_block_stmt_span_covers_braces() {
        let (program, diags) = parse_program("{ 1; }");
        assert!(diags.is_empty());
        let node = &program.stmts[0];
        assert_eq!(node.span.start, 0);
        assert_eq!(node.span.end, 6);
    }

    #[test]
    fn parse_block_stmt_no_trailing_semicolon_needed() {
        let (program, diags) = parse_program("{ }\nlet a = 1;");
        assert!(diags.is_empty());
        assert_eq!(program.stmts.len(), 2);
    }

    #[test]
    fn parse_block_stmt_missing_rbrace_is_error() {
        let (_, diags) = parse_program("{ 1;");
        assert!(!diags.is_empty());
        assert!(diags.iter().any(|d| d.message.contains('}')));
    }

    #[test]
    fn parse_if_inside_block_stmt() {
        let (program, diags) = parse_program("{ if true { 1; } }");
        assert!(diags.is_empty());
        match &program.stmts[0].inner {
            Stmt::Block(block) => {
                assert_eq!(block.stmts.len(), 1);
                assert!(matches!(block.stmts[0].inner, Stmt::If { .. }));
            }
            _ => panic!("expected Block"),
        }
    }
}
