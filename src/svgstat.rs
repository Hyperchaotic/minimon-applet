use plotters::prelude::*;
use plotters::{chart::ChartBuilder, prelude::SVGBackend, style::RGBColor};
use sysinfo::System;

use crate::{
    colorpicker::DemoSvg,
    config::{SvgColorVariant, SvgColors, SvgDevKind, SvgGraphKind},
};
use std::{collections::VecDeque, fmt::Write};

const MAX_SAMPLES: usize = 21;

const COLOR_CHOICES_RING: [(&str, SvgColorVariant); 4] = [
    ("Ring1.  ", SvgColorVariant::Color4),
    ("Ring2.  ", SvgColorVariant::Color3),
    ("Back.  ", SvgColorVariant::Color1),
    ("Text.", SvgColorVariant::Color2),
];

const COLOR_CHOICES_LINE: [(&str, SvgColorVariant); 3] = [
    ("Graph.  ", SvgColorVariant::Color4),
    ("Back.  ", SvgColorVariant::Color1),
    ("Frame.  ", SvgColorVariant::Color2),
];

#[derive(Debug)]
pub struct SvgStat {
    samples: VecDeque<f64>,
    max_val: u64,
    colors: SvgColors,
    system: System,
    kind: SvgDevKind,

    /// current value cpu/ram load shown.
    value: String,
    /// the percentage of the ring to be filled
    percentage: String,
    /// colors
    ringfront_color: String,
    text_color: String,
    circle_colors: String,
}

impl DemoSvg for SvgStat {
    fn svg_demo(&self) -> String {
        match self.kind {
            SvgDevKind::Cpu(SvgGraphKind::Ring) | SvgDevKind::Memory(SvgGraphKind::Ring) => {
                // show a number of 40% of max
                let val = self.max_val as f64 * 0.4;
                let percentage: u64 = ((val / self.max_val as f64) * 100.0) as u64;
                self.svg_compose_ring(&format!("{val}"), &format!("{percentage}"))
            }
            SvgDevKind::Cpu(SvgGraphKind::Line) | SvgDevKind::Memory(SvgGraphKind::Line) => {
                self.svg_compose_line(&VecDeque::from(DEMO_SAMPLES), self.max_val)
            }
            _ => panic!("ERROR: Wrong kind {:?}", self.kind),
        }
    }

    fn svg_colors(&self) -> SvgColors {
        self.colors
    }

    fn svg_set_colors(&mut self, colors: SvgColors) {
        self.colors = colors;
        self.ringfront_color = colors.color4_to_string();
        self.text_color = format!(" fill:{};", &colors.color2_to_string());
        self.circle_colors = format!(
            "fill=\"{}\" stroke=\"{}\"",
            colors.color1_to_string(),
            colors.color3_to_string()
        );
    }

    fn svg_color_choices(&self) -> Vec<(&'static str, SvgColorVariant)> {
        if self.kind == SvgDevKind::Cpu(SvgGraphKind::Line)
            || self.kind == SvgDevKind::Memory(SvgGraphKind::Line)
        {
            COLOR_CHOICES_LINE.into()
        } else {
            COLOR_CHOICES_RING.into()
        }
    }
}

impl SvgStat {
    pub fn new(kind: SvgDevKind) -> Self {
        let mut system = System::new();
        system.refresh_memory();
        system.refresh_cpu_all();

        let max_val = match kind {
            SvgDevKind::Cpu(_) => 100,
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
            colors: SvgColors::default(),
            system,
            kind,
            value,
            percentage,
            ringfront_color: String::new(),
            text_color: String::new(),
            circle_colors: String::new(),
        };
        svg.svg_set_colors(SvgColors::default());
        svg
    }

    pub fn latest_sample(&self) -> f64 {
        *self.samples.back().unwrap_or(&0f64)
    }

    pub fn kind(&self) -> SvgDevKind {
        self.kind
    }

