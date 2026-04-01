use crate::ast::VtFile;
use crate::error::{Result, VenturiError};
use crate::graph::Dag;

pub struct Validator;

impl Validator {
    pub fn new() -> Self {
        Validator
    }

    pub fn validate_file(&self, vt: &VtFile) -> Result<()> {
        self.check_shebang_consistency(vt)?;
        self.check_inputs_outputs(vt)?;
        self.check_func_params(vt)?;
        Ok(())
    }

    fn check_shebang_consistency(&self, vt: &VtFile) -> Result<()> {
        use crate::ast::NodeKind;
        // Vortex nodes should declare a VAN
        if vt.kind == NodeKind::Vortex && vt.van.is_none() {
            eprintln!(
                "[venturi warn] Vortex node has no VAN declaration; all side effects are unrestricted"
            );
        }
        Ok(())
    }

    fn check_inputs_outputs(&self, vt: &VtFile) -> Result<()> {
        // Input names must be unique
        let mut seen = std::collections::HashSet::new();
        for input in &vt.inputs {
            if !seen.insert(&input.name) {
                return Err(VenturiError::Validation(format!(
                    "Duplicate input declaration: {}",
                    input.name
                )));
            }
        }

        // Output names must be unique
        let mut seen = std::collections::HashSet::new();
        for output in &vt.outputs {
            if !seen.insert(&output.name) {
                return Err(VenturiError::Validation(format!(
                    "Duplicate output declaration: {}",
                    output.name
                )));
            }
        }
        Ok(())
    }

    fn check_func_params(&self, vt: &VtFile) -> Result<()> {
        if let Some(func) = &vt.func {
            // Every func param should correspond to an input
            let input_names: std::collections::HashSet<&str> =
                vt.inputs.iter().map(|i| i.name.as_str()).collect();

            for param in &func.params {
                if !input_names.contains(param.as_str()) {
                    eprintln!(
                        "[venturi warn] Func param '{}' has no matching input declaration",
                        param
                    );
                }
            }
        }
        Ok(())
    }

    pub fn validate_dag(&self, dag: &Dag) -> Result<()> {
        if dag.has_cycle() {
            return Err(VenturiError::Cycle {
                node: "unknown".to_string(),
            });
        }
        Ok(())
    }

    pub fn validate_van_consistency(&self, vt: &VtFile, required_van: Option<&str>) -> Result<()> {
        if let Some(required) = required_van {
            match &vt.van {
                None => {
                    return Err(VenturiError::Permission {
                        required: required.to_string(),
                        got: None,
                    });
                }
                Some(van) if van != required => {
                    return Err(VenturiError::Permission {
                        required: required.to_string(),
                        got: Some(van.clone()),
                    });
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}
