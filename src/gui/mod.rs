pub mod egui_app;
pub mod iced_app;

/// Parsed data from the gui-app DAG (.vt files).
#[derive(Debug, Clone)]
pub struct ContactCard {
    pub name: String,
    pub role: String,
    pub email: String,
    pub tags: String,
}

impl Default for ContactCard {
    fn default() -> Self {
        Self {
            name: "Alice Chen".into(),
            role: "Software Engineer".into(),
            email: "alice@venturi.dev".into(),
            tags: "rust  systems  dags".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Egui,
    Iced,
}

impl std::str::FromStr for Backend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "egui" => Ok(Backend::Egui),
            "iced" => Ok(Backend::Iced),
            other => Err(format!("unknown backend '{}': choose egui or iced", other)),
        }
    }
}

pub fn run(card: ContactCard, backend: Backend) -> crate::error::Result<()> {
    match backend {
        Backend::Egui => egui_app::run(card),
        Backend::Iced => iced_app::run(card),
    }
}
