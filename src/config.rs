use cosmic::{
    cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry},
    cosmic_theme::palette::{self, Srgb},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, CosmicConfigEntry, PartialEq, Eq)]
#[version = 1]
pub struct GraphColors {
    pub background: Option<u32>,
    pub text: Option<u32>,
    pub ringback: Option<u32>,
    pub ringfont: Option<u32>,
}

impl Default for GraphColors {
    fn default() -> Self {
        Self {
            background: Some(Srgb::from_components((0x1b, 0x1b, 0x1b)).into()),
            text: Some(palette::named::WHITE.into()),
            ringback: Some(palette::named::WHITE.into()),
            ringfont: Some(palette::named::RED.into()),
        }
    }
}

impl GraphColors {
    pub fn background_to_string(&self) -> String {
        GraphColors::to_string(self.background)
    }

    pub fn text_to_string(&self) -> String {
        GraphColors::to_string(self.text)
    }

    pub fn ringfront_to_string(&self) -> String {
        GraphColors::to_string(self.ringfont)
    }

    pub fn ringback_to_string(&self) -> String {
        GraphColors::to_string(self.ringback)
    }

    fn to_string(col: Option<u32>) -> String {
        if let Some(c) = col {
            let c = Srgb::from(c);
            format!("rgba({},{},{},1.0)", c.red, c.green, c.blue)
        } else {
            "rgba(0,0,0,0.0)".to_string()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, CosmicConfigEntry, PartialEq, Eq)]
#[version = 1]
pub struct MinimonConfig {
    pub text_only: bool,
    pub enable_cpu: bool,
    pub enable_mem: bool,
    pub refresh_rate: u64,
    pub cpu_colors: GraphColors,
    pub mem_colors: GraphColors,
}

impl Default for MinimonConfig {
    fn default() -> Self {
        Self {
            text_only: false,
            enable_cpu: true,
            enable_mem: true,
            refresh_rate: 1000,
            cpu_colors: GraphColors::default(),
            mem_colors: GraphColors { ringfont: Some(palette::named::PURPLE.into()), ..Default::default() },
        }
    }
}
