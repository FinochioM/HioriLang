use std::collections::HashMap;
 
use hiori_diagnostics::{Diagnostic, Span};
use hiori_parser::{Block, Expr, Node, Program, Stmt};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Type {
    Int,
    Bool,
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

            Stmt::If { condition, then_block, else_block } => {
                if let Some(ty) = self.check_expr(condition) {
                    if ty != Type::Bool { 
                        self.error(format!("'if' condition must be Bool, got {:?}", ty), condition.span.clone());
                    }
                }

                self.check_block(then_block);
                if let Some(block) = else_block {
                    self.check_block(block);
                }
            }
        }
    }

    fn check_block(&mut self, block: &Block) {
        let outer = self.env.clone();
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }

        self.env = outer;
    }

    fn check_expr(&mut self, node: &Node<Expr>) -> Option<Type> {
        match &node.inner {
            Expr::Integer(_) => Some(Type::Int),

            Expr::Bool(_) => Some(Type::Bool),
 
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

            Expr::Compare { op, left, right, .. } => {
                let left_ty = self.check_expr(left)?;
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

                Some(Type::Bool)
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

    #[test]
    fn bool_literal_true() {
        assert!(type_check_source("let b = true;").is_empty());
    }

    #[test]
    fn bool_literal_false() {
        assert!(type_check_source("let b = false;").is_empty());
    }

    #[test]
    fn comparison_int_int_produces_bool() {
        assert!(type_check_source("let b = 1 < 2;").is_empty());
        assert!(type_check_source("let b = 1 == 2;").is_empty());
        assert!(type_check_source("let b = 1 != 2;").is_empty());
        assert!(type_check_source("let b = 1 <= 2;").is_empty());
        assert!(type_check_source("let b = 1 >= 2;").is_empty());
        assert!(type_check_source("let b = 1 > 2;").is_empty());
    }

    #[test]
    fn comparison_with_bound_name_is_valid() {
        assert!(type_check_source("let x = 3;\nlet b = x < 10;").is_empty());
    }

    #[test]
    fn bool_stored_and_used_in_expr_stmt() {
        assert!(type_check_source("let b = true;\nb;").is_empty());
    }

    #[test]
    fn bool_in_binary_left_is_type_error() {
        let diags = type_check_source("let b = true;\nlet x = b + 1;");
        assert!(!diags.is_empty());
    }

    #[test]
    fn bool_in_binary_right_is_type_error() {
        let diags = type_check_source("let b = true;\nlet x = 1 + b;");
        assert!(!diags.is_empty());
    }

    #[test]
    fn neg_bool_is_type_error() {
        let diags = type_check_source("let b = true;\nlet x = -b;");
        assert!(!diags.is_empty());
    }

    #[test]
    fn chained_comparison_is_type_error() {
        let diags = type_check_source("let b = 1 < 2 < 3;");
        assert!(!diags.is_empty());
    }

    #[test]
    fn bool_as_compare_left_operand_is_type_error() {
        let diags = type_check_source("let b = true;\nlet x = b < 1;");
        assert!(!diags.is_empty());
    }

        #[test]
    fn if_bool_condition_is_ok() {
        assert!(type_check_source("if true { }").is_empty());
    }

    #[test]
    fn if_comparison_condition_is_ok() {
        assert!(type_check_source("if 1 < 2 { }").is_empty());
    }

    #[test]
    fn if_int_condition_is_type_error() {
        let diags = type_check_source("if 1 { }");
        assert!(!diags.is_empty());
        assert!(diags[0].message.contains("Bool"));
    }

    #[test]
    fn if_int_expr_condition_is_type_error() {
        let diags = type_check_source("if 1 + 2 { }");
        assert!(!diags.is_empty());
        assert!(diags[0].message.contains("Bool"));
    }

    #[test]
    fn if_else_bool_condition_is_ok() {
        assert!(type_check_source("if true { 1; } else { 2; }").is_empty());
    }

    #[test]
    fn if_block_binding_has_correct_type() {
        assert!(type_check_source("if true { let x = 1;\nx; }").is_empty());
    }

    #[test]
    fn if_block_binding_not_in_env_after_block() {
        assert!(type_check_source(
            "let a = 1;\nif true { let b = 2; }\na;"
        ).is_empty());
    }

    #[test]
    fn both_blocks_type_checked_independently() {
        assert!(type_check_source(
            "if 1 < 2 { let x = 1; } else { let y = 2; }"
        ).is_empty());
    }

    #[test]
    fn bool_value_in_then_block_is_ok() {
        assert!(type_check_source("if true { let b = 1 < 2;\nb; }").is_empty());
    }

    #[test]
    fn type_error_inside_block_is_reported() {
        let diags = type_check_source("if true { let b = true;\nb + 1; }");
        assert!(!diags.is_empty());
    }

    #[test]
    fn nested_if_type_checked() {
        assert!(type_check_source(
            "let x = 1;\nif true { if x < 2 { x; } }"
        ).is_empty());
    }
}