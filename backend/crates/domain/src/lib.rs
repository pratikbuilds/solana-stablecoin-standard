use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceComponent {
    pub name: String,
    pub layer: String,
}

impl WorkspaceComponent {
    pub fn new(name: impl Into<String>, layer: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            layer: layer.into(),
        }
    }
}

