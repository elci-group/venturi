mod ast;
mod codegen;
mod compiler;
mod error;
mod graph;
mod gui;
mod lexer;
mod parser;
mod permissions;
mod pits;
mod runtime;
mod validator;
mod vcbin;
mod vm;

use clap::{Parser, Subcommand};
use error::VenturiError;
use runtime::Runtime;
use std::path::{Path, PathBuf};

/// Venturi — a DAG-native programming language runtime
#[derive(Parser)]
#[command(name = "venturi", version = "0.1.0", about = "Venturi DAG language runtime")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Parse and execute a single .vt file
    Run {
        file: PathBuf,
    },

    /// Load all .vt files in a directory, build the DAG, and execute
    RunDag {
        dir: PathBuf,
    },

    /// Compile a directory of .vt files into a .vcbin binary
    Chass {
        dir: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, default_value = "plane")]
        mode: String,
        #[arg(long)]
        name: Option<String>,
    },

    /// Inspect or verify a .vcbin file
    #[command(subcommand)]
    Vcbin(VcbinCmd),

    /// Manage hot-update pits
    #[command(subcommand)]
    Pit(PitCmd),

    /// Validate a .vt file (parse + check, no execution)
    Validate {
        file: PathBuf,
    },

    /// Print the DAG topology for a directory of .vt files
    Graph {
        dir: PathBuf,
    },

    /// Launch the VenturiCards GUI from a gui-app DAG directory
    Gui {
        /// Directory containing the .vt GUI files
        #[arg(long, default_value = "gui-app")]
        dir: PathBuf,
        /// Rendering backend: egui (default) or iced
        #[arg(long, default_value = "egui")]
        backend: gui::Backend,
    },

    /// Emit a DAG as HTML, XML, or React/TSX
    Emit {
        /// Directory containing .vt files
        #[arg(long, default_value = "gui-app")]
        dir: PathBuf,
        /// Output format: html, xml, or react
        #[arg(long)]
        format: String,
        /// Output directory
        #[arg(short, long, default_value = "out")]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum VcbinCmd {
    /// Print metadata and interface of a .vcbin file
    Info { file: PathBuf },
    /// Verify the hash integrity of a .vcbin file
    Verify { file: PathBuf },
}

#[derive(Subcommand)]
enum PitCmd {
    /// Create a new pit bound to a .vcbin file
    Create {
        name: String,
        vcbin: String,
        #[arg(long, default_value = "@system")]
        van: String,
    },
    /// Update a pit to a new .vcbin version
    Update { name: String, vcbin: String },
    /// Rollback a pit to a prior version
    Rollback {
        name: String,
        #[arg(long)]
        version: usize,
    },
    /// Show pit status
    Status { name: String },
    /// List all pits
    List,
}

fn pit_store_path() -> PathBuf {
    dirs_next()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("pits.json")
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".venturi"))
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> error::Result<()> {
    match cli.command {
        Cmd::Run { file } => cmd_run(&file),
        Cmd::RunDag { dir } => cmd_run_dag(&dir),
        Cmd::Chass { dir, output, mode, name } => cmd_chass(&dir, &output, &mode, name),
        Cmd::Vcbin(sub) => match sub {
            VcbinCmd::Info { file } => cmd_vcbin_info(&file),
            VcbinCmd::Verify { file } => cmd_vcbin_verify(&file),
        },
        Cmd::Pit(sub) => {
            let store_path = pit_store_path();
            let mut pits = pits::PitStore::load(&store_path)?;
            match sub {
                PitCmd::Create { name, vcbin, van } => {
                    pits.create(&name, &vcbin, &van, vec![])?;
                    println!("Pit '{}' created at version 1", name);
                }
                PitCmd::Update { name, vcbin } => {
                    pits.update(&name, &vcbin)?;
                    let entry = pits.status(&name)?;
                    println!("Pit '{}' updated to version {}", name, entry.active_version);
                }
                PitCmd::Rollback { name, version } => {
                    pits.rollback(&name, version)?;
                    println!("Pit '{}' rolled back to version {}", name, version);
                }
                PitCmd::Status { name } => {
                    let entry = pits.status(&name)?;
                    println!("{}", serde_json::to_string_pretty(entry)?);
                }
                PitCmd::List => {
                    let list = pits.list();
                    if list.is_empty() {
                        println!("No pits registered.");
                    } else {
                        for entry in list {
                            println!(
                                "  {} (v{}) — {}",
                                entry.name,
                                entry.active_version,
                                entry
                                    .active()
                                    .map(|v| v.vcbin_path.as_str())
                                    .unwrap_or("no active version")
                            );
                        }
                    }
                }
            }
            Ok(())
        }
        Cmd::Validate { file } => cmd_validate(&file),
        Cmd::Graph { dir } => cmd_graph(&dir),
        Cmd::Gui { dir, backend } => cmd_gui(&dir, backend),
        Cmd::Emit { dir, format, output } => cmd_emit(&dir, &format, &output),
    }
}

