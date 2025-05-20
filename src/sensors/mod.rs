use cosmic::Element;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

use crate::{
    config::{ColorVariant, GpuConfig},
    fl,
};

pub static COLOR_CHOICES_RING: LazyLock<[(&'static str, ColorVariant); 4]> = LazyLock::new(|| {
    [
        (fl!("graph-ring-r1").leak(), ColorVariant::Color4),
        (fl!("graph-ring-r2").leak(), ColorVariant::Color3),
        (fl!("graph-ring-back").leak(), ColorVariant::Color1),
        (fl!("graph-ring-text").leak(), ColorVariant::Color2),
    ]
});

pub static COLOR_CHOICES_LINE: LazyLock<[(&'static str, ColorVariant); 3]> = LazyLock::new(|| {
    [
        (fl!("graph-line-graph").leak(), ColorVariant::Color4),
        (fl!("graph-line-back").leak(), ColorVariant::Color1),
        (fl!("graph-line-frame").leak(), ColorVariant::Color2),
    ]
});

pub static COLOR_CHOICES_HEAT: LazyLock<[(&'static str, ColorVariant); 2]> = LazyLock::new(|| {
    [
        (fl!("graph-line-back").leak(), ColorVariant::Color1),
        (fl!("graph-line-frame").leak(), ColorVariant::Color2),
    ]
});

use crate::{colorpicker::DemoGraph, config::GraphKind};

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

use std::any::Any;
pub trait Sensor: Default {
    fn update_config(&mut self, config: &dyn Any, refresh_rate: u32);
    fn graph_kind(&self) -> GraphKind;
    fn set_graph_kind(&mut self, kind: GraphKind);
    fn update(&mut self);
    fn demo_graph(&self) -> Box<dyn DemoGraph>;
    fn graph(&self) -> String;
    fn settings_ui(&self) -> Element<crate::app::Message>;
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
