use crate::ast::VtFile;
use crate::error::{Result, VenturiError};

/// Resolved DAG context passed to all emitters.
/// Nodes are in topological order (sources first, sinks last).
pub struct DagContext<'a> {
    /// All nodes as (name, VtFile) pairs in topological order
    pub nodes: Vec<(&'a str, &'a VtFile)>,
    /// Edges as (from, to) pairs (raw from dag_wires)
    pub edges: Vec<(&'a str, &'a str)>,
}

impl<'a> DagContext<'a> {
    /// Returns upstream dependencies (nodes that must execute before `name`).
    pub fn deps_of(&self, name: &str) -> Vec<&'a str> {
        self.edges
            .iter()
            .filter_map(|(from, to)| {
                if *to == name {
                    Some(*from)
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Output of any emitter: list of (filename, content) pairs.
pub type EmitOutput = Vec<(String, String)>;

/// Trait for codegen backends.
pub trait Emitter {
    fn emit(&self, ctx: &DagContext<'_>) -> Result<EmitOutput>;
}

/// Indent-aware string builder for codegen output.
pub struct CodeWriter {
    buf: String,
    indent: usize,
}

impl CodeWriter {
    /// Create a new builder with estimated capacity.
    pub fn new(capacity: usize) -> Self {
        CodeWriter {
            buf: String::with_capacity(capacity),
            indent: 0,
        }
    }

    /// Increase indentation level.
    pub fn indent(&mut self) {
        self.indent += 1;
    }

    /// Decrease indentation level.
    pub fn dedent(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    /// Write a line with current indentation, followed by newline.
    pub fn line(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.buf.push_str("  ");
        }
        self.buf.push_str(s);
        self.buf.push('\n');
    }

    /// Write raw content with no indentation.
    pub fn raw(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    /// Write a blank line.
    pub fn blank(&mut self) {
        self.buf.push('\n');
    }

    /// Consume and return the built string.
    pub fn into_string(self) -> String {
        self.buf
    }
}

/// Dispatch to the appropriate emitter based on format string.
/// Uses static dispatch (no dyn Emitter).
pub fn emit_dag(ctx: &DagContext<'_>, format: &str) -> Result<EmitOutput> {
    match format {
        "xml" => xml::XmlEmitter.emit(ctx),
        "html" => html::HtmlEmitter.emit(ctx),
        "react" => react::ReactEmitter.emit(ctx),
        other => Err(VenturiError::Codegen(format!(
            "unknown format '{}': choose xml, html, or react",
            other
        ))),
    }
}

mod xml;
mod html;
mod react;