fn cmd_run(file: &Path) -> error::Result<()> {
    let source = std::fs::read_to_string(file)?;
    let tokens = lexer::tokenize(&source)?;
    let vt = parser::parse(tokens)?;

    let validator = validator::Validator::new();
    validator.validate_file(&vt)?;

    println!(
        "Running: {} [{:?}] VAN: {}",
        file.display(),
        vt.kind,
        vt.van.as_deref().unwrap_or("(none)")
    );

    let func = match &vt.func {
        Some(f) => f,
        None => {
            println!("No function defined in file.");
            return Ok(());
        }
    };

    let bytecode = compiler::compile(func);
    let mut ctx = vm::ExecContext::new(vt.van.clone());

    // Populate input defaults
    for input in &vt.inputs {
        if let Some(default) = &input.default {
            let val = eval_const_expr(default);
            ctx.variables.insert(input.name.clone(), val);
        }
    }

    let vm = vm::Vm::new();
    let result = vm.execute(&bytecode, &mut ctx)?;

    println!("Result: {}", result);
    Ok(())
}

fn cmd_run_dag(dir: &Path) -> error::Result<()> {
    let store_path = pit_store_path();
    let mut runtime = Runtime::new(&store_path)?;

    let entries = std::fs::read_dir(dir)?;
    let mut vt_files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("vt"))
        .collect();

    vt_files.sort();

    if vt_files.is_empty() {
        println!("No .vt files found in {}", dir.display());
        return Ok(());
    }

    println!("Loading {} .vt files...", vt_files.len());
    for vt_file in &vt_files {
        let id = runtime.load_vt_file(vt_file)?;
        println!("  [{}] {}", id, vt_file.display());
    }

    println!("Executing DAG...");
    let results = runtime.execute_dag()?;

    println!("\nDAG execution complete. Node outputs:");
    for (name, value) in &results {
        println!("  {} => {}", name, value);
    }

    Ok(())
}

fn cmd_chass(dir: &Path, output: &Path, mode: &str, name: Option<String>) -> error::Result<()> {
    use vcbin::{GraphEdge, PortDef, VcBin, VcBinGraph, VcBinInterface, VcBinPermissions};

    let entries = std::fs::read_dir(dir)?;
    let vt_files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("vt"))
        .collect();

    if vt_files.is_empty() {
        return Err(VenturiError::VcBin(format!(
            "No .vt files found in {}",
            dir.display()
        )));
    }

    let chass_name = name.unwrap_or_else(|| {
        dir.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed")
            .to_string()
    });

    // Load and compile all .vt files
    let mut all_inputs = Vec::new();
    let mut all_outputs = Vec::new();
    let mut all_van: Vec<String> = Vec::new();
    let mut all_bytecode = Vec::new();
    let mut node_names = Vec::new();
    let mut edges: Vec<GraphEdge> = Vec::new();

    for vt_file in &vt_files {
        let source = std::fs::read_to_string(vt_file)?;
        let tokens = lexer::tokenize(&source)?;
        let vt = parser::parse(tokens)?;

        let fname = vt_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("node")
            .to_string();

        node_names.push(fname.clone());

        for input in &vt.inputs {
            all_inputs.push(PortDef {
                name: input.name.clone(),
                ty: format!("{}", input.ty),
            });
        }

        for output in &vt.outputs {
            all_outputs.push(PortDef {
                name: output.name.clone(),
                ty: format!("{}", output.ty),
            });
        }

        if let Some(van) = &vt.van {
            all_van.push(van.clone());
        }

        for wire in &vt.dag_wires {
            edges.push(GraphEdge {
                from: wire.from.clone(),
                to: wire.to.clone(),
            });
        }

        if let Some(func) = &vt.func {
            let bytecode = compiler::compile(func);
            all_bytecode.extend(bytecode);
        }
    }

    let first_node = node_names.first().cloned().unwrap_or_default();
    let last_node = node_names.last().cloned().unwrap_or_default();

    let vcbin = VcBin::new(
        chass_name.clone(),
        mode.to_string(),
        VcBinInterface {
            inputs: all_inputs,
            outputs: all_outputs,
        },
        VcBinPermissions {
            allowed_vans: all_van,
        },
        VcBinGraph {
            nodes: node_names,
            edges,
            entry: first_node,
            exit: last_node,
        },
        all_bytecode,
    );

    vcbin.write_to_file(output)?;

    println!(
        "Chassed '{}' → {} ({} bytes)",
        chass_name,
        output.display(),
        std::fs::metadata(output)?.len()
    );

    Ok(())
}

