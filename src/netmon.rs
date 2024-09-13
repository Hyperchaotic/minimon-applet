use plotters::prelude::*;
use std::collections::VecDeque;

use sysinfo::Networks;

use crate::{
    colorpicker::DemoSvg,
    config::{SvgColorVariant, SvgColors, SvgDevKind, SvgGraphKind},
};

const MAX_SAMPLES: usize = 30;
const GRAPH_SAMPLES: usize = 21;
const UNITS_SHORT: [&str; 5] = ["b", "K", "M", "G", "T"];
const UNITS_LONG: [&str; 5] = ["bps", "Kbps", "Mbps", "Gbps", "Tbps"];

const COLOR_CHOICES: [(&str, SvgColorVariant); 4] = [
    ("Down.  ", SvgColorVariant::Color2),
    ("Up.  ", SvgColorVariant::Color3),
    ("Back.  ", SvgColorVariant::Color1),
    ("Frame.", SvgColorVariant::Color4),
];

#[derive(Debug, PartialEq, Eq)]
pub enum UnitVariant {
    Short,
    Long,
}

#[derive(Debug)]
pub struct NetMon {
    networks: Networks,
    download: VecDeque<u64>,
    upload: VecDeque<u64>,
    max_y: Option<u64>,
    colors: SvgColors,
    kind: SvgDevKind,
}

impl DemoSvg for NetMon {
    fn svg_demo(&self) -> String {
        let download = VecDeque::from(DL_DEMO);
        let upload = VecDeque::from(UL_DEMO);
        self.svg_compose(&download, &upload, None)
    }

    fn svg_colors(&self) -> SvgColors {
        self.colors
    }

    fn svg_set_colors(&mut self, colors: SvgColors) {
        self.colors = colors;
    }

    fn svg_color_choices(&self) -> Vec<(&'static str, SvgColorVariant)> {
        COLOR_CHOICES.into()
    }
}

impl NetMon {
    pub fn new() -> Self {
        let mut networks = Networks::new_with_refreshed_list();
        networks.refresh();

        NetMon {
            networks,
            download: VecDeque::from(vec![0; MAX_SAMPLES]),
            upload: VecDeque::from(vec![0; MAX_SAMPLES]),
            max_y: None,
            colors: SvgColors::new(SvgDevKind::Network(SvgGraphKind::Line)),
            kind: SvgDevKind::Network(SvgGraphKind::Line),
        }
    }

    pub fn set_max_y(&mut self, max: Option<u64>) {
        self.max_y = max;
    }

    pub fn kind(&self) -> SvgDevKind {
        self.kind
    }

    pub fn set_kind(&mut self, kind: SvgDevKind) {
        match kind {
            SvgDevKind::Network(SvgGraphKind::Line) => (),
            _ => panic!("ERROR: Wrong kind {:?}", kind),
        }
    }

    /// Retrieve the amount of data transmitted since last update.
    pub fn update(&mut self) {
        self.networks.refresh();
        let mut dl = 0;
        let mut ul = 0;

        for (_, network) in &self.networks {
            dl += network.received() * 8;
            ul += network.transmitted() * 8;
        }

        if self.download.len() >= MAX_SAMPLES {
            self.download.pop_front();
        }
        self.download.push_back(dl);

        if self.upload.len() >= MAX_SAMPLES {
            self.upload.pop_front();
        }
        self.upload.push_back(ul);
    }

    fn makestr(val: u64, format: UnitVariant) -> String {
        let mut value = val as f64;
        let mut unit_index = 0;
        let units = if format==UnitVariant::Short { UNITS_SHORT } else { UNITS_LONG };

        // Find the appropriate unit
        while value >= 999.0 && unit_index < units.len() - 1 {
            value /= 1024.0;
            unit_index += 1;
        }

        if value < 10.0 {
            format!("{:.2}{}", value, units[unit_index])
        } else if value < 99.0 {
            format!("{:.1}{}", value, units[unit_index])
        } else {
            format!("{:.0}{}", value, units[unit_index])
        }
    }

    // Get bits per second
    pub fn get_bitrate_dl(&self, ticks_per_sec: usize) -> String {
        let len = self.download.len();
        let start = if ticks_per_sec > len { 0 } else { len - ticks_per_sec };
        // Sum the last `ticks` elements
        let bps = self.download.iter().skip(start).sum();
        NetMon::makestr(bps, UnitVariant::Long)
    }

    // Get bits per second
    pub fn get_bitrate_ul(&self, ticks_per_sec: usize) -> String {
        let len = self.upload.len();
        let start = if ticks_per_sec > len { 0 } else { len - ticks_per_sec };
        // Sum the last `ticks` elements
        let bps = self.upload.iter().skip(start).sum();
        NetMon::makestr(bps, UnitVariant::Long)
    }

    // Bits per tick
    pub fn dl_to_string(&self) -> String {
        let dl = if !self.download.is_empty() {
            *self.download.back().unwrap_or(&0u64)
        } else {
            0
        };
        NetMon::makestr(dl, UnitVariant::Short)
    }

    // Bits per tick
    pub fn ul_to_string(&self) -> String {
        let ul = if !self.upload.is_empty() {
            *self.upload.back().unwrap_or(&0u64)
        } else {
            0
        };
        NetMon::makestr(ul, UnitVariant::Short)
    }

