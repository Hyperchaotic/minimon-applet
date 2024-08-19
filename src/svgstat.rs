use crate::config::GraphColors;

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
    colors: GraphColors,
    svg: String,
}

impl SvgStat {
    pub fn new(max_val: u64) -> Self {
        let mut svg = SvgStat {
            current_val: 0.0,
            max_val,
            colors: GraphColors::default(),
            svg: String::with_capacity(SVG_LEN),
        };
        svg.generate_svg();
        svg
    }

    pub fn set_variable(&mut self, val: f64) {
        if self.current_val != val {
            self.current_val = val;
            self.generate_svg();
        }
    }

    pub fn set_colors(&mut self, colors: &GraphColors) {
        self.colors = *colors;
        self.generate_svg();
    }

    pub fn colors(&self) -> GraphColors {
        self.colors
    }

    pub fn svg(&self) -> String {
        self.svg.clone()
    }

    fn generate_svg(&mut self) {
        let formatted_val;
        if self.current_val < 10.0 {
            formatted_val = format!("{:.2}", self.current_val);
        } else if self.current_val < 100.0 {
            formatted_val = format!("{:.1}", self.current_val);
        } else {
            formatted_val = format!("{}", self.current_val);
        }

        #[allow(clippy::cast_possible_truncation)]
        let percentage: u64 = ((self.current_val / self.max_val as f64) * 100.0) as u64;

        self.svg.clear();
        self.svg.push_str(SVGSTATSTART);
        self.svg.push_str(&format!(
            "fill=\"{}\" stroke=\"{}\"",
            self.colors.background_to_string(),
            self.colors.ringback_to_string()
        ));

        self.svg.push_str(SVGSTATPART2);
        self.svg.push_str(&self.colors.ringfront_to_string());
        self.svg.push_str(SVGSTATPART3);
        self.svg.push_str(&format!("{percentage}"));
        self.svg.push_str(SVGSTATPART4);
        self.svg
            .push_str(&format!(" fill: {};", self.colors.text_to_string()));
        self.svg.push_str(SVGSTATPART5);
        self.svg.push_str(&formatted_val);
        self.svg.push_str(SVGSTATPART6);
    }
}