fn cmd_vcbin_info(file: &Path) -> error::Result<()> {
    let vcbin = vcbin::VcBin::read_from_file(file)?;

    println!("Name:      {}", vcbin.metadata.name);
    println!("Version:   {}", vcbin.metadata.version);
    println!("Mode:      {}", vcbin.metadata.mode);
    println!("Timestamp: {}", vcbin.metadata.timestamp);
    println!("Hash valid: {}", vcbin.verify_hash());

    println!("\nInputs:");
    for port in &vcbin.interface.inputs {
        println!("  {} : {}", port.name, port.ty);
    }

    println!("Outputs:");
    for port in &vcbin.interface.outputs {
        println!("  {} : {}", port.name, port.ty);
    }

    println!("\nAllowed VANs:");
    for van in &vcbin.permissions.allowed_vans {
        println!("  {}", van);
    }

    println!("\nGraph nodes: {}", vcbin.graph.nodes.join(", "));
    println!("Entry: {}  Exit: {}", vcbin.graph.entry, vcbin.graph.exit);
    println!("Bytecode instructions: {}", vcbin.bytecode.len());

    Ok(())
}

fn cmd_vcbin_verify(file: &Path) -> error::Result<()> {
    let vcbin = vcbin::VcBin::read_from_file(file)?;

    if vcbin.verify_hash() {
        println!("✓ Hash verified: {}", hex::encode(vcbin.hash));
        Ok(())
    } else {
        Err(VenturiError::VcBin(format!(
            "Hash mismatch for {}",
            file.display()
        )))
    }
}

fn cmd_validate(file: &Path) -> error::Result<()> {
    let source = std::fs::read_to_string(file)?;
    let tokens = lexer::tokenize(&source)?;
    let vt = parser::parse(tokens)?;

    let validator = validator::Validator::new();
    validator.validate_file(&vt)?;

    println!("✓ {} is valid", file.display());
    println!("  Kind:    {:?}", vt.kind);
    println!("  VAN:     {}", vt.van.as_deref().unwrap_or("(none)"));
    println!("  Inputs:  {}", vt.inputs.len());
    println!("  Outputs: {}", vt.outputs.len());
    println!("  Uses:    {}", vt.uses.len());
    println!("  Pits:    {}", vt.pits.len());
    println!(
        "  Func:    {}",
        vt.func.as_ref().map(|f| f.name.as_str()).unwrap_or("(none)")
    );

    Ok(())
}

