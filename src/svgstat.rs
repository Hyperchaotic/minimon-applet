use sysinfo::System;

use crate::{
    colorpicker::DemoSvg,
    config::{SvgColors, SvgKind},
};
use std::fmt::Write;

#[derive(Debug)]

pub struct SvgStat {
    current_val: f64,
    max_val: u64,
    colors: SvgColors,
    system: System,
    kind: SvgKind,

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
        // show a number of 40% of max 
        let val = self.max_val as f64 * 0.4;
        let percentage: u64 = ((val / self.max_val as f64) * 100.0) as u64;
        self.svg_compose(&format!("{val}"), &format!("{percentage}"))
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
}

impl SvgStat {
    pub fn new(kind: SvgKind) -> Self {
        let mut system = System::new();
        system.refresh_memory();
        system.refresh_cpu_all();

        let max_val = if kind == SvgKind::Cpu {
            100
        } else {
            system.total_memory() / 1_073_741_824
        };

        // value and percentage are pre-allocated and reused as they're changed often.
        let mut percentage = String::with_capacity(6);
        write!(percentage, "0").unwrap();

        let mut value = String::with_capacity(6);
        write!(value, "0").unwrap();

        let mut svg = SvgStat {
            current_val: 0.0,
            max_val: max_val,
            colors: SvgColors::default(),
            system: system,
            kind: kind,
            value,
            percentage,
            ringfront_color: String::new(),
            text_color: String::new(),
            circle_colors: String::new(),
        };
        svg.svg_set_colors(SvgColors::default());
        svg
    }

    fn format_variable(&mut self) {
        self.value.clear();
        if self.current_val < 10.0 {
            write!(self.value, "{:.2}", self.current_val).unwrap();
        } else if self.current_val < 100.0 {
            write!(self.value, "{:.1}", self.current_val).unwrap();
        } else {
            write!(self.value, "{}", self.current_val).unwrap();
        }

        let percentage: u64 = ((self.current_val / self.max_val as f64) * 100.0) as u64;
        self.percentage.clear();
        write!(self.percentage, "{percentage}").unwrap();
    }

    pub fn update(&mut self) {

        let mut new_val = 0.0;
        if self.kind == SvgKind::Cpu {
            self.system.refresh_cpu_usage();
            new_val = self
                .system
                .cpus()
                .iter()
                .map(|p| f64::from(p.cpu_usage()))
                .sum::<f64>()
                / self.system.cpus().len() as f64;
        }

        if self.kind == SvgKind::Memory {
            self.system.refresh_memory();
            new_val = self.system.used_memory() as f64 / 1_073_741_824.0;
        }

        if new_val != self.current_val {
            self.current_val = new_val;
            self.format_variable();
        }
    }

    pub fn value(&self) -> f64 {
        self.current_val
    }

    fn svg_compose(&self, value: &str, percentage: &str) -> String {
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

    pub fn svg(&self) -> String {
        self.svg_compose(&self.value, &self.percentage)
    }
}

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
  font-family: sans-serif;
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
