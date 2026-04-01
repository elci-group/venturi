use crate::ast::{NodeKind, VtFile};
use crate::compiler;
use crate::error::{Result, VenturiError};
use crate::graph::{Dag, DagNodeKind, PitState};
use crate::lexer;
use crate::parser;
use crate::pits::PitStore;
use crate::validator::Validator;
use crate::vcbin::VcBin;
use crate::vm::bytecode::Value;
use crate::vm::{ExecContext, Vm};
use std::collections::HashMap;
use std::path::Path;

pub struct Runtime {
    pub dag: Dag,
    pub vm: Vm,
    pub pits: PitStore,
    pub cache: HashMap<u32, Value>,
    validator: Validator,
}

impl Runtime {
    pub fn new(pit_store_path: &Path) -> Result<Self> {
        Ok(Runtime {
            dag: Dag::new(),
            vm: Vm::new(),
            pits: PitStore::load(pit_store_path)?,
            cache: HashMap::new(),
            validator: Validator::new(),
        })
    }

    pub fn load_vt_file(&mut self, path: &Path) -> Result<u32> {
        let source = std::fs::read_to_string(path)?;
        let tokens = lexer::tokenize(&source)?;
        let vt = parser::parse(tokens)?;

        self.validator.validate_file(&vt)?;

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let van = vt.van.clone();

        // Register pit refs as pit nodes in the DAG
        for pit_ref in &vt.pits {
            let pit_name = pit_ref.url.trim_start_matches('@').to_string();
            let pit_state = PitState {
                name: pit_name.clone(),
                vcbin_path: self
                    .pits
                    .status(&pit_name)
                    .ok()
                    .and_then(|e| e.active())
                    .map(|v| v.vcbin_path.clone()),
                version: self
                    .pits
                    .status(&pit_name)
                    .ok()
                    .map(|e| e.active_version)
                    .unwrap_or(0),
            };
            let _pit_id = self
                .dag
                .add_node(pit_name, DagNodeKind::Pit(pit_state), None);
        }

        let id = self.dag.add_node(name, DagNodeKind::Module(vt), van);

        // Wire DAG edges from dag_wires in the file
        // We'll do this after all files are loaded; expose a separate step
        Ok(id)
    }

    pub fn load_vcbin(&mut self, path: &Path) -> Result<u32> {
        let vcbin = VcBin::read_from_file(path)?;

        if !vcbin.verify_hash() {
            return Err(VenturiError::VcBin(
                "Hash verification failed".to_string(),
            ));
        }

        let name = vcbin.metadata.name.clone();
        let van = vcbin.permissions.allowed_vans.first().cloned();

        let id = self.dag.add_node(name, DagNodeKind::Chass(vcbin), van);
        Ok(id)
    }

    pub fn wire_dag_edges(&mut self) -> Result<()> {
        // Collect wires first to avoid borrow issues
        let wires: Vec<(String, String)> = self
            .dag
            .nodes
            .values()
            .filter_map(|n| {
                if let DagNodeKind::Module(ref vt) = n.kind {
                    Some(
                        vt.dag_wires
                            .iter()
                            .map(|w| (w.from.clone(), w.to.clone()))
                            .collect::<Vec<_>>(),
                    )
                } else {
                    None
                }
            })
            .flatten()
            .collect();

        for (from_name, to_name) in wires {
            let from_id = self.dag.node_by_name(&from_name);
            let to_id = self.dag.node_by_name(&to_name);

            match (from_id, to_id) {
                (Some(f), Some(t)) => {
                    self.dag.add_edge(f, t)?;
                }
                (None, _) => {
                    eprintln!("[venturi warn] DAG wire: node '{}' not found", from_name);
                }
                (_, None) => {
                    eprintln!("[venturi warn] DAG wire: node '{}' not found", to_name);
                }
            }
        }

        Ok(())
    }

    pub fn execute_dag(&mut self) -> Result<HashMap<String, Value>> {
        self.wire_dag_edges()?;
        self.validator.validate_dag(&self.dag)?;

        let order = self.dag.topological_order();
        let mut outputs: HashMap<String, Value> = HashMap::new();

        for node_id in order {
            let result = self.execute_node(node_id, &outputs)?;
            let node_name = self
                .dag
                .nodes
                .get(&node_id)
                .map(|n| n.name.clone())
                .unwrap_or_default();
            outputs.insert(node_name, result);
        }

        Ok(outputs)
    }

