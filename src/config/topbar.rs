use serde::{Deserialize, Deserializer};

use super::{sidebar::validate_sidebar_rows, AgentSidebarToken};

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TopbarConfig {
    pub enabled: bool,
    #[serde(deserialize_with = "deserialize_rows")]
    pub rows: Vec<Vec<AgentSidebarToken>>,
}

impl Default for TopbarConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rows: vec![vec![AgentSidebarToken::Workspace]],
        }
    }
}

fn deserialize_rows<'de, D>(deserializer: D) -> Result<Vec<Vec<AgentSidebarToken>>, D::Error>
where
    D: Deserializer<'de>,
{
    let rows = Vec::<Vec<AgentSidebarToken>>::deserialize(deserializer)?;
    validate_sidebar_rows(&rows).map_err(serde::de::Error::custom)?;
    Ok(rows)
}
