#[cfg(test)]
mod tests {
    use venturi::graph::{Dag, DagNodeKind};
    use venturi::ast::{VtFile, NodeKind};

    #[test]
    fn test_cycle_detection() {
        let mut dag = Dag::new();
        
        let file_a = VtFile::new(NodeKind::Plane);
        let file_b = VtFile::new(NodeKind::Plane);
        let file_c = VtFile::new(NodeKind::Plane);

        let node_a = dag.add_node("A".to_string(), DagNodeKind::Module(file_a), None);
        let node_b = dag.add_node("B".to_string(), DagNodeKind::Module(file_b), None);
        let node_c = dag.add_node("C".to_string(), DagNodeKind::Module(file_c), None);

        // A -> B
        assert!(dag.add_edge(node_a, node_b).is_ok());
        // B -> C
        assert!(dag.add_edge(node_b, node_c).is_ok());
        
        // C -> A (Creates a cycle, should fail)
        let cycle_err = dag.add_edge(node_c, node_a);
        assert!(cycle_err.is_err());
        
        match cycle_err {
            Err(venturi::error::VenturiError::Cycle { node }) => {
                assert_eq!(node, "C");
            }
            _ => panic!("Expected a Cycle error"),
        }
    }
}
