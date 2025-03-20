use sysinfo::System;

use crate::{
    colorpicker::DemoGraph,
    config::{ColorVariant, DeviceKind, GraphColors, GraphKind},
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
    color1_hex: String,
    color2_hex: String,
    color3_hex: String,
    color4_hex: String,
}

impl DemoGraph for SvgStat {
    fn demo(&self) -> String {
        match self.kind {
            DeviceKind::Cpu(GraphKind::Ring) | DeviceKind::Memory(GraphKind::Ring) => {
                // show a number of 40% of max
                let val = self.max_val as f64 * 0.4;
                let percentage: u64 = ((val / self.max_val as f64) * 100.0) as u64;
                self.svg_compose_ring(&format!("{val}"), &format!("{percentage}"))
            }
            DeviceKind::Cpu(GraphKind::Line) | DeviceKind::Memory(GraphKind::Line) => {
                self.svg_compose_line(&VecDeque::from(DEMO_SAMPLES), self.max_val)
            }
            _ => panic!("ERROR: Wrong kind {:?}", self.kind),
        }
    }

    fn colors(&self) -> GraphColors {
        self.colors
    }

    fn set_colors(&mut self, colors: GraphColors) {
        self.colors = colors;
        self.color1_hex = colors.color1_as_string();
        self.color2_hex = colors.color2_as_string();
        self.color3_hex = colors.color3_as_string();
        self.color4_hex = colors.color4_as_string();
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
            color1_hex: String::new(),
            color2_hex: String::new(),
            color3_hex: String::new(),
            color4_hex: String::new(),
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

    fn svg_compose_ring(&self, value: &str, percentage: &str) -> String {
        let mut svg = String::with_capacity(SVG_LEN);
        svg.push_str(RINGSVG_1);
        svg.push_str(&self.color1_hex);
        svg.push_str(RINGSVG_1_1);
        svg.push_str(&self.color3_hex);
        svg.push_str(RINGSVG_2);
        svg.push_str(&self.color4_hex);
        svg.push_str(RINGSVG_3);
        svg.push_str(percentage);
        svg.push_str(RINGSVG_4);
        svg.push_str(&self.color2_hex);
        svg.push_str(RINGSVG_5);
        svg.push_str(value);
        svg.push_str(RINGSVG_6);

        svg
    }

    fn svg_compose_line(&self, samples: &VecDeque<f64>, max_y: u64) -> String {
        // Generate list of coordinates for line
        let scaling: f32 = 40.0 / max_y as f32;
        let indexed_string: String = samples
            .iter()
            .enumerate()
            .map(|(index, &value)| {
                let x = ((index * 2) + 1) as u32;
                let y = (41.0 - (scaling * value as f32)).round() as u32;
                format!("{},{}", x, y)
            })
            .collect::<Vec<String>>()
            .join(" ");

        let mut svg = String::with_capacity(LINE_LEN);
        svg.push_str(LINESVG_1);
        svg.push_str(&self.color1_hex);
        svg.push_str(LINESVG_2);
        svg.push_str(&self.color2_hex);
        svg.push_str(LINESVG_3);
        svg.push_str(LINESVG_4);
        svg.push_str(&self.color4_hex);
        svg.push_str(LINESVG_5);
        svg.push_str(&indexed_string);
        svg.push_str(LINESVG_6);
        svg.push_str(&self.color4_hex);
        svg.push_str(LINESVG_7);
        svg.push_str(&indexed_string);
        svg.push_str(LINESVG_8);
        svg.push_str(LINESVG_9);

        svg
    }

    pub fn svg(&self) -> String {
        if self.kind == DeviceKind::Cpu(GraphKind::Ring)
            || self.kind == DeviceKind::Memory(GraphKind::Ring)
        {
            self.svg_compose_ring(&self.value, &self.percentage)
        } else {
            self.svg_compose_line(&self.samples, self.max_val)
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

const LINESVG_1: &str =
    "<svg width=\"42\" height=\"42\" viewBox=\"0 0 42 42\" xmlns=\"http://www.w3.org/2000/svg\">\n\
<rect x=\"0\" y=\"0\" width=\"42\" height=\"42\" opacity=\"1\" fill=\""; // background color

const LINESVG_2: &str = "\" stroke=\""; // frame color
const LINESVG_3: &str = "\"/>\n";

// polyline part
const LINESVG_4: &str = "<polyline fill=\"none\" opacity=\"1\" stroke=\""; // line color
const LINESVG_5: &str = "\" stroke-width=\"1\" points=\"";

// Polygon part
const LINESVG_6: &str = "\"/>\n<polygon opacity=\"0.3\" fill=\""; // polygon color
const LINESVG_7: &str = "\" points=\""; // polygonpoints
const LINESVG_8: &str = "  41,41 1,41\"/>";

// End
const LINESVG_9: &str = "</svg>";

const LINE_LEN: usize = 640; // Just for preallocation

const RINGSVG_1: &str = "
<svg viewBox=\"0 0 34 34\" xmlns=\"http://www.w3.org/2000/svg\">
 <path
    d=\"M17 1.0845
      a 15.9155 15.9155 0 0 1 0 31.831
      a 15.9155 15.9155 0 0 1 0 -31.831\"
      fill=\"";

const RINGSVG_1_1: &str = "\" stroke=\"";

const RINGSVG_2: &str = "\"\nstroke-width=\"2\"
  />
  <path
    d=\"M17 32.831
      a 15.9155 15.9155 0 0 1 0 -31.831
      a 15.9155 15.9155 0 0 1 0 31.831\"
    fill=\"none\"
    stroke=\"";

const RINGSVG_3: &str = "\"
    stroke-width=\"2\"
    stroke-dasharray=\"";

const RINGSVG_4: &str = ", 100\"
  />
  <style>
.percentage {
 fill: ";

const RINGSVG_5: &str = ";
  font-family: \"Noto Sans\", sans-serif;
  font-size: 1.2em;
  text-anchor: middle;
}
</style>
  <text x=\"17\" y=\"22.35\" class=\"percentage\">";

const RINGSVG_6: &str = "</text></svg>";
const SVG_LEN: usize = 680; // For preallocation
