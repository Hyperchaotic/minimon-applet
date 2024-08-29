use cosmic::{
    cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry},
    cosmic_theme::palette::{self, Srgb},
};
use serde::{Deserialize, Serialize};


#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum CircleGraphColorVariant {
    Background,
    Text,
    RingBack,
    RingFront,
}

impl CircleGraphColorVariant {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Background => "Back.   ",
            Self::Text => "Text.",
            Self::RingBack => "Ring2.   ",
            Self::RingFront => "Ring1.   ",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum LineGraphColorVariant {
    Background,
    Download,
    Upload,
}

impl LineGraphColorVariant {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Background => "Back. ",
            Self::Download => "Download.    ",
            Self::Upload => "Upload.     ",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircleGraphKind {
    Cpu,
    Memory,
}

impl std::fmt::Display for CircleGraphKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CircleGraphKind::Cpu => write!(f, "CPU"),
            CircleGraphKind::Memory => write!(f, "Memory"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, CosmicConfigEntry, PartialEq, Eq)]
#[version = 1]
pub struct CircleGraphColors {
    pub background: u32,
    pub text: u32,
    pub ringback: u32,
    pub ringfont: u32,
}

impl Default for CircleGraphColors {
    fn default() -> Self {
        Self {
            background: Srgb::from_components((0x1b, 0x1b, 0x1b)).into(),
            text: palette::named::WHITE.into(),
            ringback: palette::named::WHITE.into(),
            ringfont: palette::named::RED.into(),
        }
    }
}

impl CircleGraphColors {
    pub fn new(kind: CircleGraphKind) -> CircleGraphColors {
        let mut n = CircleGraphColors::default();
        if kind == CircleGraphKind::Cpu {
            n.ringfont = palette::named::RED.into();
        } else {
            n.ringfont = palette::named::PURPLE.into();
        }
        n
    }

    pub fn background_to_string(&self) -> String {
        CircleGraphColors::to_string(self.background)
    }

    pub fn text_to_string(&self) -> String {
        CircleGraphColors::to_string(self.text)
    }

    pub fn ringfront_to_string(&self) -> String {
        CircleGraphColors::to_string(self.ringfont)
    }

    pub fn ringback_to_string(&self) -> String {
        CircleGraphColors::to_string(self.ringback)
    }

    fn to_string(col: u32) -> String {
        let c = Srgb::from(col);
        format!("rgba({},{},{},1.0)", c.red, c.green, c.blue)
    }

    pub fn set_color(&mut self, srgb: Srgb<u8>, variant: CircleGraphColorVariant) {
        match variant {
            CircleGraphColorVariant::Background => self.background = srgb.into(),
            CircleGraphColorVariant::Text => self.text = srgb.into(),
            CircleGraphColorVariant::RingBack => self.ringback = srgb.into(),
            CircleGraphColorVariant::RingFront => self.ringfont = srgb.into(),
        }
    }

    pub fn to_srgb(self, variant: CircleGraphColorVariant) -> Srgb<u8> {
        match variant {
            CircleGraphColorVariant::Background => Srgb::from(self.background),
            CircleGraphColorVariant::Text => Srgb::from(self.text),
            CircleGraphColorVariant::RingBack => Srgb::from(self.ringback),
            CircleGraphColorVariant::RingFront => Srgb::from(self.ringfont),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, CosmicConfigEntry, PartialEq, Eq)]
#[version = 1]
pub struct LineGraphColors {
    pub background: u32,
    pub download: u32,
    pub upload: u32,
}

impl Default for LineGraphColors {
    fn default() -> Self {
        Self {
            background: Srgb::from_components((0x1b, 0x1b, 0x1b)).into(),
            download: Srgb::from_components((47, 141, 255)).into(),
            upload: Srgb::from_components((255, 0, 0)).into(),
        }
    }
}

impl LineGraphColors {

    pub fn set_color(&mut self, srgb: Srgb<u8>, variant: LineGraphColorVariant) {
        match variant {
            LineGraphColorVariant::Background => self.background = srgb.into(),
            LineGraphColorVariant::Download => self.download = srgb.into(),
            LineGraphColorVariant::Upload => self.upload = srgb.into(),
        }
    }

    pub fn to_srgb(self, variant: LineGraphColorVariant) -> Srgb<u8> {
        match variant {
            LineGraphColorVariant::Background => Srgb::from(self.background),
            LineGraphColorVariant::Download => Srgb::from(self.download),
            LineGraphColorVariant::Upload => Srgb::from(self.upload),
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
    pub cpu_colors: CircleGraphColors,
    pub mem_colors: CircleGraphColors,
    pub net_colors: LineGraphColors,
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
            cpu_colors: CircleGraphColors::default(),
            mem_colors: CircleGraphColors {
                ringfont: palette::named::PURPLE.into(),
                ..Default::default()
            },
            net_colors: LineGraphColors::default(),
        }
    }
}
