use serde::{Deserialize, Serialize};
use sss_domain::WorkspaceComponent;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthReport {
    pub component: WorkspaceComponent,
    pub status: String,
}

pub fn health_report(name: &str, layer: &str) -> HealthReport {
    HealthReport {
        component: WorkspaceComponent::new(name, layer),
        status: "bootstrap-ready".to_string(),
    }
}

