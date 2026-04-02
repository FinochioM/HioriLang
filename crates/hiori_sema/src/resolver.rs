use std::collections::HashMap;

use hiori_diagnostics::{Diagnostic, Span};
use hiori_parser::{Expr, Node, Program, Stmt};

pub fn resolve(program: &Program) -> Vec<Diagnostic> {
    let mut resolver = Resolver::new();
    resolver.resolve_program(program);
    resolver.diagnostics
}

struct Resolver {
    defined: HashMap<String, Span>,
    diagnostics: Vec<Diagnostic>,
}

impl Resolver {
    fn new() -> Self {
        Self {
            defined: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic::error(message, span));
    }

    fn resolve_program(&mut self, program: &Program) {
        for stmt in &program.stmts {
            self.resolve_stmt(stmt);
        }
    }

    fn resolve_stmt(&mut self, node: &Node<Stmt>) {
        match &node.inner {
            Stmt::Let { name, name_span, value } => {
                self.resolve_expr(value);
                if let Some(prior_span) = self.defined.get(name) {
                    self.error(
                        format!(
                            "name '{}' is already defined (first defined at {}:{})",
                            name,
                            prior_span.start,
                            prior_span.end,
                        ),
                        name_span.clone(),
                    );
                } else {
                    self.defined.insert(name.clone(), name_span.clone());
                }
            }

            Stmt::Expr(expr) => {
                self.resolve_expr(expr);
            }
        }
    }

    fn resolve_expr(&mut self, node: &Node<Expr>) {
        match &node.inner {
            Expr::Ident(name) => {
                if !self.defined.contains_key(name.as_str()) {
                    self.error(
                        format!("undefined name '{}'", name),
                        node.span.clone(),
                    );
                }
            }

            Expr::Integer(_) => {}

            Expr::Binary { left, right, .. } => {
                self.resolve_expr(left);
                self.resolve_expr(right);
            }

            Expr::Neg(operand) => {
                self.resolve_expr(operand);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hiori_lexer::Lexer;
    use hiori_parser::Parser;

    fn resolve_source(source: &str) -> Vec<Diagnostic> {
        let (tokens, _) = Lexer::new(source).tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program();
        resolve(&program)
    }

    fn has_error(diags: &[Diagnostic], fragment: &str) -> bool {
        diags.iter().any(|d| d.message.contains(fragment))
    }

    #[test]
    fn empty_program_is_valid() {
        assert!(resolve_source("").is_empty());
    }

    #[test]
    fn let_literal_value_is_valid() {
        assert!(resolve_source("let x = 42;").is_empty());
    }

    #[test]
    fn let_then_use_is_valid() {
        assert!(resolve_source("let x = 1;\nlet y = x;").is_empty());
    }

    #[test]
    fn let_chain_is_valid() {
        assert!(resolve_source("let a = 1;\nlet b = 2;\nlet c = a + b;").is_empty());
    }

    #[test]
    fn expr_statement_using_bound_name_is_valid() {
        assert!(resolve_source("let x = 1;\nx + 1;").is_empty());
    }

    #[test]
    fn expr_statement_with_only_literal_is_valid() {
        assert!(resolve_source("1 + 2;").is_empty());
    }

    #[test]
    fn negated_literal_is_valid() {
        assert!(resolve_source("let x = -1;").is_empty());
    }

    #[test]
    fn negated_bound_name_is_valid() {
        assert!(resolve_source("let x = 1;\nlet y = -x;").is_empty());
    }

    #[test]
    fn complex_expr_with_all_bound_names_is_valid() {
        assert!(resolve_source(
            "let a = 1;\nlet b = 2;\nlet c = (a + b) * -(a + 1);"
        ).is_empty());
    }

    #[test]
    fn undefined_name_in_let_value_is_error() {
        let diags = resolve_source("let x = y;");
        assert!(has_error(&diags, "undefined name 'y'"));
    }

    #[test]
    fn undefined_name_in_expr_stmt_is_error() {
        let diags = resolve_source("x + 1;");
        assert!(has_error(&diags, "undefined name 'x'"));
    }

    #[test]
    fn two_undefined_names_both_reported() {
        let diags = resolve_source("x + 1;\ny + 2;");
        assert!(has_error(&diags, "undefined name 'x'"));
        assert!(has_error(&diags, "undefined name 'y'"));
    }

    #[test]
    fn undefined_name_in_binary_right_operand() {
        let diags = resolve_source("let a = 1;\nlet b = a + z;");
        assert!(has_error(&diags, "undefined name 'z'"));
    }

    #[test]
    fn undefined_name_inside_negation() {
        let diags = resolve_source("let x = -y;");
        assert!(has_error(&diags, "undefined name 'y'"));
    }

    #[test]
    fn self_reference_is_error() {
        let diags = resolve_source("let x = x;");
        assert!(has_error(&diags, "undefined name 'x'"));
    }

    #[test]
    fn self_reference_in_expression_is_error() {
        let diags = resolve_source("let x = x + 1;");
        assert!(has_error(&diags, "undefined name 'x'"));
    }

    #[test]
    fn forward_reference_is_error() {
        let diags = resolve_source("let x = y;\nlet y = 1;");
        assert!(has_error(&diags, "undefined name 'y'"));
    }

    #[test]
    fn forward_reference_in_expr_stmt_is_error() {
        let diags = resolve_source("x + 1;\nlet x = 5;");
        assert!(has_error(&diags, "undefined name 'x'"));
    }

    #[test]
    fn duplicate_binding_is_error() {
        let diags = resolve_source("let x = 1;\nlet x = 2;");
        assert!(has_error(&diags, "name 'x' is already defined"));
    }

    #[test]
    fn duplicate_binding_error_is_on_second_occurrence() {
        let diags = resolve_source("let x = 1;\nlet x = 2;");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].span.start, 15);
        assert_eq!(diags[0].span.end,   16);
    }

    #[test]
    fn duplicate_binding_after_valid_use_is_still_error() {
        let diags = resolve_source("let x = 1;\nlet y = x;\nlet x = 2;");
        assert!(has_error(&diags, "name 'x' is already defined"));
    }

    #[test]
    fn three_bindings_same_name_reports_two_errors() {
        let diags = resolve_source("let x = 1;\nlet x = 2;\nlet x = 3;");
        assert_eq!(diags.iter().filter(|d| d.message.contains("already defined")).count(), 2);
    }

    #[test]
    fn undefined_name_span_is_precise() {
        let diags = resolve_source("let x = y;");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].span.start, 8);
        assert_eq!(diags[0].span.end,   9);
    }

    #[test]
    fn undefined_name_in_expr_stmt_span_is_precise() {
        let diags = resolve_source("x + 1;");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].span.start, 0);
        assert_eq!(diags[0].span.end,   1);
    }
}