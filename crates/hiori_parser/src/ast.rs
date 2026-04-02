use hiori_diagnostics::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Node<T> {
    pub inner: T,
    pub span: Span,
}

impl<T> Node<T> {
    pub fn new(inner: T, span: Span) -> Self {
        Self { inner, span }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add, // +
    Sub, // -
    Mul, // *
    Div, // /
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Integer(i64),
    Ident(String),
    Binary {
        op: BinOp,
        left: Box<Node<Expr>>,
        right: Box<Node<Expr>>,
    },
    Neg(Box<Node<Expr>>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        name: String,
        value: Box<Node<Expr>>,
    },

    Expr(Node<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub stmts: Vec<Node<Stmt>>,
}