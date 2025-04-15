use std::collections::VecDeque;

use cosmic::cosmic_theme::palette::Srgba;

use crate::config::GraphColors;

use std::fmt::Write;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgColors {
    pub color1: String,
    pub color2: String,
    pub color3: String,
    pub color4: String,
}

impl From<GraphColors> for SvgColors {
    fn from(graph_colors: GraphColors) -> Self {
        fn to_hex(color: Srgba<u8>) -> String {
            format!(
                "#{:02X}{:02X}{:02X}{:02X}",
                color.red, color.green, color.blue, color.alpha
            )
        }

        SvgColors {
            color1: to_hex(graph_colors.color1),
            color2: to_hex(graph_colors.color2),
            color3: to_hex(graph_colors.color3),
            color4: to_hex(graph_colors.color4),
        }
    }
}

impl SvgColors {
    pub fn new(colors: &GraphColors) -> SvgColors {
        (*colors).into()
    }

    pub fn set_colors(&mut self, colors: &GraphColors) {
        *self = (*colors).into();
    }
}

pub fn ring(value: &str, percentage: &str, color: &SvgColors) -> String {
    let mut svg = String::with_capacity(RINGSVG_LEN);
    svg.push_str(RINGSVG_1);
    svg.push_str(&color.color1);
    svg.push_str(RINGSVG_1_1);
    svg.push_str(&color.color3);
    svg.push_str(RINGSVG_2);
    svg.push_str(&color.color4);
    svg.push_str(RINGSVG_3);
    svg.push_str(percentage);
    svg.push_str(RINGSVG_4);
    svg.push_str(&color.color2);
    svg.push_str(RINGSVG_5);
    svg.push_str(value);
    svg.push_str(RINGSVG_6);

    svg
}

pub fn line(samples: &VecDeque<f64>, max_y: u64, colors: &SvgColors) -> String {
    // Generate list of coordinates for line

    let scaling: f32 = 40.0 / max_y as f32;
    let est_len = samples.len() * 10; // Rough estimate: each pair + separator

    let indexed_string = samples.iter().enumerate().fold(
        String::with_capacity(est_len),
        |mut acc, (index, &value)| {
            let x = ((index * 2) + 1) as u32;
            let y = (41.0 - (scaling * value as f32)).round() as u32;
            if index > 0 {
                acc.push(' ');
            }
            write!(&mut acc, "{},{}", x, y).unwrap();
            acc
        },
    );

    let mut svg = String::with_capacity(LINE_LEN);
    svg.push_str(LINESVG_1);
    svg.push_str(&colors.color1);
    svg.push_str(LINESVG_2);
    svg.push_str(&colors.color2);
    svg.push_str(LINESVG_3);
    svg.push_str(LINESVG_4);
    svg.push_str(&colors.color4);
    svg.push_str(LINESVG_5);
    svg.push_str(&indexed_string);
    svg.push_str(LINESVG_6);
    svg.push_str(&colors.color4);
    svg.push_str(LINESVG_7);
    svg.push_str(&indexed_string);
    svg.push_str(LINESVG_8);
    svg.push_str(LINESVG_9);

    svg
}

pub fn double_line(
    samples: &VecDeque<u64>,
    samples2: &VecDeque<u64>,
    graph_samples: usize,
    colors: &SvgColors,
    max_y: Option<u64>,
) -> String {
    assert!(samples.len() == samples2.len());

    let len = samples.len();

    let start = len.saturating_sub(graph_samples);

    let max = max_y.unwrap_or_else(|| {
        let calculated_max = samples
            .iter()
            .chain(samples2.iter())
            .copied()
            .max()
            .unwrap_or(40);
        std::cmp::max(40, calculated_max) // Ensure min value is 40
    });

    // Generate list of coordinates for line
    let est_len = (samples.len() - start) * 10;
    let scaling: f64 = 40.0 / max as f64;
    let (indexed_string, indexed_string2) = samples
        .iter()
        .skip(start)
        .zip(samples2.iter())
        .enumerate()
        .fold(
            (
                String::with_capacity(est_len),
                String::with_capacity(est_len),
            ),
            |(mut acc1, mut acc2), (index, (&value1, &value2))| {
                let x = ((index * 2) + 1) as u32;
                let y1 = (41.0 - (scaling * value1 as f64)).round() as u32;
                let y2 = (41.0 - (scaling * value2 as f64)).round() as u32;
                write!(&mut acc1, "{},{} ", x, y1).unwrap();
                write!(&mut acc2, "{},{} ", x, y2).unwrap();
                (acc1, acc2)
            },
        );

    let mut svg = String::with_capacity(DBLLINESVG_LEN);
    svg.push_str(DBLLINESVG_1);
    svg.push_str(&colors.color1);
    svg.push_str(DBLLINESVG_2);
    svg.push_str(&colors.color4);
    svg.push_str(DBLLINESVG_3);

    //First graph and polygon
    svg.push_str(DBLLINESVG_4);
    svg.push_str(&colors.color2);
    svg.push_str(DBLLINESVG_5);
    svg.push_str(&indexed_string);
    svg.push_str(DBLLINESVG_6);
    svg.push_str(&colors.color2);
    svg.push_str(DBLLINESVG_7);
    svg.push_str(&indexed_string);
    svg.push_str(DBLLINESVG_8);

    //Second graph and polygon
    svg.push_str(DBLLINESVG_4);
    svg.push_str(&colors.color3);
    svg.push_str(DBLLINESVG_5);
    svg.push_str(&indexed_string2);
    svg.push_str(DBLLINESVG_6);
    svg.push_str(&colors.color3);
    svg.push_str(DBLLINESVG_7);
    svg.push_str(&indexed_string2);
    svg.push_str(DBLLINESVG_8);

    svg.push_str(DBLLINESVG_9);

    svg
}

