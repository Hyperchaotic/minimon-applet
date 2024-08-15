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

pub struct SvgStat {
    current_val: f64,
    max_val: u64,
    colors: GraphColors,
    svg_len: usize,
}

impl SvgStat {
    pub fn new(max_val: u64) -> Self {
        SvgStat {
            current_val: 0.0,
            max_val,
            colors: GraphColors::default(),
            svg_len: SVGSTATSTART.len()
                + SVGSTATPART2.len()
                + SVGSTATPART3.len()
                + SVGSTATPART4.len()
                + SVGSTATPART5.len()
                + SVGSTATPART6.len()
                + 20,
        }
    }

    pub fn set_variable(&mut self, val: f64) {
        self.current_val = val;
    }

    pub fn set_colors(&mut self, colors: &GraphColors) {
        self.colors = colors.clone();
    }

    pub fn to_string(&self) -> String {
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

        let mut svg = String::with_capacity(self.svg_len);
        svg.push_str(SVGSTATSTART);
        svg.push_str(&format!(
            "fill=\"{}\" stroke=\"{}\"",
            self.colors.background_to_string(),
            self.colors.ringback_to_string()
        ));

        svg.push_str(SVGSTATPART2);
        svg.push_str(&self.colors.ringfront_to_string());
        svg.push_str(SVGSTATPART3);
        svg.push_str(&format!("{percentage}"));
        svg.push_str(SVGSTATPART4);
        svg.push_str(&format!(" fill: {};", self.colors.text_to_string()));
        svg.push_str(SVGSTATPART5);
        svg.push_str(&formatted_val);
        svg.push_str(SVGSTATPART6);
        /*
        println!(
            " ================================================= \n {}",
            &svg
        ); */

        svg
    }
}
