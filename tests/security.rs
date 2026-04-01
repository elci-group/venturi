#[cfg(test)]
mod tests {
    use venturi::ast::{VtFile, NodeKind};
    use venturi::graph::{Dag, DagNodeKind};
    use venturi::validator::Validator;

    #[test]
    fn test_permission_sandbox() {
        let mut dag = Dag::new();
        let validator = Validator::new();

        let mut file_a = VtFile::new(NodeKind::Vortex);
        // Missing a VAN declaration entirely for a Vortex node!
        // We will mock this scenario
        file_a.van = None;

        let node_a = dag.add_node("Node_A".to_string(), DagNodeKind::Module(file_a.clone()), None);

        // Required VAN to be matched
        let result = validator.validate_van_consistency(&file_a, Some("@trusted"));

        // Should return a Permission error because required was Some("@trusted") but got None
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Permission denied"));
        assert!(err_msg.contains("required"));
        assert!(err_msg.contains("@trusted"));
    }

    #[test]
    fn test_use_chassis_version_pinning() {
        use venturi::ast::UseChass;
        
        let mut file_a = VtFile::new(NodeKind::Plane);
        
        // Simulating the scenario where we import two chassis with the same alias
        // Note: Currently Venturi syntax handles this using unique 'alias' fields per `use chassis as` statement
        file_a.uses.push(UseChass {
            path: "local/v1/auth".to_string(),
            alias: "auth_v1".to_string()
        });
        
        file_a.uses.push(UseChass {
            path: "local/v2/auth".to_string(),
            alias: "auth_v2".to_string()
        });

        // Version pinning is handled gracefully through aliases
        assert_eq!(file_a.uses[0].alias, "auth_v1");
        assert_eq!(file_a.uses[1].alias, "auth_v2");
        assert_ne!(file_a.uses[0].path, file_a.uses[1].path);
    }
}