fn cmd_graph(dir: &Path) -> error::Result<()> {
    let store_path = pit_store_path();
    let mut runtime = Runtime::new(&store_path)?;

    let mut vt_files: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("vt"))
        .collect();
    vt_files.sort();

    for vt_file in &vt_files {
        runtime.load_vt_file(vt_file)?;
    }
    runtime.wire_dag_edges()?;

    println!("DAG topology ({} nodes):", runtime.dag.nodes.len());
    let order = runtime.dag.topological_order();
    for id in &order {
        let node = &runtime.dag.nodes[id];
        let deps: Vec<String> = runtime
            .dag
            .edges
            .get(id)
            .map(|edges| {
                edges
                    .iter()
                    .map(|dep_id| {
                        runtime
                            .dag
                            .nodes
                            .get(dep_id)
                            .map(|n| n.name.clone())
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .unwrap_or_default();

        if deps.is_empty() {
            println!("  [{}] {}", id, node.name);
        } else {
            println!("  [{}] {} → {}", id, node.name, deps.join(", "));
        }
    }

    Ok(())
}

fn cmd_gui(dir: &Path, backend: gui::Backend) -> error::Result<()> {
    // Load contact data from the data.vt node in the gui-app DAG.
    let data_path = dir.join("data.vt");
    let card = if data_path.exists() {
        let source = std::fs::read_to_string(&data_path)?;
        let tokens = lexer::tokenize(&source)?;
        let vt = parser::parse(tokens)?;
        let mut card = gui::ContactCard::default();
        for input in &vt.inputs {
            match input.name.as_str() {
                "name" => {
                    if let Some(ast::Expr::StrLit(v)) = &input.default {
                        card.name = v.clone();
                    }
                }
                "role" => {
                    if let Some(ast::Expr::StrLit(v)) = &input.default {
                        card.role = v.clone();
                    }
                }
                "email" => {
                    if let Some(ast::Expr::StrLit(v)) = &input.default {
                        card.email = v.clone();
                    }
                }
                "tags" => {
                    if let Some(ast::Expr::StrLit(v)) = &input.default {
                        card.tags = v.clone();
                    }
                }
                _ => {}
            }
        }
        card
    } else {
        gui::ContactCard::default()
    };

    println!(
        "Launching VenturiCards GUI ({:?} backend)...",
        backend
    );
    gui::run(card, backend)
}

fn cmd_emit(dir: &Path, format: &str, output: &Path) -> error::Result<()> {
    let mut vt_files: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("vt"))
        .collect();
    vt_files.sort();

    if vt_files.is_empty() {
        println!("No .vt files found in {}", dir.display());
        return Ok(());
    }

    // Parse and validate all files
    let mut named_files: Vec<(String, ast::VtFile)> = Vec::new();
    for path in &vt_files {
        let source = std::fs::read_to_string(path)?;
        let tokens = lexer::tokenize(&source)?;
        let vt = parser::parse(tokens)?;

        let validator = validator::Validator::new();
        validator.validate_file(&vt)?;

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("node")
            .to_string();
        named_files.push((name, vt));
    }

    // Collect all edges
    let edges: Vec<(String, String)> = named_files
        .iter()
        .flat_map(|(_, vt)| {
            vt.dag_wires
                .iter()
                .map(|w| (w.from.clone(), w.to.clone()))
        })
        .collect();

    // Topological sort via Kahn's algorithm
    let ordered = topo_sort_names(&named_files, &edges)?;

    // Build DagContext with borrowed refs
    let nodes: Vec<(&str, &ast::VtFile)> = ordered
        .iter()
        .filter_map(|name| {
            named_files
                .iter()
                .find(|(n, _)| n == name)
                .map(|(n, vt)| (n.as_str(), vt))
        })
        .collect();

    let edge_refs: Vec<(&str, &str)> = edges
        .iter()
        .map(|(f, t)| (f.as_str(), t.as_str()))
        .collect();

    let ctx = codegen::DagContext {
        nodes,
        edges: edge_refs,
    };

    // Dispatch to emitter
    let files = codegen::emit_dag(&ctx, format)?;

    // Write output
    std::fs::create_dir_all(output)?;
    for (filename, content) in &files {
        let path = output.join(filename);
        std::fs::write(&path, content)?;
        println!("  wrote {}", path.display());
    }

    println!("Emitted {} file(s) to {}", files.len(), output.display());
    Ok(())
}

fn topo_sort_names(
    files: &[(String, ast::VtFile)],
    edges: &[(String, String)],
) -> error::Result<Vec<String>> {
    // Kahn's algorithm for topological sort on name strings.
    let mut in_degree: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut adj: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    // Initialize
    for (name, _) in files {
        in_degree.insert(name.clone(), 0);
        adj.entry(name.clone()).or_insert_with(Vec::new);
    }

    // Count in-degrees
    for (from, to) in edges {
        adj.entry(from.clone())
            .or_insert_with(Vec::new)
            .push(to.clone());
        *in_degree.entry(to.clone()).or_insert(0) += 1;
    }

    // Find all nodes with in_degree 0
    let mut queue: Vec<String> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(name, _)| name.clone())
        .collect();

    let mut result = Vec::new();
    while let Some(node) = queue.pop() {
        result.push(node.clone());

        if let Some(neighbors) = adj.get(&node) {
            for neighbor in neighbors.clone() {
                if let Some(deg) = in_degree.get_mut(&neighbor) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(neighbor.clone());
                    }
                }
            }
        }
    }

    if result.len() != files.len() {
        return Err(error::VenturiError::Codegen(
            "cycle detected in DAG".to_string(),
        ));
    }

    Ok(result)
}

fn eval_const_expr(expr: &ast::Expr) -> vm::bytecode::Value {
    use ast::Expr;
    use vm::bytecode::Value;
    match expr {
        Expr::IntLit(n) => Value::Int(*n),
        Expr::FloatLit(f) => Value::Float(*f),
        Expr::StrLit(s) => Value::Str(s.clone()),
        Expr::BoolLit(b) => Value::Bool(*b),
        _ => Value::Null,
    }
}
