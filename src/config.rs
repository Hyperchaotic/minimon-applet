use std::collections::HashMap;

use cosmic::{
    cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry},
    cosmic_theme::palette::Srgba,
};
use serde::{Deserialize, Serialize};

use crate::{fl, sensors::TempUnit};

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
    Heat,
    StackedBars,
}

impl From<usize> for GraphKind {
    fn from(index: usize) -> Self {
        match index {
            0 => GraphKind::Ring,
            1 => GraphKind::Line,
            2 => GraphKind::Heat,
            3 => GraphKind::StackedBars,
            _ => {
                log::error!("GrapKind::From({}) Invalid index for GraphKind", index);
                GraphKind::Line
            }
        }
    }
}

impl From<GraphKind> for usize {
    fn from(kind: GraphKind) -> Self {
        match kind {
            GraphKind::Ring => 0,
            GraphKind::Line => 1,
            GraphKind::Heat => 2,
            GraphKind::StackedBars => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Cpu,
    CpuTemp,
    Memory,
    Network(NetworkVariant),
    Disks(DisksVariant),
    Gpu,
    Vram,
    GpuTemp,
}

impl std::fmt::Display for DeviceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DeviceKind::Cpu => write!(f, "{}", fl!("sensor-cpu")),
            DeviceKind::CpuTemp => write!(f, "{}", fl!("sensor-cpu-temperature")),
            DeviceKind::Memory => write!(f, "{}", fl!("sensor-memory")),
            DeviceKind::Network(_) => write!(f, "{}", fl!("sensor-network")),
            DeviceKind::Disks(_) => write!(f, "{}", fl!("sensor-disks")),
            DeviceKind::Gpu => write!(f, "{}", fl!("sensor-gpu")),
            DeviceKind::Vram => write!(f, "{}", fl!("sensor-vram")),
            DeviceKind::GpuTemp => write!(f, "{}", fl!("sensor-gpu-temp")),
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
            DeviceKind::Cpu => GraphColors::default(),
            DeviceKind::CpuTemp => GraphColors::default(),

            DeviceKind::Memory => GraphColors {
                color4: Srgba::from_components((187, 41, 187, 255)),
                ..Default::default()
            },

            DeviceKind::Network(_) => GraphColors {
                color1: Srgba::from_components((0x2b, 0x2b, 0x2b, 255)),
                color2: Srgba::from_components((47, 141, 255, 255)),
                color3: Srgba::from_components((0, 255, 0, 255)),
                color4: Srgba::from_components((0x2b, 0x2b, 0x2b, 255)),
            },

            DeviceKind::Disks(_) => GraphColors {
                color1: Srgba::from_components((0x2b, 0x2b, 0x2b, 255)),
                color2: Srgba::from_components((255, 102, 0, 255)),
                color3: Srgba::from_components((255, 255, 0, 255)),
                color4: Srgba::from_components((0x2b, 0x2b, 0x2b, 255)),
            },
            DeviceKind::Gpu => GraphColors {
                color4: Srgba::from_components((0, 255, 0, 255)),
                ..Default::default()
            },
            DeviceKind::Vram => GraphColors {
                color4: Srgba::from_components((0, 255, 0, 255)),
                ..Default::default()
            },
            DeviceKind::GpuTemp => GraphColors {
                color4: Srgba::from_components((255, 95, 31, 255)),
                ..Default::default()
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, CosmicConfigEntry, PartialEq, Eq)]
#[version = 1]
pub struct Colors {
    ring: GraphColors,
    line: GraphColors,
    heat: GraphColors,
    stackedbars: GraphColors,
}

impl Colors {
    pub fn new(kind: DeviceKind) -> Self {
        let def = GraphColors::new(kind);
        Colors {
            ring: def,
            line: def,
            heat: def,
            stackedbars: GraphColors {
                color3: Srgba::from_components((80, 80, 255, 255)),
                ..Default::default()
            },
        }
    }