    pub fn set_kind(&mut self, kind: SvgDevKind) {
        match kind {
            SvgDevKind::Cpu(_) | SvgDevKind::Memory(_) => {
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
            SvgDevKind::Cpu(_) => "%",
            SvgDevKind::Memory(_) => "GB",
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
            SvgDevKind::Cpu(_) => {
                self.system.refresh_cpu_usage();
                self.system
                    .cpus()
                    .iter()
                    .map(|p| f64::from(p.cpu_usage()))
                    .sum::<f64>()
                    / self.system.cpus().len() as f64
            }
            SvgDevKind::Memory(_) => {
                self.system.refresh_memory();
                self.system.used_memory() as f64 / 1_073_741_824.0
            }
            SvgDevKind::Network(_) => panic!("ERROR: Wrong kind {:?}", self.kind),
        };

        if self.samples.len() >= MAX_SAMPLES {
            self.samples.pop_front();
        }
        self.samples.push_back(new_val);

        if self.kind == SvgDevKind::Cpu(SvgGraphKind::Ring)
            || self.kind == SvgDevKind::Memory(SvgGraphKind::Ring)
        {
            self.format_variable();
        }
    }

    fn svg_compose_ring(&self, value: &str, percentage: &str) -> String {
        let mut svg = String::with_capacity(SVG_LEN);
        svg.push_str(SVGSTATSTART);
        svg.push_str(&self.circle_colors);
        svg.push_str(SVGSTATPART2);
        svg.push_str(&self.ringfront_color);
        svg.push_str(SVGSTATPART3);
        svg.push_str(percentage);
        svg.push_str(SVGSTATPART4);
        svg.push_str(&self.text_color);
        svg.push_str(SVGSTATPART5);
        svg.push_str(value);
        svg.push_str(SVGSTATPART6);

        svg
    }

    fn svg_compose_line(&self, samples: &VecDeque<f64>, max_y: u64) -> String {
        let mut sname: String = String::new();
        {
            let bg = self.colors.get_color(SvgColorVariant::Color1);
            let root = SVGBackend::with_string(&mut sname, (40, 40)).into_drawing_area();
            root.fill(&RGBColor(bg.red, bg.green, bg.blue)).unwrap();
            let root = root.margin(0, 0, 0, 0);
            // After this point, we should be able to construct a chart context
            let mut chart = ChartBuilder::on(&root)
                // Finally attach a coordinate on the drawing area and make a chart context
                .build_cartesian_2d(0f32..40f32, 0f32..40f32)
                .unwrap();

            // Then we can draw a mesh
            chart
                .configure_mesh()
                .disable_x_axis()
                .disable_y_axis()
                .disable_mesh()
                .draw()
                .unwrap();

            let col = self.colors.get_color(SvgColorVariant::Color2);
            let rect = Rectangle::new(
                [(0, 0), (40, 40)],
                ShapeStyle {
                    color: RGBAColor(col.red, col.green, col.blue, 1.0),
                    filled: false,
                    stroke_width: 1,
                },
            );
            root.draw(&rect).unwrap();

            if !samples.is_empty() {
                let scaling: f32 = 39.0 / max_y as f32;

                let indexed_vec: Vec<(f32, f32)> = samples
                    .iter()
                    .enumerate()
                    .map(|(index, &value)| ((index * 2) as f32, scaling * value as f32))
                    .collect();

                let col = self.colors.get_color(SvgColorVariant::Color4);
                let line_color = RGBColor(col.red, col.green, col.blue);
                let _ = chart.draw_series(AreaSeries::new(
                    indexed_vec.clone(),
                    0.0,
                    line_color.mix(0.3), // Rust color with some transparency
                ));

                let _ = chart.draw_series(LineSeries::new(indexed_vec, &line_color));
            }

            let _ = root.present();
        }
        sname
    }

    pub fn svg(&self) -> String {
        if self.kind == SvgDevKind::Cpu(SvgGraphKind::Ring)
            || self.kind == SvgDevKind::Memory(SvgGraphKind::Ring)
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

const SVGSTATSTART: &str = "
<svg viewBox=\"0 0 34 34\" xmlns=\"http://www.w3.org/2000/svg\">
 <path
    d=\"M17 1.0845
      a 15.9155 15.9155 0 0 1 0 31.831
      a 15.9155 15.9155 0 0 1 0 -31.831\"
      ";

const SVGSTATPART2: &str = "
        stroke-width=\"2\"
  />
  <path
    d=\"M17 32.831
      a 15.9155 15.9155 0 0 1 0 -31.831
      a 15.9155 15.9155 0 0 1 0 31.831\"
    fill=\"none\"
    stroke=\"";

const SVGSTATPART3: &str = "\"
    stroke-width=\"2\"
    stroke-dasharray=\"";

const SVGSTATPART4: &str = ", 100\"
  />
  <style>
.percentage {
 ";
const SVGSTATPART5: &str = "
  font-family: \"Noto Sans\", sans-serif;
  font-size: 1.2em;
  text-anchor: middle;
}
</style>
  <text x=\"17\" y=\"22.35\" class=\"percentage\">";

const SVGSTATPART6: &str = "</text></svg>";
const SVG_LEN: usize = SVGSTATSTART.len()
    + SVGSTATPART2.len()
    + SVGSTATPART3.len()
    + SVGSTATPART4.len()
    + SVGSTATPART5.len()
    + SVGSTATPART6.len()
    + 40;
