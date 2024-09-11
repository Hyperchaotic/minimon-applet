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
pub enum SvgGraphKind {
    Ring,
    Line,
}

impl From<usize> for SvgGraphKind {
    fn from(index: usize) -> Self {
        match index {
            0 => SvgGraphKind::Ring,
            1 => SvgGraphKind::Line,
            _ => panic!("Invalid index for SvgKind"),
        }
    }
}

impl Into<usize> for SvgGraphKind {
    fn into(self) -> usize {
        match self {
            SvgGraphKind::Ring => 0,
            SvgGraphKind::Line => 1,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SvgDevKind {
    Cpu(SvgGraphKind),
    Memory(SvgGraphKind),
    Network(SvgGraphKind),
}

impl std::fmt::Display for SvgDevKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SvgDevKind::Cpu(_) => write!(f, "CPU"),
            SvgDevKind::Memory(_) => write!(f, "Memory"),
            SvgDevKind::Network(_) => write!(f, "Network"),
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
            color1: Srgb::from_components((0x2b, 0x2b, 0x2b)),
            color2: palette::named::WHITE,
            color3: palette::named::WHITE,
            color4: Srgb::from_components((255, 6, 0)),
        }
    }
}

impl SvgColors {
    pub fn new(kind: SvgDevKind) -> Self {
        match kind {
            SvgDevKind::Cpu(_) => SvgColors::default(),
            SvgDevKind::Memory(_) => SvgColors {
                color4: Srgb::from_components((187, 41, 187)),
                ..Default::default()
            },
            SvgDevKind::Network(_) => SvgColors {
                color1: Srgb::from_components((0x2b, 0x2b, 0x2b)),
                color2: Srgb::from_components((47, 141, 255)),
                color3: Srgb::from_components((255, 0, 0)),
                color4: Srgb::from_components((0x2b, 0x2b, 0x2b)),
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
    cpu_type: usize,
    pub enable_mem: bool,
    mem_type: usize,
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
            cpu_type: 0,
            enable_mem: true,
            mem_type: 0,
            enable_net: true,
            refresh_rate: 1000,
            enable_adaptive_net: true,
            net_bandwidth: 62_500_000, // 500Mbit/s
            net_unit: Some(0),
            cpu_colors: SvgColors::new(SvgDevKind::Cpu(SvgGraphKind::Ring)),
            mem_colors: SvgColors::new(SvgDevKind::Memory(SvgGraphKind::Ring)),
            net_colors: SvgColors::new(SvgDevKind::Network(SvgGraphKind::Line)),
        }
    }
}

impl MinimonConfig {
    pub fn cpu_kind(&self) -> SvgDevKind {
        SvgDevKind::Cpu(self.cpu_type.into())
    }
    pub fn set_cpu_kind(&mut self, kind: SvgGraphKind) {
        self.cpu_type = kind.into();
    }
    pub fn memory_kind(&self) -> SvgDevKind {
        SvgDevKind::Memory(self.mem_type.into())
    }
    pub fn set_memory_kind(&mut self, kind: SvgGraphKind) {
        self.mem_type = kind.into();
    }
}
