use sysinfo::System;

use crate::{
    colorpicker::DemoGraph,
    config::{ColorVariant, DeviceKind, GraphColors, GraphKind},
    svg_graph::SvgColors,
};
use std::{collections::VecDeque, fmt::Write};

const MAX_SAMPLES: usize = 21;

const COLOR_CHOICES_RING: [(&str, ColorVariant); 4] = [
    ("Ring1.  ", ColorVariant::Color4),
    ("Ring2.  ", ColorVariant::Color3),
    ("Back.  ", ColorVariant::Color1),
    ("Text.", ColorVariant::Color2),
];

const COLOR_CHOICES_LINE: [(&str, ColorVariant); 3] = [
    ("Graph.  ", ColorVariant::Color4),
    ("Back.  ", ColorVariant::Color1),
    ("Frame.  ", ColorVariant::Color2),
];

#[derive(Debug)]
pub struct SvgStat {
    samples: VecDeque<f64>,
    max_val: u64,
    colors: GraphColors,
    system: System,
    kind: DeviceKind,

    /// current value cpu/ram load shown.
    value: String,
    /// the percentage of the ring to be filled
    percentage: String,

    /// colors cached so we don't need to convert to string every time
    svg_colors: SvgColors,
}

impl DemoGraph for SvgStat {
    fn demo(&self) -> String {
        match self.kind {
            DeviceKind::Cpu(GraphKind::Ring) | DeviceKind::Memory(GraphKind::Ring) => {
                // show a number of 40% of max
                let val = self.max_val as f64 * 0.4;
                let percentage: u64 = ((val / self.max_val as f64) * 100.0) as u64;
                crate::svg_graph::ring(
                    &format!("{val}"),
                    &format!("{percentage}"),
                    &self.svg_colors,
                )
            }
            DeviceKind::Cpu(GraphKind::Line) | DeviceKind::Memory(GraphKind::Line) => {
                crate::svg_graph::line(
                    &VecDeque::from(DEMO_SAMPLES),
                    self.max_val,
                    &self.svg_colors,
                )
            }
            _ => panic!("ERROR: Wrong kind {:?}", self.kind),
        }
    }

    fn colors(&self) -> GraphColors {
        self.colors
    }

    fn set_colors(&mut self, colors: GraphColors) {
        self.colors = colors;
        self.svg_colors.set_colors(&colors);
    }

    fn color_choices(&self) -> Vec<(&'static str, ColorVariant)> {
        if self.kind == DeviceKind::Cpu(GraphKind::Line)
            || self.kind == DeviceKind::Memory(GraphKind::Line)
        {
            COLOR_CHOICES_LINE.into()
        } else {
            COLOR_CHOICES_RING.into()
        }
    }
}

impl SvgStat {
    pub fn new(kind: DeviceKind) -> Self {
        let mut system = System::new();
        system.refresh_memory();
        system.refresh_cpu_all();

        let max_val = match kind {
            DeviceKind::Cpu(_) => 100,
            _ => system.total_memory() / 1_073_741_824,
        };

        // value and percentage are pre-allocated and reused as they're changed often.
        let mut percentage = String::with_capacity(6);
        write!(percentage, "0").unwrap();

        let mut value = String::with_capacity(6);
        write!(value, "0").unwrap();

        let mut svg = SvgStat {
            samples: VecDeque::from(vec![0.0; MAX_SAMPLES]),
            max_val,
            colors: GraphColors::default(),
            system,
            kind,
            value,
            percentage,
            svg_colors: SvgColors::new(&GraphColors::default()),
        };
        svg.set_colors(GraphColors::default());
        svg
    }

    pub fn latest_sample(&self) -> f64 {
        *self.samples.back().unwrap_or(&0f64)
    }

    pub fn kind(&self) -> DeviceKind {
        self.kind
    }

    pub fn set_kind(&mut self, kind: DeviceKind) {
        match kind {
            DeviceKind::Cpu(_) | DeviceKind::Memory(_) => {
                self.kind = kind;
            }
            _ => {
                panic!("ERROR: Unexpected SvgKind variant: {:?}", kind);
            }
        }
    }

    pub fn to_string(&self) -> String {
        let current_val = self.latest_sample();
        let unit = match self.kind {
            DeviceKind::Cpu(_) => "%",
            DeviceKind::Memory(_) => " GB",
            _ => panic!("ERROR: Wrong kind {:?}", self.kind),
        };

        if current_val < 10.0 {
            format!("{:.2}{}", current_val, unit)
        } else if current_val < 100.0 {
            format!("{:.1}{}", current_val, unit)
        } else {
            format!("{}{}", current_val, unit)
        }
    }

    fn format_variable(&mut self) {
        self.value.clear();
        let current_val = self.latest_sample();
        if current_val < 10.0 {
            write!(self.value, "{:.2}", current_val).unwrap();
        } else if current_val < 100.0 {
            write!(self.value, "{:.1}", current_val).unwrap();
        } else {
            write!(self.value, "{}", current_val).unwrap();
        }

        let percentage: u64 = ((current_val / self.max_val as f64) * 100.0) as u64;
        self.percentage.clear();
        write!(self.percentage, "{percentage}").unwrap();
    }

    pub fn update(&mut self) {
        let new_val: f64 = match self.kind {
            DeviceKind::Cpu(_) => {
                self.system.refresh_cpu_usage();
                self.system
                    .cpus()
                    .iter()
                    .map(|p| f64::from(p.cpu_usage()))
                    .sum::<f64>()
                    / self.system.cpus().len() as f64
            }
            DeviceKind::Memory(_) => {
                self.system.refresh_memory();
                self.system.used_memory() as f64 / 1_073_741_824.0
            }
            DeviceKind::Network(_) => panic!("ERROR: Wrong kind {:?}", self.kind),
        };

        if self.samples.len() >= MAX_SAMPLES {
            self.samples.pop_front();
        }
        self.samples.push_back(new_val);

        if self.kind == DeviceKind::Cpu(GraphKind::Ring)
            || self.kind == DeviceKind::Memory(GraphKind::Ring)
        {
            self.format_variable();
        }
    }

    pub fn svg(&self) -> String {
        if self.kind == DeviceKind::Cpu(GraphKind::Ring)
            || self.kind == DeviceKind::Memory(GraphKind::Ring)
        {
            crate::svg_graph::ring(&self.value, &self.percentage, &self.svg_colors)
        } else {
            crate::svg_graph::line(&self.samples, self.max_val, &self.svg_colors)
        }
    }
}

const DEMO_SAMPLES: [f64; 21] = [
    0.0,
    12.689857482910156,
    12.642768859863281,
    12.615306854248047,
    12.658184051513672,
    12.65273666381836,
    12.626102447509766,
    12.624862670898438,
    12.613967895507813,
    12.619949340820313,
    19.061111450195313,
    21.691085815429688,
    21.810935974121094,
    21.28915786743164,
    22.041973114013672,
    21.764171600341797,
    21.89263916015625,
    15.258216857910156,
    14.770732879638672,
    14.496528625488281,
    13.892818450927734,
];