pub fn line_adaptive(
    samples: &VecDeque<u64>,
    graph_samples: usize,
    colors: &SvgColors,
    max_y: Option<u64>,
) -> String {
    let len = samples.len();
    let start = len.saturating_sub(graph_samples);

    let max = max_y.unwrap_or_else(|| {
        let calculated_max = samples.iter().copied().max().unwrap_or(40);
        std::cmp::max(40, calculated_max) // Ensure min value is 40
    });

    // Generate list of coordinates for line
    let est_len = (samples.len() - start) * 10;
    let scaling: f64 = 40.0 / max as f64;
    let indexed_string = samples.iter().skip(start).enumerate().fold(
        String::with_capacity(est_len),
        |mut acc, (index, &value)| {
            let x = ((index * 2) + 1) as u32;
            let y = (41.0 - (scaling * value as f64)).round() as u32;
            write!(&mut acc, "{},{} ", x, y).unwrap();
            acc
        },
    );

    let mut svg = String::with_capacity(DBLLINESVG_LEN);
    svg.push_str(DBLLINESVG_1);
    svg.push_str(&colors.color1);
    svg.push_str(DBLLINESVG_2);
    svg.push_str(&colors.color4);
    svg.push_str(DBLLINESVG_3);

    //First graph and polygon
    svg.push_str(DBLLINESVG_4);
    svg.push_str(&colors.color2);
    svg.push_str(DBLLINESVG_5);
    svg.push_str(&indexed_string);
    svg.push_str(DBLLINESVG_6);
    svg.push_str(&colors.color2);
    svg.push_str(DBLLINESVG_7);
    svg.push_str(&indexed_string);
    svg.push_str(DBLLINESVG_8);

    svg.push_str(DBLLINESVG_9);

    svg
}

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
const RINGSVG_LEN: usize = 680; // For preallocation

const DBLLINESVG_1: &str = "
<svg width=\"42\" height=\"42\" viewBox=\"0 0 42 42\" xmlns=\"http://www.w3.org/2000/svg\">\n\
<rect x=\"0\" y=\"0\" width=\"42\" height=\"42\" opacity=\"1\" fill=\""; // background color

const DBLLINESVG_2: &str = "\" stroke=\""; // frame color
const DBLLINESVG_3: &str = "\"/>\n";

// line
const DBLLINESVG_4: &str = "<polyline fill=\"none\" opacity=\"1\" stroke=\""; // line color

const DBLLINESVG_5: &str = "\" stroke-width=\"1\" points=\"";

// Polygon
const DBLLINESVG_6: &str = "\"/>\
<polygon opacity=\"0.3\" fill=\""; // polygon color

const DBLLINESVG_7: &str = "\" points=\""; // polygonpoints

const DBLLINESVG_8: &str = "  41,41 1,41\"/>";

const DBLLINESVG_9: &str = "</svg>";

