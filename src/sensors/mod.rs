use crate::{colorpicker::DemoGraph, config::{GraphColors, GraphKind}};

pub trait Sensor {
    fn new(kind: GraphKind) -> Self 
        where Self: Sized;
    fn kind(&self) -> GraphKind;
    fn set_kind(&mut self, kind: GraphKind);
    fn update(&mut self);
    fn demo_graph(&self, colors: GraphColors) -> Box<dyn DemoGraph>;
    fn graph(&self) -> String;
}

pub mod cpu;
pub mod network;
pub mod memory;