    pub fn execute_node(&mut self, node_id: u32, node_outputs: &HashMap<String, Value>) -> Result<Value> {
        // Check cache for plane nodes
        if let Some(cached) = self.cache.get(&node_id) {
            return Ok(cached.clone());
        }

        let node = self
            .dag
            .nodes
            .get(&node_id)
            .ok_or_else(|| VenturiError::Vm(format!("Node {} not found", node_id)))?
            .clone();

        let result = match &node.kind {
            DagNodeKind::Module(vt) => self.execute_module(vt, node_outputs)?,
            DagNodeKind::Chass(vcbin) => self.execute_chass(vcbin, node_outputs)?,
            DagNodeKind::Pit(pit_state) => self.execute_pit(pit_state, node_outputs)?,
        };

        // Memoize plane nodes
        if let DagNodeKind::Module(vt) = &node.kind {
            if vt.kind == NodeKind::Plane {
                self.cache.insert(node_id, result.clone());
            }
        }

        Ok(result)
    }

    fn execute_module(&self, vt: &VtFile, node_outputs: &HashMap<String, Value>) -> Result<Value> {
        let func = match &vt.func {
            Some(f) => f,
            None => return Ok(Value::Null),
        };

        let bytecode = compiler::compile(func);

        let mut ctx = ExecContext::new(vt.van.clone());

        // Pre-populate variables from inputs
        for input in &vt.inputs {
            if let Some(default) = &input.default {
                let val = eval_const_expr(default);
                ctx.variables.insert(input.name.clone(), val);
            }
        }

        // Pre-populate node outputs from upstream
        for (k, v) in node_outputs {
            ctx.node_outputs.insert(k.clone(), v.clone());
            ctx.variables.insert(k.clone(), v.clone());
        }

        // Map func params to their values (use node_outputs or null)
        for param in &func.params {
            if !ctx.variables.contains_key(param) {
                if let Some(val) = node_outputs.get(param) {
                    ctx.variables.insert(param.clone(), val.clone());
                }
            }
        }

        self.vm.execute(&bytecode, &mut ctx)
    }

    fn execute_chass(&self, vcbin: &VcBin, node_outputs: &HashMap<String, Value>) -> Result<Value> {
        // Check VAN permissions
        // (simplified: just check the first allowed VAN)

        let mut ctx = ExecContext::new(vcbin.permissions.allowed_vans.first().cloned());

        for (k, v) in node_outputs {
            ctx.node_outputs.insert(k.clone(), v.clone());
            ctx.variables.insert(k.clone(), v.clone());
        }

        self.vm.execute(&vcbin.bytecode, &mut ctx)
    }

    fn execute_pit(&self, pit_state: &PitState, node_outputs: &HashMap<String, Value>) -> Result<Value> {
        match &pit_state.vcbin_path {
            None => {
                eprintln!("[venturi warn] Pit '{}' has no active vcbin; returning Null", pit_state.name);
                Ok(Value::Null)
            }
            Some(path) => {
                let vcbin = VcBin::read_from_file(Path::new(path))?;
                if !vcbin.verify_hash() {
                    return Err(VenturiError::VcBin(
                        "Pit vcbin hash verification failed".to_string(),
                    ));
                }
                self.execute_chass(&vcbin, node_outputs)
            }
        }
    }

    pub fn apply_pit_update(&mut self, pit_name: &str, vcbin_path: &str) -> Result<()> {
        self.pits.update(pit_name, vcbin_path)?;

        // Invalidate cache for pit node and all downstream nodes
        if let Some(pit_id) = self.dag.node_by_name(pit_name) {
            self.cache.remove(&pit_id);
            let downstream = self.dag.downstream(pit_id);
            for id in downstream {
                self.cache.remove(&id);
            }

            // Update the pit state in the dag node
            if let Some(node) = self.dag.nodes.get_mut(&pit_id) {
                if let DagNodeKind::Pit(ref mut state) = node.kind {
                    let entry = self.pits.status(pit_name)?;
                    state.vcbin_path = entry.active().map(|v| v.vcbin_path.clone());
                    state.version = entry.active_version;
                }
            }
        }

        Ok(())
    }

    pub fn dag(&self) -> &Dag {
        &self.dag
    }
}

fn eval_const_expr(expr: &crate::ast::Expr) -> Value {
    use crate::ast::Expr;
    match expr {
        Expr::IntLit(n) => Value::Int(*n),
        Expr::FloatLit(f) => Value::Float(*f),
        Expr::StrLit(s) => Value::Str(s.clone()),
        Expr::BoolLit(b) => Value::Bool(*b),
        _ => Value::Null,
    }
}
