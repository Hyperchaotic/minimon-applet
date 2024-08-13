use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, CosmicConfigEntry, PartialEq, Eq)]
#[version = 1]
pub struct MinimonConfig {
    pub text_only: bool,
    pub enable_cpu: bool,
    pub enable_mem: bool,
}

impl Default for MinimonConfig {
    fn default() -> Self {
        Self {
            text_only: false,
            enable_cpu: true,
            enable_mem: true,
        }
    }
}
