#[cfg(test)]
mod tests {
    use venturi::ast::{VtFile, NodeKind, InputDecl, OutputDecl, VtType, DagWire};
    use venturi::graph::{Dag, DagNodeKind};
    use venturi::validator::Validator;

    #[test]
    fn test_schema_enforcement_mismatch() {
        let mut dag = Dag::new();
        let validator = Validator::new();
        
        // Node A outputs a String
        let mut file_a = VtFile::new(NodeKind::Plane);
        file_a.outputs.push(OutputDecl {
            name: "out_data".to_string(),
            ty: VtType::Str,
        });

        // Node B expects a DataFrame
        let mut file_b = VtFile::new(NodeKind::Plane);
        file_b.inputs.push(InputDecl {
            name: "out_data".to_string(),
            ty: VtType::DataFrame,
            default: None,
        });
        
        file_a.dag_wires.push(DagWire {
            from: "Node_A".to_string(),
            to: "Node_B".to_string(),
        });

        let node_a = dag.add_node("Node_A".to_string(), DagNodeKind::Module(file_a), None);
        let node_b = dag.add_node("Node_B".to_string(), DagNodeKind::Module(file_b), None);

        // We also need to add the edge to the DAG explicitly for the validator to check it
        dag.add_edge(node_a, node_b).unwrap();
        
        // This should fail validation due to a type mismatch
        let result = validator.validate_dag_types(&dag);
        assert!(result.is_err());
        
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Type mismatch"));
        assert!(err_msg.contains("String"));
        assert!(err_msg.contains("DataFrame"));
    }
}
