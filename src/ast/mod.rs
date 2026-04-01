use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Plane,
    Vortex,
}

#[derive(Debug, Clone)]
pub struct VtFile {
    pub kind: NodeKind,
    pub van: Option<String>,
    pub meta: HashMap<String, String>,
    pub inputs: Vec<InputDecl>,
    pub outputs: Vec<OutputDecl>,
    pub uses: Vec<UseChass>,
    pub pits: Vec<PitRef>,
    pub func: Option<FuncDef>,
    pub dag_wires: Vec<DagWire>,
}

impl VtFile {
    pub fn new(kind: NodeKind) -> Self {
        VtFile {
            kind,
            van: None,
            meta: HashMap::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            uses: Vec::new(),
            pits: Vec::new(),
            func: None,
            dag_wires: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InputDecl {
    pub name: String,
    pub ty: VtType,
    pub default: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct OutputDecl {
    pub name: String,
    pub ty: VtType,
}

#[derive(Debug, Clone)]
pub struct UseChass {
    pub path: String,
    pub alias: String,
}

#[derive(Debug, Clone)]
pub struct PitRef {
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct FuncDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct DagWire {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VtType {
    Int,
    Float,
    Bool,
    Str,
    DataFrame,
    Custom(String),
}

impl std::fmt::Display for VtType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VtType::Int => write!(f, "Int"),
            VtType::Float => write!(f, "Float"),
            VtType::Bool => write!(f, "Bool"),
            VtType::Str => write!(f, "String"),
            VtType::DataFrame => write!(f, "DataFrame"),
            VtType::Custom(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Assign(String, Expr),
    Return(Expr),
    TryCatch {
        body: Vec<Stmt>,
        catches: Vec<CatchClause>,
    },
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub struct CatchClause {
    pub error_type: String,
    pub binding: String,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Ident(String),
    IntLit(i64),
    FloatLit(f64),
    StrLit(String),
    BoolLit(bool),
    Call(String, Vec<Expr>),
    FieldAccess(Box<Expr>, String),
    ResultOk(Box<Expr>),
    ResultErr(Box<Expr>),
}