    pub fn get(&self, kind: GraphKind) -> &GraphColors {
        match kind {
            GraphKind::Ring => &self.ring,
            GraphKind::Line => &self.line,
            GraphKind::Heat => &self.heat,
            GraphKind::StackedBars => &self.stackedbars,
        }
    }

    pub fn get_mut(&mut self, kind: GraphKind) -> &mut GraphColors {
        match kind {
            GraphKind::Ring => &mut self.ring,
            GraphKind::Line => &mut self.line,
            GraphKind::Heat => &mut self.heat,
            GraphKind::StackedBars => &mut self.stackedbars,
        }
    }

    pub fn set(&mut self, kind: GraphKind, colors: GraphColors) {
        *self.get_mut(kind) = colors;
    }
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            ring: Default::default(),
            line: Default::default(),
            heat: Default::default(),
            stackedbars: Default::default(),
        }
    }
}

macro_rules! make_config {
    ($name:ident { $($extra:tt)* }) => {
        #[derive(Debug, Clone, Serialize, Deserialize, CosmicConfigEntry, PartialEq, Eq)]
        #[version = 1]
        pub struct $name {
            chart: bool,
            label: bool,
            pub kind: GraphKind,
            colors: Colors,
            $($extra)*
        }

       impl $name {
            pub fn kind(&self) -> GraphKind {
                self.kind
            }
            pub fn visible(&self) -> bool {
                self.chart || self.label
            }
            pub fn chart_visible(&self) -> bool {
                self.chart
            }
            pub fn label_visible(&self) -> bool {
                self.label
            }
            pub fn show_chart(&mut self, visible: bool) {
                self.chart = visible;
            }
            pub fn show_label(&mut self, visible: bool) {
                self.label = visible;
            }
            pub fn colors(&self) -> &GraphColors {
                self.colors.get(self.kind)
            }
            pub fn colors_mut(&mut self) -> &mut GraphColors {
                self.colors.get_mut(self.kind)
            }
        }
    };
}

make_config!(CpuConfig {
    pub no_decimals: bool,
    pub bar_width: u16,
    pub bar_spacing: u16,
});

impl Default for CpuConfig {
    fn default() -> Self {
        Self {
            chart: true,
            label: false,
            kind: GraphKind::Ring,
            colors: Colors::new(DeviceKind::Cpu),
            no_decimals: false,
            bar_width: 4,
            bar_spacing: 1,
        }
    }
}

make_config!(CpuTempConfig {
    pub unit: TempUnit,
});

impl Default for CpuTempConfig {
    fn default() -> Self {
        Self {
            chart: false,
            label: false,
            kind: GraphKind::Heat,
            colors: Colors::new(DeviceKind::CpuTemp),
            unit: TempUnit::Celcius,
        }
    }
}

make_config!(MemoryConfig {
    pub percentage: bool,
});

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            chart: true,
            label: false,
            kind: GraphKind::Ring,
            colors: Colors::new(DeviceKind::Memory),
            percentage: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkVariant {
    Download,
    Upload,
    Combined,
}

make_config!(NetworkConfig {
    pub adaptive: bool,
    pub bandwidth: u64,
    pub unit: Option<usize>,
    pub variant: NetworkVariant,
    pub show_bytes: bool,
});

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            chart: true,
            label: false,
            kind: GraphKind::Line,
            colors: Colors::new(DeviceKind::Network(NetworkVariant::Combined)),
            adaptive: true,
            bandwidth: 62_500_000, // 500Mbit/s
            unit: Some(0),
            variant: NetworkVariant::Combined,
            show_bytes: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DisksVariant {
    Write,
    Read,
    Combined,
}

make_config!(DisksConfig {
    pub variant: DisksVariant,
});

