use std::collections::HashMap;
 
use hiori_diagnostics::{Diagnostic, Span};
use hiori_parser::{Expr, Node, Program, Stmt};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Type {
    Int,
}

pub fn type_check(program: &Program) -> Vec<Diagnostic> {
    let mut checker = TypeChecker::new();
    checker.check_program(program);
    checker.diagnostics
}

struct TypeChecker {
    env: HashMap<String, Type>,
    diagnostics: Vec<Diagnostic>,
}

impl TypeChecker {
    fn new() -> Self {
        Self {
            env: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic::error(message, span));
    }

    fn check_program(&mut self, program: &Program) {
        for stmt in &program.stmts {
            self.check_stmt(stmt);
        }
    }

    fn check_stmt(&mut self, node: &Node<Stmt>) {
        match &node.inner {
            Stmt::Let {name, value, ..} => {
                if let Some(ty) = self.check_expr(value) {
                    self.env.insert(name.clone(), ty);
                }
            }

            Stmt::Expr(expr) => {
                self.check_expr(expr);
            }
        }
    }

        fn check_expr(&mut self, node: &Node<Expr>) -> Option<Type> {
        match &node.inner {
            Expr::Integer(_) => Some(Type::Int),
 
            Expr::Ident(name) => {
                match self.env.get(name.as_str()).copied() {
                    Some(ty) => Some(ty),
                    None => {
                        self.error(
                            format!("internal: name '{}' not in type environment", name),
                            node.span.clone(),
                        );
                        None
                    }
                }
            }
 
            Expr::Neg(operand) => {
                let ty = self.check_expr(operand)?;
                if ty != Type::Int {
                    self.error(
                        format!("operator '-' requires Int, got {:?}", ty),
                        operand.span.clone(),
                    );
                    return None;
                }
                Some(Type::Int)
            }
 
            Expr::Binary { op, left, right , ..} => {
                let left_ty  = self.check_expr(left)?;
                let right_ty = self.check_expr(right)?;
 
                if left_ty != Type::Int {
                    self.error(
                        format!("operator '{:?}' requires Int on left, got {:?}", op, left_ty),
                        left.span.clone(),
                    );
                    return None;
                }
                if right_ty != Type::Int {
                    self.error(
                        format!("operator '{:?}' requires Int on right, got {:?}", op, right_ty),
                        right.span.clone(),
                    );
                    return None;
                }
 
                Some(Type::Int)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hiori_lexer::Lexer;
    use hiori_parser::Parser;
 
    fn type_check_source(source: &str) -> Vec<Diagnostic> {
        let (tokens, _) = Lexer::new(source).tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program();
        let resolver_diags = crate::resolve(&program);
        assert!(
            resolver_diags.is_empty(),
            "resolver errors in type_check test — fix the source: {:?}",
            resolver_diags
        );
        type_check(&program)
    }
 
    #[test]
    fn empty_program() {
        assert!(type_check_source("").is_empty());
    }
 
    #[test]
    fn single_integer_literal() {
        assert!(type_check_source("let x = 42;").is_empty());
    }
 
    #[test]
    fn negative_integer_literal() {
        assert!(type_check_source("let x = -1;").is_empty());
    }
 
    #[test]
    fn double_negation() {
        assert!(type_check_source("let x = --1;").is_empty());
    }
 
    #[test]
    fn binary_addition() {
        assert!(type_check_source("let x = 1 + 2;").is_empty());
    }
 
    #[test]
    fn binary_subtraction() {
        assert!(type_check_source("let x = 10 - 3;").is_empty());
    }
 
    #[test]
    fn binary_multiplication() {
        assert!(type_check_source("let x = 4 * 5;").is_empty());
    }
 
    #[test]
    fn binary_division() {
        assert!(type_check_source("let x = 10 / 2;").is_empty());
    }
 
    #[test]
    fn identifier_in_expression() {
        assert!(type_check_source("let x = 1;\nlet y = x;").is_empty());
    }
 
    #[test]
    fn identifier_in_binary() {
        assert!(type_check_source("let a = 1;\nlet b = 2;\nlet c = a + b;").is_empty());
    }
 
    #[test]
    fn identifier_negated() {
        assert!(type_check_source("let x = 5;\nlet y = -x;").is_empty());
    }
 
    #[test]
    fn complex_nested_expression() {
        assert!(type_check_source(
            "let a = 1;\nlet b = 2;\nlet c = (a + b) * -(a + 1);"
        ).is_empty());
    }
 
    #[test]
    fn expression_statement_literal() {
        assert!(type_check_source("1 + 2;").is_empty());
    }
 
    #[test]
    fn expression_statement_with_bound_name() {
        assert!(type_check_source("let x = 3;\nx + 1;").is_empty());
    }
 
    #[test]
    fn mixed_let_and_expr_statements() {
        assert!(type_check_source(
            "let x = 10;\nlet y = x + 5;\ny * 2;"
        ).is_empty());
    }
 
    #[test]
    fn long_binding_chain() {
        assert!(type_check_source(
            "let a = 1;\nlet b = a + 1;\nlet c = b + 1;\nlet d = c + 1;"
        ).is_empty());
    }
 
    #[test]
    fn precedence_and_parentheses() {
        assert!(type_check_source(
            "let x = (1 + 2) * (3 - 4) / -5;"
        ).is_empty());
    }
}