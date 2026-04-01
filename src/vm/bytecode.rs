use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    List(Vec<Value>),
    Map(IndexMap<String, Value>),
    Null,
    Ok(Box<Value>),
    Err(String),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Str(s) => write!(f, "{}", s),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Map(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Null => write!(f, "null"),
            Value::Ok(v) => write!(f, "Ok({})", v),
            Value::Err(e) => write!(f, "Err({})", e),
        }
    }
}

impl Value {
    pub fn is_ok(&self) -> bool {
        matches!(self, Value::Ok(_))
    }

    pub fn is_err(&self) -> bool {
        matches!(self, Value::Err(_))
    }

    pub fn truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Int(0) => false,
            Value::Str(s) => !s.is_empty(),
            Value::Err(_) => false,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    // Stack manipulation
    Push(Value),
    Pop,
    Dup,

    // Variable access
    Load(String),
    Store(String),

    // Function calls
    Call(String, usize), // name, arg_count
    CallNode(String),    // call another DAG node by name
    Return,

    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,

    // Comparison
    Eq,
    Neq,
    Lt,
    Gt,

    // Control flow
    Jump(usize),
    JumpIfFalse(usize),
    Nop,

    // Error handling
    PushCatcher(String, usize), // error_type, handler_ip
    PopCatcher,
    Throw(String),

    // Result wrapping
    WrapOk,
    WrapErr,

    // Permissions
    CheckVan(String),

    // Logging
    Log,
}
