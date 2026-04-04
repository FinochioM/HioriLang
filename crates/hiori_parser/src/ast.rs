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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CmpOp {
    Eq, //==
    Ne, // !=
    Lt, //
    Le, // <=
    Gt, // >
    Ge, // >=
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Integer(i64),
    Bool(bool),
    Ident(String),
    Binary {
        op: BinOp,
        op_span: Span,
        left: Box<Node<Expr>>,
        right: Box<Node<Expr>>,
    },
    Compare {
        op: CmpOp,
        op_span: Span,
        left: Box<Node<Expr>>,
        right: Box<Node<Expr>>,
    },
    Neg(Box<Node<Expr>>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Node<Stmt>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        name: String,
        name_span: Span,
        value: Box<Node<Expr>>,
    },

    Expr(Node<Expr>),
    If {
        condition: Box<Node<Expr>>,
        then_block: Block,
        else_block: Option<Block>,
    },
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub stmts: Vec<Node<Stmt>>,
}