    fn svg_compose(
        &self,
        download: &VecDeque<u64>,
        upload: &VecDeque<u64>,
        max_y: Option<u64>,
    ) -> String {
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

            let col = self.colors.get_color(SvgColorVariant::Color4);
            let rect = Rectangle::new(
                [(0, 0), (40, 40)],
                ShapeStyle {
                    color: RGBAColor(col.red, col.green, col.blue, 1.0),
                    filled: false,
                    stroke_width: 1,
                },
            );
            root.draw(&rect).unwrap();

            if !download.is_empty() {
                // Configured max or adaptive
                let max: u64 = if let Some(m) = max_y {
                    m
                } else {
                    *std::cmp::max(
                        download.iter().max().unwrap_or(&0),
                        upload.iter().max().unwrap_or(&0),
                    )
                };

                let scaling: f32 = 39.0 / max as f32;

                let dl_len = download.len();
                let dl_start = if dl_len > GRAPH_SAMPLES {
                    dl_len - GRAPH_SAMPLES
                } else {
                    0
                };

                let ul_len = upload.len();
                let ul_start = if ul_len > GRAPH_SAMPLES {
                    ul_len - GRAPH_SAMPLES
                } else {
                    0
                };

                let indexed_vec_dl: Vec<(f32, f32)> = download
                    .iter()
                    .skip(dl_start)
                    .enumerate()
                    .map(|(index, &value)| ((index * 2) as f32, scaling * value as f32))
                    .collect();

                let indexed_vec_ul: Vec<(f32, f32)> = upload
                    .iter()
                    .skip(ul_start)
                    .enumerate()
                    .map(|(index, &value)| ((index * 2) as f32, scaling * value as f32))
                    .collect();

                let dl = self.colors.get_color(SvgColorVariant::Color2);
                let ul = self.colors.get_color(SvgColorVariant::Color3);

                let dl_color = RGBColor(dl.red, dl.green, dl.blue);
                let ul_color = RGBColor(ul.red, ul.green, ul.blue);
                let _ = chart.draw_series(AreaSeries::new(
                    indexed_vec_dl.clone(),
                    0.0,
                    dl_color.mix(0.3), // Rust color with some transparency
                ));

                let _ = chart.draw_series(AreaSeries::new(
                    indexed_vec_ul.clone(),
                    0.0,
                    ul_color.mix(0.5), // Rust color with some transparency
                ));

                let _ = chart.draw_series(LineSeries::new(indexed_vec_dl, &dl_color));

                let _ = chart.draw_series(LineSeries::new(indexed_vec_ul, &ul_color));
            }

            let _ = root.present();
        }
        sname
    }

    pub fn svg(&self) -> String {
        self.svg_compose(&self.download, &self.upload, self.max_y)
    }
}

const DL_DEMO: [u64; 21] = [
    208, 2071, 0, 1056588, 912575, 912875, 912975, 912600, 1397, 1173024, 1228, 6910, 2493,
    1102101, 380, 2287, 1109656, 1541, 3798, 1132822, 68479,
];
const UL_DEMO: [u64; 21] = [
    0, 1687, 0, 9417, 9161, 838, 6739, 1561, 212372, 312372, 412372, 512372, 512372, 512372,
    412372, 312372, 112372, 864, 0, 8587, 760,
];

/*
    pub fn svg_circle(&self) -> String {
        let mut ul: u64 = 0;
        let mut dl: u64 = 0;

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
        svg.push_str(SVGSTATSTART);
        svg.push_str(&background);
        svg.push_str(SVGSTATPART2);
        svg.push_str(&strokebg);
        svg.push_str(SVGSTATPART3);
        svg.push_str(&outerstrokefg);
        svg.push_str(SVGSTATPART4);
        svg.push_str(&outerpercentage);
        svg.push_str(SVGSTATPART5);
        svg.push_str(&strokebg);
        svg.push_str(SVGSTATPART6);
        svg.push_str(&innerstrokefg);
        svg.push_str(SVGSTATPART7);
        svg.push_str(&innerpercentage);
        svg.push_str(SVGSTATPART8);

        svg
    }

const SVG_LEN: usize = SVGSTATSTART.len()
    + SVGSTATPART2.len()
    + SVGSTATPART3.len()
    + SVGSTATPART4.len()
    + SVGSTATPART5.len()
    + SVGSTATPART6.len()
    + SVGSTATPART7.len()
    + SVGSTATPART8.len()
    + 40;

const SVGSTATSTART: &str = "<svg viewBox=\"0 0 34 34\" xmlns=\"http://www.w3.org/2000/svg\">
  <path d=\"M17 2.0845
      a 13.9155 13.9155 0 0 1 0 29.831
      a 13.9155 13.9155 0 0 1 0 -29.831\" fill=\""; // background

const SVGSTATPART2: &str = "\" stroke=\""; // outerstrokebg

const SVGSTATPART3: &str = "\" stroke-width=\"4\"/>
  <path d=\"M17 31.931
      a 13.9155 13.9155 0 0 1 0 -29.831
      a 13.9155 13.9155 0 0 1 0 29.931\" fill=\"none\" stroke=\""; //outerstrokefg

const SVGSTATPART4: &str = "\" stroke-width=\"4\" stroke-dasharray=\""; //outerpercentage

const SVGSTATPART5: &str = ", 94\"/>
  <path d=\"M17 28
      a 7.9155 7.9155 0 0 1 0 -22
      a 7.9155 7.9155 0 0 1 0 22\" fill=\"none\" stroke=\""; //innerstrokebg

const SVGSTATPART6: &str = "\" stroke-width=\"3.7\"/>
  <path d=\"M17 28
      a 7.9155 7.9155 0 0 1 0 -22
      a 7.9155 7.9155 0 0 1 0 22\" fill=\"none\" stroke=\""; //innerstrokefg

const SVGSTATPART7: &str = "\" stroke-width=\"3.7\" stroke-dasharray=\""; //innerpercentage

const SVGSTATPART8: &str = ", 69\"/></svg>";

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
*/