const DBLLINESVG_LEN: usize = 1000; // For preallocation
                                    /*
                                    pub fn dbl_circle(
                                        samples: &VecDeque<u64>,
                                        samples2: &VecDeque<u64>,
                                        graph_samples: usize,
                                        colors: &SvgColors,
                                    ) -> String {
                                        let mut dl: u64 = 0;
                                        let mut ul: u64 = 0;


                                        if self.max_val > 0 && !self.download.is_empty() {
                                            let scaling_dl: f32 = 94.0 / self.max_val as f32;
                                            let scaling_ul: f32 = 69.0 / self.max_val as f32;

                                            dl = *self.download.get(self.download.len() - 1).unwrap_or(&0u64);
                                            ul = *self.upload.get(self.upload.len() - 1).unwrap_or(&0u64);
                                            dl = (dl as f32 * scaling_dl) as u64;
                                            ul = (ul as f32 * scaling_ul) as u64;
                                        }

                                        let background = "none";
                                        let strokebg = "white";
                                        let outerstrokefg = "blue";
                                        let outerpercentage = dl.to_string();
                                        let innerstrokefg = "red";
                                        let innerpercentage = ul.to_string();
                                        let mut svg = String::with_capacity(SVG_LEN);
                                        svg.push_str(DBLCIRCLESTART);
                                        svg.push_str(&background);
                                        svg.push_str(DBLCIRCLEPART2);
                                        svg.push_str(&strokebg);
                                        svg.push_str(DBLCIRCLEPART3);
                                        svg.push_str(&outerstrokefg);
                                        svg.push_str(DBLCIRCLEPART4);
                                        svg.push_str(&outerpercentage);
                                        svg.push_str(DBLCIRCLEPART5);
                                        svg.push_str(&strokebg);
                                        svg.push_str(DBLCIRCLEPART6);
                                        svg.push_str(&innerstrokefg);
                                        svg.push_str(DBLCIRCLEPART7);
                                        svg.push_str(&innerpercentage);
                                        svg.push_str(DBLCIRCLEPART8);

                                        svg
                                    }

                                    const SVG_LEN: usize = DBLCIRCLESTART.len()
                                        + DBLCIRCLEPART2.len()
                                        + DBLCIRCLEPART3.len()
                                        + DBLCIRCLEPART4.len()
                                        + DBLCIRCLEPART5.len()
                                        + DBLCIRCLEPART6.len()
                                        + DBLCIRCLEPART7.len()
                                        + DBLCIRCLEPART8.len()
                                        + 40;

                                    const DBLCIRCLESTART: &str = "<svg viewBox=\"0 0 34 34\" xmlns=\"http://www.w3.org/2000/svg\">
                                      <path d=\"M17 2.0845
                                          a 13.9155 13.9155 0 0 1 0 29.831
                                          a 13.9155 13.9155 0 0 1 0 -29.831\" fill=\""; // background

                                    const DBLCIRCLEPART2: &str = "\" stroke=\""; // outerstrokebg

                                    const DBLCIRCLEPART3: &str = "\" stroke-width=\"4\"/>
                                      <path d=\"M17 31.931
                                          a 13.9155 13.9155 0 0 1 0 -29.831
                                          a 13.9155 13.9155 0 0 1 0 29.931\" fill=\"none\" stroke=\""; //outerstrokefg

                                    const DBLCIRCLEPART4: &str = "\" stroke-width=\"4\" stroke-dasharray=\""; //outerpercentage

                                    const DBLCIRCLEPART5: &str = ", 94\"/>
                                      <path d=\"M17 28
                                          a 7.9155 7.9155 0 0 1 0 -22
                                          a 7.9155 7.9155 0 0 1 0 22\" fill=\"none\" stroke=\""; //innerstrokebg

                                    const DBLCIRCLEPART6: &str = "\" stroke-width=\"3.7\"/>
                                      <path d=\"M17 28
                                          a 7.9155 7.9155 0 0 1 0 -22
                                          a 7.9155 7.9155 0 0 1 0 22\" fill=\"none\" stroke=\""; //innerstrokefg

                                    const DBLCIRCLEPART7: &str = "\" stroke-width=\"3.7\" stroke-dasharray=\""; //innerpercentage

                                    const DBLCIRCLEPART8: &str = ", 69\"/></svg>";
                                    */
/*
<svg viewBox="0 0 34 34" xmlns="http://www.w3.org/2000/svg">
  <path d="M17 2.0845
      a 13.9155 13.9155 0 0 1 0 29.831
      a 13.9155 13.9155 0 0 1 0 -29.831" fill="none" stroke="#eee" stroke-width="4"/>

  <path d="M17 31.931
      a 13.9155 13.9155 0 0 1 0 -29.831
      a 13.9155 13.9155 0 0 1 0 29.931" fill="none" stroke="blue" stroke-width="4" stroke-dasharray="60, 94"/>

  <path d="M17 28
      a 7.9155 7.9155 0 0 1 0 -22
      a 7.9155 7.9155 0 0 1 0 22" fill="none" stroke="#eee" stroke-width="3.7"/>

  <path d="M17 28
      a 7.9155 7.9155 0 0 1 0 -22
      a 7.9155 7.9155 0 0 1 0 22" fill="none" stroke="red" stroke-width="3.7" stroke-dasharray="30, 69"/>
</svg>
*/
