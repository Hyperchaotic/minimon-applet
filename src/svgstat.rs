use crate::config::CircleGraphColors;

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

#[derive(Debug, Default, Clone, PartialEq)]

pub struct SvgStat {
    current_val: f64,
    max_val: u64,
    colors: CircleGraphColors,

    /// current value cpu/ram load shown. 
    value: String,
    /// the percentage of the ring to be filled
    percentage: String,
    /// colors
    ringfront_color: String,
    text_color: String,
    circle_colors: String,
}

use std::fmt::Write;

impl SvgStat {
    pub fn new(max_val: u64) -> Self {

        // value and percentage are pre-allocated and reused as they're changed often.
        let mut percentage = String::with_capacity(6);
        write!(percentage, "0").unwrap();

        let mut value = String::with_capacity(6);
        write!(value, "0").unwrap();

        let mut svg = SvgStat {
            current_val: 0.0,
            max_val,
            colors: CircleGraphColors::default(),
            value,
            percentage,
            ringfront_color: String::new(),
            text_color: String::new(),
            circle_colors: String::new(),
        };
        svg.set_colors(CircleGraphColors::default());
        svg
    }

    pub fn set_variable(&mut self, val: f64) {
        if self.current_val != val {

            self.current_val = val;

            self.value.clear();
            if self.current_val < 10.0 {
                write!(self.value, "{:.2}", self.current_val).unwrap();
            } else if self.current_val < 100.0 {
                write!(self.value, "{:.1}", self.current_val).unwrap();
            } else {
                write!(self.value, "{}", self.current_val).unwrap();
            }

            #[allow(clippy::cast_possible_truncation)]
            let percentage: u64 = ((self.current_val / self.max_val as f64) * 100.0) as u64;
            self.percentage.clear();
            write!(self.percentage, "{percentage}").unwrap();
            }
    }

    pub fn set_colors(&mut self, colors: CircleGraphColors) {
        self.colors = colors;
        self.ringfront_color = self.colors.ringfront_to_string();
        self.text_color = format!(" fill:{};", &self.colors.text_to_string());
        self.circle_colors = format!(
            "fill=\"{}\" stroke=\"{}\"",
            self.colors.background_to_string(),
            self.colors.ringback_to_string()
        );
    }

    pub fn colors(&self) -> CircleGraphColors {
        self.colors
    }

    pub fn svg(&self) -> String {

        let mut svg = String::with_capacity(SVG_LEN);
        svg.push_str(SVGSTATSTART);
        svg.push_str(&self.circle_colors);
        svg.push_str(SVGSTATPART2);
        svg.push_str(&self.ringfront_color);
        svg.push_str(SVGSTATPART3);
        svg.push_str(&self.percentage);
        svg.push_str(SVGSTATPART4);
        svg.push_str(&self.text_color);
        svg.push_str(SVGSTATPART5);
        svg.push_str(&self.value);
        svg.push_str(SVGSTATPART6);

        svg
    }
}
