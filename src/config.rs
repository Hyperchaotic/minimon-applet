use cosmic::{
    cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry},
    cosmic_theme::palette::Srgba,
};
use serde::{Deserialize, Serialize};

use crate::fl;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum ColorVariant {
    Color1,
    Color2,
    Color3,
    Color4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphKind {
    Ring,
    Line,
}

impl From<usize> for GraphKind {
    fn from(index: usize) -> Self {
        match index {
            0 => GraphKind::Ring,
            1 => GraphKind::Line,
            _ => panic!("Invalid index for SvgKind"),
        }
    }
}

impl From<GraphKind> for usize {
    fn from(kind: GraphKind) -> Self {
        match kind {
            GraphKind::Ring => 0,
            GraphKind::Line => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Cpu(GraphKind),
    Memory(GraphKind),
    Network(GraphKind),
}

impl std::fmt::Display for DeviceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DeviceKind::Cpu(_) => write!(f, "{}", fl!("sensor-cpu")),
            DeviceKind::Memory(_) => write!(f, "{}", fl!("sensor-memory")),
            DeviceKind::Network(_) => write!(f, "{}", fl!("sensor-network")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, CosmicConfigEntry, PartialEq, Eq)]
#[version = 1]
pub struct GraphColors {
    pub color1: Srgba<u8>,
    pub color2: Srgba<u8>,
    pub color3: Srgba<u8>,
    pub color4: Srgba<u8>,
}

impl Default for GraphColors {
    fn default() -> Self {
        Self {
            color1: Srgba::from_components((0x2b, 0x2b, 0x2b, 0xff)),
            color2: Srgba::from_components((255, 255, 255, 255)),
            color3: Srgba::from_components((85, 85, 85, 255)),
            color4: Srgba::from_components((255, 6, 0, 255)),
        }
    }
}

impl GraphColors {
    pub fn new(kind: DeviceKind) -> Self {
        match kind {
            DeviceKind::Cpu(_) => GraphColors::default(),
            DeviceKind::Memory(_) => GraphColors {
                color4: Srgba::from_components((187, 41, 187, 255)),
                ..Default::default()
            },
            DeviceKind::Network(_) => GraphColors {
                color1: Srgba::from_components((0x2b, 0x2b, 0x2b, 255)),
                color2: Srgba::from_components((47, 141, 255, 255)),
                color3: Srgba::from_components((255, 0, 0, 255)),
                color4: Srgba::from_components((0x2b, 0x2b, 0x2b, 255)),
            },
        }
    }

    pub fn set_color(&mut self, srgb: Srgba<u8>, variant: ColorVariant) {
        match variant {
            ColorVariant::Color1 => self.color1 = srgb,
            ColorVariant::Color2 => self.color2 = srgb,
            ColorVariant::Color3 => self.color3 = srgb,
            ColorVariant::Color4 => self.color4 = srgb,
        }
    }

    pub fn get_color(self, variant: ColorVariant) -> Srgba<u8> {
        match variant {
            ColorVariant::Color1 => self.color1,
            ColorVariant::Color2 => self.color2,
            ColorVariant::Color3 => self.color3,
            ColorVariant::Color4 => self.color4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, CosmicConfigEntry, PartialEq)]
#[version = 1]
pub struct MinimonConfig {
    pub enable_cpu_chart: bool,
    pub enable_cpu_label: bool,
    pub cpu_type: GraphKind,
    pub enable_mem_chart: bool,
    pub enable_mem_label: bool,
    pub mem_type: GraphKind,
    pub enable_net_chart: bool,
    pub enable_net_label: bool,
    pub refresh_rate: u64,
    pub enable_adaptive_net: bool,
    pub net_bandwidth: u64,
    pub net_unit: Option<usize>,
    pub cpu_colors: GraphColors,
    pub mem_colors: GraphColors,
    pub net_colors: GraphColors,
    /// The minimum size of labels
    pub label_size_default: u16,
}

impl Default for MinimonConfig {
    fn default() -> Self {
        Self {
            enable_cpu_chart: true,
            enable_cpu_label: false,
            cpu_type: GraphKind::Ring,
            enable_mem_chart: true,
            enable_mem_label: false,
            mem_type: GraphKind::Line,
            enable_net_chart: true,
            enable_net_label: false,
            refresh_rate: 1000,
            enable_adaptive_net: true,
            net_bandwidth: 62_500_000, // 500Mbit/s
            net_unit: Some(0),
            cpu_colors: GraphColors::new(DeviceKind::Cpu(GraphKind::Ring)),
            mem_colors: GraphColors::new(DeviceKind::Memory(GraphKind::Line)),
            net_colors: GraphColors::new(DeviceKind::Network(GraphKind::Line)),
            label_size_default: 11,
        }
    }
}
