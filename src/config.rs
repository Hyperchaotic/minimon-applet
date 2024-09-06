use cosmic::{
    cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry},
    cosmic_theme::palette::{self, Srgb},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum SvgColorVariant {
    Color1,
    Color2,
    Color3,
    Color4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SvgKind {
    Cpu,
    Memory,
    Network,
}

impl std::fmt::Display for SvgKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SvgKind::Cpu => write!(f, "CPU"),
            SvgKind::Memory => write!(f, "Memory"),
            SvgKind::Network => write!(f, "Network"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, CosmicConfigEntry, PartialEq, Eq)]
#[version = 1]
pub struct SvgColors {
    pub color1: Srgb<u8>,
    pub color2: Srgb<u8>,
    pub color3: Srgb<u8>,
    pub color4: Srgb<u8>,
}

impl Default for SvgColors {
    fn default() -> Self {
        Self {
            color1: Srgb::from_components((0x1b, 0x1b, 0x1b)),
            color2: palette::named::WHITE,
            color3: palette::named::WHITE,
            color4: palette::named::RED,
        }
    }
}

impl SvgColors {
    pub fn new(kind: SvgKind) -> Self {
        match kind {
            SvgKind::Cpu => SvgColors::default(),
            SvgKind::Memory => SvgColors {
                color4: palette::named::PURPLE.into(),
                ..Default::default()
            },
            SvgKind::Network => SvgColors {
                color1: Srgb::from_components((0x1b, 0x1b, 0x1b)),
                color2: Srgb::from_components((47, 141, 255)),
                color3: Srgb::from_components((255, 0, 0)),
                ..Default::default()
            },
        }
    }

    pub fn color1_to_string(&self) -> String {
        SvgColors::to_string(self.color1)
    }

    pub fn color2_to_string(&self) -> String {
        SvgColors::to_string(self.color2)
    }

    pub fn color3_to_string(&self) -> String {
        SvgColors::to_string(self.color3)
    }

    pub fn color4_to_string(&self) -> String {
        SvgColors::to_string(self.color4)
    }

    fn to_string(col: Srgb<u8>) -> String {
        format!("rgba({},{},{})", col.red, col.green, col.blue)
    }

    pub fn set_color(&mut self, srgb: Srgb<u8>, variant: SvgColorVariant) {
        match variant {
            SvgColorVariant::Color1 => self.color1 = srgb,
            SvgColorVariant::Color2 => self.color2 = srgb,
            SvgColorVariant::Color3 => self.color3 = srgb,
            SvgColorVariant::Color4 => self.color4 = srgb,
        }
    }

    pub fn get_color(self, variant: SvgColorVariant) -> Srgb<u8> {
        match variant {
            SvgColorVariant::Color1 => self.color1,
            SvgColorVariant::Color2 => self.color2,
            SvgColorVariant::Color3 => self.color3,
            SvgColorVariant::Color4 => self.color4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, CosmicConfigEntry, PartialEq)]
#[version = 1]
pub struct MinimonConfig {
    pub text_only: bool,
    pub enable_cpu: bool,
    pub enable_mem: bool,
    pub enable_net: bool,
    pub refresh_rate: u64,
    pub enable_adaptive_net: bool,
    pub net_bandwidth: u64,
    pub net_unit: Option<usize>,
    pub cpu_colors: SvgColors,
    pub mem_colors: SvgColors,
    pub net_colors: SvgColors,
}

impl Default for MinimonConfig {
    fn default() -> Self {
        Self {
            text_only: false,
            enable_cpu: true,
            enable_mem: true,
            enable_net: true,
            refresh_rate: 1000,
            enable_adaptive_net: true,
            net_bandwidth: 62_500_000, // 500Mbit/s
            net_unit: Some(0),
            cpu_colors: SvgColors::new(SvgKind::Cpu),
            mem_colors: SvgColors::new(SvgKind::Memory),
            net_colors: SvgColors::new(SvgKind::Network),
        }
    }
}
