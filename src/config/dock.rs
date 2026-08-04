use serde::{Deserialize, Deserializer};

pub const MIN_DOCK_SIZE: u16 = 3;

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
    #[serde(deserialize_with = "deserialize_size")]
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

fn deserialize_size<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    let size = u16::deserialize(deserializer)?;
    if size < MIN_DOCK_SIZE {
        return Err(serde::de::Error::custom(format!(
            "dock size must be at least {MIN_DOCK_SIZE}"
        )));
    }
    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct TestConfig {
        #[serde(rename = "dock")]
        _dock: DockConfig,
    }

    #[test]
    fn dock_config_rejects_size_below_three_with_diagnostic() {
        let error = toml::from_str::<TestConfig>("[dock]\nsize = 2\n")
            .err()
            .expect("undersized dock should be rejected");

        assert!(error.to_string().contains("at least 3"), "{error}");
    }
}
