use cosmic::Element;

use crate::{
    colorpicker::DemoGraph,
    config::{GraphColors, GraphKind, MinimonConfig},
};

pub trait Sensor {
    fn graph_kind(&self) -> GraphKind;
    fn set_graph_kind(&mut self, kind: GraphKind);
    fn update(&mut self);
    fn demo_graph(&self, colors: GraphColors) -> Box<dyn DemoGraph>;
    fn graph(&self) -> String;
    fn settings_ui(&self, config: &MinimonConfig) -> Element<crate::app::Message>;
}

pub mod gpus;
pub mod gpu;
pub mod cpu;
pub mod disks;
pub mod memory;
pub mod network;
