const SVGSTATSTART: &'static str = "
<svg height=\"36\" width=\"36\" viewBox=\"0 0 36 36\" xmlns=\"http://www.w3.org/2000/svg\">
 <path
    d=\"M18 2.0845
      a 15.9155 15.9155 0 0 1 0 31.831
      a 15.9155 15.9155 0 0 1 0 -31.831\"
    fill=\"#1b1b1b\"
    stroke=\"#eee\"
    stroke-width=\"2\"
  />
  <path
    d=\"M18 33.9155
      a 15.9155 15.9155 0 0 1 0 -31.831
      a 15.9155 15.9155 0 0 1 0 31.831\"
    fill=\"none\" 
    stroke=\"";

const SVGSTATPART2: &'static str = "\"
    stroke-width=\"2\"
    stroke-dasharray=\"";

const SVGSTATPART3: &'static str = ", 100\"
  />
  <style>
.percentage {
  fill: white;
  font-family: sans-serif;
  font-size: 1.2em;
  text-anchor: middle;
}
</style>
  <text x=\"18\" y=\"23.35\" class=\"percentage\">";

const SVGSTATPART4: &'static str = "</text></svg>";

pub struct SvgStat {
    current_val: f64,
    max_val: f64,
    color: String,
}

impl SvgStat {
    pub fn new(color: &str, max_val: f64) -> Self {
        SvgStat {
            current_val: 0.0,
            max_val: max_val,
            color: color.to_string(),
        }
    }

    pub fn set_variable(&mut self, val: f64) {
        self.current_val = val;
    }

    pub fn to_string(&self) -> String {
        let formated_val;
        if self.current_val < 10.0 {
            formated_val = format!("{:.2}", self.current_val);
        } else if self.current_val < 100.0 {
            formated_val = format!("{:.1}", self.current_val);
        } else {
            formated_val = format!("{}", self.current_val);
        }

        let percentage: u64 = ((self.current_val / self.max_val as f64) * 100 as f64) as u64;

        SVGSTATSTART.to_owned()
            + &self.color
            + SVGSTATPART2
            + &format!("{}", percentage)
            + SVGSTATPART3
            + &formated_val
            + SVGSTATPART4
    }
}
