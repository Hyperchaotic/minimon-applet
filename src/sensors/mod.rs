use cosmic::Element;
use serde::{Deserialize, Serialize};

use crate::{
    colorpicker::DemoGraph,
    config::{GraphColors, GraphKind, MinimonConfig},
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TempUnit {
    Celcius,
    Farenheit,
    Kelvin,
    Rankine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuVariant {
    Amd,
    Intel,
}

pub trait Sensor {
    fn graph_kind(&self) -> GraphKind;
    fn set_graph_kind(&mut self, kind: GraphKind);
    fn update(&mut self);
    fn demo_graph(&self, colors: GraphColors) -> Box<dyn DemoGraph>;
    fn graph(&self) -> String;
    fn settings_ui(&self, config: &MinimonConfig) -> Element<crate::app::Message>;
}

pub mod cpu;
pub mod cputemp;
pub mod disks;
pub mod gpu;
pub mod gpus;
pub mod memory;
pub mod network;

impl From<usize> for TempUnit {
    fn from(index: usize) -> Self {
        match index {
            0 => TempUnit::Celcius,
            1 => TempUnit::Farenheit,
            2 => TempUnit::Kelvin,
            3 => TempUnit::Rankine,
            _ => panic!("Invalid index for TempUnit"),
        }
    }
}

impl From<TempUnit> for usize {
    fn from(kind: TempUnit) -> Self {
        match kind {
            TempUnit::Celcius => 0,
            TempUnit::Farenheit => 1,
            TempUnit::Kelvin => 2,
            TempUnit::Rankine => 3,
        }
    }
}
