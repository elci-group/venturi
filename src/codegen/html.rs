use super::{CodeWriter, DagContext, Emitter, EmitOutput};
use crate::error::Result;

pub struct HtmlEmitter;

impl Emitter for HtmlEmitter {
    fn emit(&self, ctx: &DagContext<'_>) -> Result<EmitOutput> {
        let mut w = CodeWriter::new(ctx.nodes.len() * 1024);

        w.line("<!DOCTYPE html>");
        w.line("<html lang=\"en\">");
        w.line("<head>");
        w.indent();
        w.line(r#"<meta charset="UTF-8">"#);
        w.line("<title>Venturi DAG</title>");
        w.line("<style>");
        emit_css(&mut w);
        w.line("</style>");
        w.dedent();
        w.line("</head>");
        w.line("<body>");
        w.indent();

        w.line("<h1>Venturi DAG Overview</h1>");

        for (name, vt) in &ctx.nodes {
            emit_node_html(&mut w, name, vt);
        }

        if !ctx.edges.is_empty() {
            w.line("<section id=\"wires\">");
            w.indent();
            w.line("<h2>Data Flow</h2>");
            w.line("<ul>");
            w.indent();
            for (from, to) in &ctx.edges {
                w.line(&format!(
                    r#"<li><a href="#{}">{}</a> → <a href="#{}">{}</a></li>"#,
                    from, from, to, to
                ));
            }
            w.dedent();
            w.line("</ul>");
            w.dedent();
            w.line("</section>");
        }

        w.dedent();
        w.line("</body>");
        w.line("</html>");

        Ok(vec![("index.html".to_string(), w.into_string())])
    }
}

fn escape_html(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '<' => "&lt;".chars().collect::<Vec<_>>(),
            '>' => "&gt;".chars().collect::<Vec<_>>(),
            '&' => "&amp;".chars().collect::<Vec<_>>(),
            '"' => "&quot;".chars().collect::<Vec<_>>(),
            c => vec![c],
        })
        .collect()
}

fn emit_css(w: &mut CodeWriter) {
    w.raw(
        r#"
body { font-family: monospace; margin: 2em; color: #333; background: #f9f9f9; }
h1 { border-bottom: 2px solid #0066cc; padding-bottom: 0.5em; }
h2 { margin-top: 1.5em; color: #0066cc; }
section { margin: 2em 0; padding: 1em; background: white; border: 1px solid #ddd; border-radius: 4px; }
table { border-collapse: collapse; width: 100%; }
th, td { border: 1px solid #ddd; padding: 0.5em; text-align: left; }
th { background: #f0f0f0; font-weight: bold; }
pre { background: #f5f5f5; padding: 1em; overflow-x: auto; border-left: 3px solid #0066cc; }
.kind { background: #e3f2fd; padding: 0.2em 0.5em; border-radius: 3px; font-size: 0.9em; }
a { color: #0066cc; text-decoration: none; }
a:hover { text-decoration: underline; }
ul { list-style: none; padding: 0; }
li { padding: 0.5em 0; }
"#,
    );
}

fn emit_node_html(w: &mut CodeWriter, name: &str, vt: &crate::ast::VtFile) {
    let kind = format!("{:?}", vt.kind);
    let safe_name = escape_html(name);

    w.line(&format!(
        r#"<section class="node" id="{}">"#,
        safe_name
    ));
    w.indent();

    w.line(&format!(
        r#"<h2>{} <span class="kind">{}</span></h2>"#,
        safe_name, escape_html(&kind)
    ));

    if !vt.inputs.is_empty() || !vt.outputs.is_empty() {
        w.line("<table>");
        w.indent();
        w.line("<thead><tr><th>Port</th><th>Type</th><th>Direction</th></tr></thead>");
        w.line("<tbody>");
        w.indent();

        for input in &vt.inputs {
            w.line(&format!(
                r#"<tr><td>{}</td><td>{}</td><td>input</td></tr>"#,
                escape_html(&input.name),
                escape_html(&input.ty.to_string())
            ));
        }

        for output in &vt.outputs {
            w.line(&format!(
                r#"<tr><td>{}</td><td>{}</td><td>output</td></tr>"#,
                escape_html(&output.name),
                escape_html(&output.ty.to_string())
            ));
        }

        w.dedent();
        w.line("</tbody>");
        w.dedent();
        w.line("</table>");
    }

    if let Some(func) = &vt.func {
        w.line("<h3>Logic</h3>");
        w.line("<pre>");
        w.raw(&format!("func {}():\n", escape_html(&func.name)));
        w.raw("  [function body]\n");
        w.line("</pre>");
    }

    w.dedent();
    w.line("</section>");
}
