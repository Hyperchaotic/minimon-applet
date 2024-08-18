use cosmic::{
    cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry},
    cosmic_theme::palette::{self, Srgb},
};
use serde::{Deserialize, Serialize};
use sysinfo::Cpu;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum GraphColorVariant {
    Background,
    Text,
    RingBack,
    RingFront,
}

impl GraphColorVariant {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Background => "Back.   ",
            Self::Text => "Text.",
            Self::RingBack => "Ring2.   ",
            Self::RingFront => "Ring1.   ",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphKind {
    Cpu,
    Memory,
}

impl std::fmt::Display for GraphKind {
   fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
        GraphKind::Cpu => write!(f, "CPU"),
        GraphKind::Memory => write!(f, "Memory"),
    }
}
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, CosmicConfigEntry, PartialEq, Eq)]
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

    pub fn new(kind: GraphKind) -> GraphColors {
        let mut n = GraphColors::default();
        if kind==GraphKind::Cpu {
            n.ringfont = Some(palette::named::RED.into());
        } else {
            n.ringfont = Some(palette::named::PURPLE.into());
        }
        n
    }

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

    pub fn set_color(&mut self, srgb: Srgb<u8>, variant: GraphColorVariant) {
        match variant {
            GraphColorVariant::Background => self.background = Some(srgb.into()),
            GraphColorVariant::Text => self.text = Some(srgb.into()),
            GraphColorVariant::RingBack => self.ringback = Some(srgb.into()),
            GraphColorVariant::RingFront => self.ringfont = Some(srgb.into()),
        }
    }

    pub fn to_srgb(self, variant: GraphColorVariant) -> Srgb<u8> {
        let mut res = Srgb::from_components((0, 0, 0));
        match variant {
            GraphColorVariant::Background => {
                if let Some(c) = self.background {
                    res = Srgb::from(c);
                }
            }
            GraphColorVariant::Text => {
                if let Some(c) = self.text {
                    res = Srgb::from(c);
                }
            }
            GraphColorVariant::RingBack => {
                if let Some(c) = self.ringback {
                    res = Srgb::from(c);
                }
            }
            GraphColorVariant::RingFront => {
                if let Some(c) = self.ringfont {
                    res = Srgb::from(c);
                }
            }
        }
        res
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
            mem_colors: GraphColors {
                ringfont: Some(palette::named::PURPLE.into()),
                ..Default::default()
            },
        }
    }
}