impl Default for DisksConfig {
    fn default() -> Self {
        Self {
            chart: false,
            label: false,
            kind: GraphKind::Line,
            colors: Colors::new(DeviceKind::Disks(DisksVariant::Combined)),
            variant: DisksVariant::Combined,
        }
    }
}

make_config!(GpuUsageConfig {});

impl Default for GpuUsageConfig {
    fn default() -> Self {
        Self {
            chart: true,
            label: false,
            kind: GraphKind::Ring,
            colors: Colors::new(DeviceKind::Gpu),
        }
    }
}

make_config!(GpuVramConfig {});

impl Default for GpuVramConfig {
    fn default() -> Self {
        Self {
            chart: true,
            label: false,
            kind: GraphKind::Ring,
            colors: Colors::new(DeviceKind::Vram),
        }
    }
}

make_config!(GpuTempConfig {
        pub unit: TempUnit,
});

impl Default for GpuTempConfig {
    fn default() -> Self {
        Self {
            chart: false,
            label: false,
            kind: GraphKind::Ring,
            colors: Colors::new(DeviceKind::GpuTemp),
            unit: TempUnit::Celcius,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, CosmicConfigEntry, PartialEq, Eq)]
#[version = 1]
pub struct GpuConfig {
    pub usage: GpuUsageConfig,
    pub vram: GpuVramConfig,
    pub temp: GpuTempConfig,
    pub pause_on_battery: bool,
    pub stack_labels: bool,
}

impl GpuConfig {
    pub fn is_visible(&self) -> bool {
        self.usage.visible() || self.vram.visible() || self.temp.visible()
    }
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            usage: GpuUsageConfig::default(),
            vram: GpuVramConfig::default(),
            temp: GpuTempConfig::default(),
            pause_on_battery: true,
            stack_labels: true,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum ContentType {
    CpuUsage,
    CpuTemp,
    MemoryUsage,
    NetworkUsage,
    DiskUsage,
    GpuInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, CosmicConfigEntry, PartialEq, Eq)]
#[version = 1]
pub struct ContentOrder {
    pub order: Vec<ContentType>,
}

impl Default for ContentOrder {
    fn default() -> Self {
        Self {
            order: vec![
                ContentType::CpuUsage,
                ContentType::CpuTemp,
                ContentType::MemoryUsage,
                ContentType::NetworkUsage,
                ContentType::DiskUsage,
                ContentType::GpuInfo,
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, CosmicConfigEntry, PartialEq)]
#[version = 1]
pub struct MinimonConfig {
    pub refresh_rate: u32,
    pub label_size_default: u16,
    pub monospace_labels: bool,

    pub cpu: CpuConfig,
    pub cputemp: CpuTempConfig,
    pub memory: MemoryConfig,

    pub network1: NetworkConfig,
    pub network2: NetworkConfig,

    pub disks1: DisksConfig,
    pub disks2: DisksConfig,

    pub gpus: HashMap<String, GpuConfig>,

    pub sysmon: Option<String>,

    pub symbols: bool,
    pub panel_spacing: u16,

    pub content_order: ContentOrder,
}

impl Default for MinimonConfig {
    fn default() -> Self {
        Self {
            refresh_rate: 1000,
            label_size_default: 11,
            monospace_labels: false,
            cpu: CpuConfig::default(),
            cputemp: CpuTempConfig::default(),
            memory: MemoryConfig::default(),
            network1: NetworkConfig {
                variant: NetworkVariant::Combined,
                ..Default::default()
            },
            network2: NetworkConfig {
                variant: NetworkVariant::Upload,
                ..Default::default()
            },
            disks1: DisksConfig {
                variant: DisksVariant::Combined,
                ..Default::default()
            },
            disks2: DisksConfig {
                variant: DisksVariant::Read,
                ..Default::default()
            },
            gpus: HashMap::new(),
            sysmon: None,
            symbols: false,
            panel_spacing: 3, // Slider setting for cosmic.space_xs()
            content_order: ContentOrder::default(),
        }
    }
}
