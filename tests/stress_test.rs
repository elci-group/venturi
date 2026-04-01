#[cfg(test)]
mod tests {
    use venturi::graph::{Dag, DagNodeKind};
    use venturi::ast::{VtFile, NodeKind};
    use std::time::Instant;

    #[test]
    fn test_deep_graph_stress() {
        let mut dag = Dag::new();
        let num_nodes = 10_000;
        let mut nodes = Vec::with_capacity(num_nodes);

        let start = Instant::now();
        
        // Generate 10,000 nodes
        for i in 0..num_nodes {
            let file = VtFile::new(NodeKind::Plane);
            let id = dag.add_node(format!("Node_{}", i), DagNodeKind::Module(file), None);
            nodes.push(id);
        }

        // Wire them linearly: Node_0 -> Node_1 -> ... -> Node_9999
        for i in 0..(num_nodes - 1) {
            assert!(dag.add_edge(nodes[i], nodes[i + 1]).is_ok());
        }

        // Add 10,000 random cross-edges (ensuring no cycles by only pointing forward)
        for i in 0..(num_nodes - 10) {
            // Forward connections only
            assert!(dag.add_edge(nodes[i], nodes[i + 5]).is_ok());
        }

        let duration = start.elapsed();
        println!("DAG generation and cycle detection for 20,000 edges took: {:?}", duration);

        assert_eq!(dag.topological_order().len(), num_nodes);
    }
}
