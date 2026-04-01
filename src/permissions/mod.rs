use crate::error::{Result, VenturiError};
use crate::pits::PitEntry;

pub struct PermissionsHandler {
    pub current_van: Option<String>,
}

impl PermissionsHandler {
    pub fn new(van: Option<String>) -> Self {
        PermissionsHandler { current_van: van }
    }

    pub fn check_van(&self, required: &str) -> Result<()> {
        match &self.current_van {
            Some(van) if van == required => Ok(()),
            Some(van) => Err(VenturiError::Permission {
                required: required.to_string(),
                got: Some(van.clone()),
            }),
            None => Err(VenturiError::Permission {
                required: required.to_string(),
                got: None,
            }),
        }
    }

    pub fn validate_pit_update(&self, pit: &PitEntry, source_van: &str) -> Result<()> {
        if pit.authorized_sources.is_empty() {
            return Ok(());
        }
        if pit.authorized_sources.iter().any(|s| s == source_van) {
            Ok(())
        } else {
            Err(VenturiError::Permission {
                required: pit.authorized_sources.join("|"),
                got: Some(source_van.to_string()),
            })
        }
    }

    pub fn is_sandboxed(&self) -> bool {
        // Vortex nodes are always sandboxed; determined by caller
        false
    }
}
