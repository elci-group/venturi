use super::{CodeWriter, DagContext, Emitter, EmitOutput};
use crate::error::Result;

pub struct XmlEmitter;

impl Emitter for XmlEmitter {
    fn emit(&self, ctx: &DagContext<'_>) -> Result<EmitOutput> {
        let mut w = CodeWriter::new(ctx.nodes.len() * 512);
        w.line(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        w.line("<dag>");
        w.indent();

        for (name, vt) in &ctx.nodes {
            emit_node_xml(&mut w, name, vt);
        }

        emit_wires_xml(&mut w, &ctx.edges);

        w.dedent();
        w.line("</dag>");

        Ok(vec![("dag.xml".to_string(), w.into_string())])
    }
}

fn escape_xml(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '&' => "&amp;".chars().collect::<Vec<_>>(),
            '<' => "&lt;".chars().collect::<Vec<_>>(),
            '>' => "&gt;".chars().collect::<Vec<_>>(),
            '"' => "&quot;".chars().collect::<Vec<_>>(),
            '\'' => "&apos;".chars().collect::<Vec<_>>(),
            c => vec![c],
        })
        .collect()
}

fn emit_node_xml(w: &mut CodeWriter, name: &str, vt: &crate::ast::VtFile) {
    let kind = format!("{:?}", vt.kind);
    let van = vt.van.as_deref().unwrap_or("none");

    w.line(&format!(
        r#"<node name="{}" kind="{}" van="{}">"#,
        escape_xml(name),
        escape_xml(&kind),
        escape_xml(van)
    ));
    w.indent();

    // Inputs
    w.line("<inputs>");
    w.indent();
    for input in &vt.inputs {
        let default_attr = if input.default.is_some() {
            r#" default="true""#
        } else {
            ""
        };
        w.line(&format!(
            r#"<port name="{}" type="{}"{}/>"#,
            escape_xml(&input.name),
            escape_xml(&input.ty.to_string()),
            default_attr
        ));
    }
    w.dedent();
    w.line("</inputs>");

    // Outputs
    w.line("<outputs>");
    w.indent();
    for output in &vt.outputs {
        w.line(&format!(
            r#"<port name="{}" type="{}"/>"#,
            escape_xml(&output.name),
            escape_xml(&output.ty.to_string())
        ));
    }
    w.dedent();
    w.line("</outputs>");

    // Func
    if let Some(func) = &vt.func {
        w.line(&format!(
            r#"<func name="{}" params="{}"/>"#,
            escape_xml(&func.name),
            escape_xml(&func.params.join(", "))
        ));
    } else {
        w.line(r#"<func/>"#);
    }

    // Metadata
    if !vt.meta.is_empty() {
        w.line("<meta>");
        w.indent();
        for (k, v) in &vt.meta {
            w.line(&format!(
                r#"<entry key="{}" value="{}"/>"#,
                escape_xml(k),
                escape_xml(v)
            ));
        }
        w.dedent();
        w.line("</meta>");
    }

    w.dedent();
    w.line("</node>");
}

fn emit_wires_xml(w: &mut CodeWriter, edges: &[(&str, &str)]) {
    if !edges.is_empty() {
        w.line("<wires>");
        w.indent();
        for (from, to) in edges {
            w.line(&format!(
                r#"<wire from="{}" to="{}"/>"#,
                escape_xml(from),
                escape_xml(to)
            ));
        }
        w.dedent();
        w.line("</wires>");
    }
}
