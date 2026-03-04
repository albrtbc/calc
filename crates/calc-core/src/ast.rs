use crate::units::Unit;

#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    Ident(String),
    BinaryOp { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    UnaryNeg(Box<Expr>),
    UnaryPos(Box<Expr>),
    FuncCall { name: String, args: Vec<Expr> },
    Assign { name: String, value: Box<Expr> },
    TupleAssign { names: Vec<String>, values: Vec<Expr> },
    Convert { expr: Box<Expr>, target_unit: Unit },
    /// "N%" = N/100
    Percentage(Box<Expr>),
    /// "N% of X"
    PercentOf { percent: Box<Expr>, base: Box<Expr> },
    /// "X + N%"
    PercentAdd { base: Box<Expr>, percent: Box<Expr> },
    /// "X - N%"
    PercentSub { base: Box<Expr>, percent: Box<Expr> },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
}

#[derive(Debug, Clone)]
pub enum Line {
    Empty,
    Comment(String),
    Label(String),
    Expr(Expr),
    Error(String),
}
