use thiserror::Error;

#[derive(Error, Debug)]
pub enum VenturiError {
    #[error("Parse error at line {line}: {msg}")]
    Parse { line: usize, msg: String },

    #[error("DAG cycle detected involving node: {node}")]
    Cycle { node: String },

    #[error("Permission denied: VAN {required} required, got {got:?}")]
    Permission {
        required: String,
        got: Option<String>,
    },

    #[error("Pit error: {0}")]
    Pit(String),

    #[error("VM error: {0}")]
    Vm(String),

    #[error("VCBIN error: {0}")]
    VcBin(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("GUI error: {0}")]
    Gui(String),

    #[error("Codegen error: {0}")]
    Codegen(String),
}

pub type Result<T> = std::result::Result<T, VenturiError>;
