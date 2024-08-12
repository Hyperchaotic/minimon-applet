
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
    
const SVGSTATMID: &'static str = "\"
    stroke-width=\"2\"
    stroke-dasharray=\"XXX, 100\"
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

const SVGSTATEND: &'static str = "</text></svg>";
const SVGSTATPAD: &'static str = "    ";
const SVGSTATGUARD: &'static str = "<!-- svg end -->";

pub struct SvgStat {
    // Current degrees of rotation
    //variable: f64,
    svg: String,
    // Location to modify the string in-place
    range_val: core::ops::Range<usize>,
    // Location to modify the circle in-place
    range_circle: core::ops::Range<usize>,
}

impl SvgStat {
    pub fn new(color: &str) -> Self {

      let color_len = color.len();
      let location = SVGSTATSTART.len()+color_len+SVGSTATMID.find("XXX").unwrap();
        let mut s = SvgStat {
      //      variable: 0.0,
            // We add extra spaces here so the string is long enough we can safely replace
            // the number in-place later, including the following xml, without writing
            // into unallocated memory.
            svg: SVGSTATSTART.to_owned() + color+SVGSTATMID+"   0" + SVGSTATEND + SVGSTATPAD + SVGSTATGUARD,
            range_val: SVGSTATSTART.len()+color_len+SVGSTATMID.len()..SVGSTATSTART.len()+color_len+SVGSTATMID.len() + 4 + SVGSTATEND.len() + SVGSTATPAD.len(),
            range_circle: location..location+3
        };

        // Making sure to format the svg, remove the XXX...
        s.set_variable(0.0, 100);

        s
    }

    // Updates the status (cpu load or memory use). Also updates the SVG string in-place.
    pub fn set_variable(&mut self, val: f64, total: u64) {

            let formated_val: String;

            let variable = val;
            let percentage: f64 = (val/total as f64)*100 as f64;

            println!("HHHHHHHHHHHHHHHHHH-->{}-{}-{}", val, total, percentage);

            if variable < 10.0 {
                formated_val = format!("{:.2}{}{}", variable, SVGSTATEND, SVGSTATPAD);
            } else if variable < 100.0 {
                formated_val = format!("{:.1}{}{}", variable, SVGSTATEND, SVGSTATPAD);
            } else {
                formated_val = format!("{}{}{}", variable, SVGSTATEND, SVGSTATPAD);
            }

            println!("BEFORE: {}", self.svg);

            // Be a lot simpler just building a new SVG without unsafe.
            // This is faster but not really necessary on a modern CPU.
            // The code is safe as long as the original SVG is not altered,
            // which is why it's included in this file.

            let formated_circle = format!("{:03}", percentage as u32);
            let range = self.range_val.clone();
            let range_circle = self.range_circle.clone();
            unsafe {
                core::ptr::copy_nonoverlapping(
                    formated_val.as_ptr(),
                    self.svg[range].as_mut_ptr(),
                    formated_val.len(),
                );
                core::ptr::copy_nonoverlapping(
                    formated_circle.as_ptr(),
                    self.svg[range_circle].as_mut_ptr(),
                    formated_circle.len(),
                );
            }
      
          println!("AFTER: {}", self.svg);


    }

    pub fn as_bytes(&self) -> &[u8] {
        self.svg.as_bytes()
    }
}
