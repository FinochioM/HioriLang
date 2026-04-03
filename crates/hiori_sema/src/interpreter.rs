use std::collections::HashMap;
use std::fmt;
 
use hiori_diagnostics::Diagnostic;
use hiori_parser::{BinOp, Expr, Node, Program, Stmt};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
        }
    }
}

pub fn interpret(program: &Program) -> Result<(), Diagnostic> {
    let mut interp = Interpreter::new();

    interp.run_program(program)
}

struct Interpreter {
    env: HashMap<String, Value>,
}

impl Interpreter {
    fn new() -> Self {
        Self {
            env: HashMap::new()
        }
    }

    fn run_program(&mut self, program: &Program) -> Result<(), Diagnostic> {
        for stmt in &program.stmts {
            self.exec_stmt(stmt)?;
        }

        Ok(())
    }

    fn exec_stmt(&mut self, node: &Node<Stmt>) -> Result<(), Diagnostic> {
        match &node.inner {
            Stmt::Let { name, value, ..} => {
                let val = self.eval_expr(value)?;
                self.env.insert(name.clone(), val);
                Ok(())
            }

            Stmt::Expr(expr) => {
                let val = self.eval_expr(expr)?;
                println!("{}", val);
                Ok(())
            }
        }
    }

    fn eval_expr(&mut self, node: &Node<Expr>) -> Result<Value, Diagnostic> {
        match &node.inner {
            Expr::Integer(n) => Ok(Value::Int(*n)),

            Expr::Ident(name) => {
                Ok(self.env[name.as_str()].clone())
            }

            Expr::Neg(operand) => {
                let Value::Int(v) = self.eval_expr(operand)?;
                Ok(Value::Int(v.wrapping_neg()))
            }

            Expr::Binary { op, op_span, left, right } => {
                let Value::Int(l) = self.eval_expr(left)?;
                let Value::Int(r) = self.eval_expr(right)?;

                let result = match op {
                    BinOp::Add => l.wrapping_add(r),
                    BinOp::Sub => l.wrapping_sub(r),
                    BinOp::Mul => l.wrapping_mul(r),
                    BinOp::Div => {
                        if r == 0 {
                            return Err(Diagnostic::error(
                                "division by zero",
                                op_span.clone(),
                            ));
                        }

                        l.wrapping_div(r)
                    }
                };

                Ok(Value::Int(result))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hiori_lexer::Lexer;
    use hiori_parser::Parser;
 
    fn run(source: &str) -> Result<(), Diagnostic> {
        let (tokens, _) = Lexer::new(source).tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program();
 
        let resolver_diags = crate::resolve(&program);
        assert!(resolver_diags.is_empty(), "resolver errors: {:?}", resolver_diags);
 
        let type_diags = crate::type_check(&program);
        assert!(type_diags.is_empty(), "type errors: {:?}", type_diags);
 
        interpret(&program)
    }
 
    #[test]
    fn empty_program() {
        assert!(run("").is_ok());
    }
 
    #[test]
    fn let_only_program_is_ok() {
        assert!(run("let x = 42;").is_ok());
    }
 
    #[test]
    fn expr_statement_is_ok() {
        assert!(run("1 + 2;").is_ok());
    }
 
    #[test]
    fn let_then_expr_statement_is_ok() {
        assert!(run("let x = 10;\nx * 2;").is_ok());
    }
 
    #[test]
    fn multiple_expr_statements_are_ok() {
        assert!(run("let x = 10;\nx;\nx + 1;\nx * 2;").is_ok());
    }
 
    #[test]
    fn negation_is_ok() {
        assert!(run("let x = -5;").is_ok());
    }
 
    #[test]
    fn double_negation_is_ok() {
        assert!(run("let x = --5;").is_ok());
    }
 
    #[test]
    fn complex_expression_is_ok() {
        assert!(run("let a = 3;\nlet b = 4;\na * a + b * b;").is_ok());
    }
 
    #[test]
    fn nonzero_division_is_ok() {
        assert!(run("10 / 2;").is_ok());
    }
 
    #[test]
    fn long_binding_chain_is_ok() {
        assert!(run("let a = 1;\nlet b = a + 1;\nlet c = b + 1;\nc;").is_ok());
    }

    #[test]
    fn division_by_zero_literal_is_err() {
        let err = run("10 / 0;").unwrap_err();
        assert!(err.message.contains("division by zero"));
    }
 
    #[test]
    fn division_by_zero_via_binding_is_err() {
        let err = run("let x = 0;\n10 / x;").unwrap_err();
        assert!(err.message.contains("division by zero"));
    }
 
    #[test]
    fn division_by_zero_in_let_value_is_err() {
        let err = run("let x = 1 / 0;").unwrap_err();
        assert!(err.message.contains("division by zero"));
    }
 
    #[test]
    fn division_by_zero_stops_evaluation() {
        assert!(run("1 / 0;\n2 + 2;").is_err());
    }
 
    #[test]
    fn division_by_zero_span_is_operator_token() {
        let err = run("10 / 0;").unwrap_err();
        assert_eq!(err.span.start, 3);
        assert_eq!(err.span.end,   4);
    }
 
    #[test]
    fn division_by_zero_in_let_operator_span() {
        let err = run("let x = 1 / 0;").unwrap_err();
        assert_eq!(err.span.start, 10);
        assert_eq!(err.span.end,   11);
    }
 
    #[test]
    fn division_by_zero_via_binding_operator_span() {
        let err = run("let x = 0;\n10 / x;").unwrap_err();
        assert_eq!(err.span.start, 14);
        assert_eq!(err.span.end,   15);
    }
}