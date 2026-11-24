use crate::frontend::Span;

pub struct Node {
    pub kind: NodeKind,
    pub span: Span,
}

pub enum NodeKind {
    Program(Vec<Node>),
    Expr,
}

