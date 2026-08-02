use serde::Deserialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockSide {
    #[default]
    Bottom,
    Right,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DockConfig {
    pub enabled: bool,
    pub side: DockSide,
    pub size: u16,
}

impl Default for DockConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            side: DockSide::Bottom,
            size: 10,
        }
    }
}
