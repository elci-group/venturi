use crate::ast::VtFile;
use crate::error::{Result, VenturiError};
use crate::vcbin::VcBin;
use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum DagNodeKind {
    Module(VtFile),
    Chass(VcBin),
    Pit(PitState),
}

#[derive(Debug, Clone)]
pub struct PitState {
    pub name: String,
    pub vcbin_path: Option<String>,
    pub version: usize,
}

#[derive(Debug, Clone)]
pub struct DagNode {
    pub id: u32,
    pub name: String,
    pub kind: DagNodeKind,
    pub van: Option<String>,
}

pub struct Dag {
    pub nodes: HashMap<u32, DagNode>,
    pub edges: HashMap<u32, Vec<u32>>,
    graph: DiGraph<u32, ()>,
    node_indices: HashMap<u32, NodeIndex>,
    next_id: u32,
}

impl Dag {
    pub fn new() -> Self {
        Dag {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            graph: DiGraph::new(),
            node_indices: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn add_node(&mut self, name: String, kind: DagNodeKind, van: Option<String>) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        let idx = self.graph.add_node(id);
        self.node_indices.insert(id, idx);

        self.nodes.insert(
            id,
            DagNode {
                id,
                name,
                kind,
                van,
            },
        );
        self.edges.insert(id, Vec::new());

        id
    }

    pub fn add_edge(&mut self, from: u32, to: u32) -> Result<()> {
        let from_idx = self.node_indices.get(&from).copied().ok_or_else(|| {
            VenturiError::Vm(format!("Node {} not found in DAG", from))
        })?;
        let to_idx = self.node_indices.get(&to).copied().ok_or_else(|| {
            VenturiError::Vm(format!("Node {} not found in DAG", to))
        })?;

        self.graph.add_edge(from_idx, to_idx, ());

        // Check for cycle immediately
        if is_cyclic_directed(&self.graph) {
            // Remove the edge we just added
            if let Some(edge) = self.graph.find_edge(from_idx, to_idx) {
                self.graph.remove_edge(edge);
            }
            let node_name = self.nodes.get(&from).map(|n| n.name.clone()).unwrap_or_default();
            return Err(VenturiError::Cycle { node: node_name });
        }

        self.edges.entry(from).or_default().push(to);
        Ok(())
    }

    pub fn topological_order(&self) -> Vec<u32> {
        match toposort(&self.graph, None) {
            Ok(order) => order
                .into_iter()
                .map(|idx| *self.graph.node_weight(idx).unwrap())
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn downstream(&self, node_id: u32) -> Vec<u32> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        self.collect_downstream(node_id, &mut result, &mut visited);
        result
    }

    fn collect_downstream(
        &self,
        node_id: u32,
        result: &mut Vec<u32>,
        visited: &mut std::collections::HashSet<u32>,
    ) {
        if visited.contains(&node_id) {
            return;
        }
        visited.insert(node_id);

        if let Some(neighbors) = self.edges.get(&node_id) {
            for &neighbor in neighbors {
                result.push(neighbor);
                self.collect_downstream(neighbor, result, visited);
            }
        }
    }

    pub fn has_cycle(&self) -> bool {
        is_cyclic_directed(&self.graph)
    }

    pub fn node_by_name(&self, name: &str) -> Option<u32> {
        self.nodes
            .values()
            .find(|n| n.name == name)
            .map(|n| n.id)
    }
}

impl Default for Dag {
    fn default() -> Self {
        Self::new()
    }
}
