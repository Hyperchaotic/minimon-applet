use std::collections::VecDeque;

use sysinfo::Networks;

use crate::{
    colorpicker::DemoGraph, config::{ColorVariant, DeviceKind, GraphColors, GraphKind}, svg_graph::SvgColors
};

use super::Sensor;

const MAX_SAMPLES: usize = 30;
const GRAPH_SAMPLES: usize = 21;
const UNITS_SHORT: [&str; 5] = ["b", "K", "M", "G", "T"];
const UNITS_LONG: [&str; 5] = ["bps", "Kbps", "Mbps", "Gbps", "Tbps"];

const COLOR_CHOICES: [(&str, ColorVariant); 4] = [
    ("Down.  ", ColorVariant::Color2),
    ("Up.  ", ColorVariant::Color3),
    ("Back.  ", ColorVariant::Color1),
    ("Frame.", ColorVariant::Color4),
];

#[derive(Debug, PartialEq, Eq)]
pub enum UnitVariant {
    Short,
    Long,
}

#[derive(Debug)]
pub struct Network {
    networks: Networks,
    download: VecDeque<u64>,
    upload: VecDeque<u64>,
    max_y: Option<u64>,
    colors: GraphColors,
    svg_colors: SvgColors,
    kind: GraphKind,
}

impl DemoGraph for Network {
    fn demo(&self) -> String {
        let download = VecDeque::from(DL_DEMO);
        let upload = VecDeque::from(UL_DEMO);

        crate::svg_graph::double_line(&download, &upload, GRAPH_SAMPLES, &self.svg_colors, None)
    }

    fn colors(&self) -> GraphColors {
        self.colors
    }

    fn set_colors(&mut self, colors: GraphColors) {
        self.colors = colors;
        self.svg_colors.set_colors(&colors);
    }

    fn color_choices(&self) -> Vec<(&'static str, ColorVariant)> {
        COLOR_CHOICES.into()
    }
}

impl Sensor for Network {

    fn new(kind: GraphKind) -> Self {
        assert!(kind==GraphKind::Line);
        let networks = Networks::new_with_refreshed_list();
        let colors = GraphColors::new(DeviceKind::Network(GraphKind::Line));
        Network {
            networks,
            download: VecDeque::from(vec![0; MAX_SAMPLES]),
            upload: VecDeque::from(vec![0; MAX_SAMPLES]),
            max_y: None,
            colors,
            kind: GraphKind::Line,
            svg_colors: SvgColors::new(&colors),
        }
    }

    fn kind(&self) -> GraphKind {
        self.kind
    }

    fn set_kind(&mut self, kind: GraphKind) {
        assert!(kind==GraphKind::Line);
    }

    /// Retrieve the amount of data transmitted since last update.
    fn update(&mut self) {
        self.networks.refresh(true);
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

    fn demo_graph(&self, colors: GraphColors) -> Box<dyn DemoGraph> {
        let mut dmo = Network::new(self.kind);
        dmo.set_colors(colors);
        Box::new(dmo)
    }

    fn graph(&self) -> String {
        crate::svg_graph::double_line(
            &self.download,
            &self.upload,
            GRAPH_SAMPLES,
            &self.svg_colors,
            self.max_y,
        )
    }

}

impl Network {
    pub fn set_max_y(&mut self, max: Option<u64>) {
        self.max_y = max;
    }

    fn makestr(val: u64, format: UnitVariant) -> String {
        let mut value = val as f64;
        let mut unit_index = 0;
        let units = if format == UnitVariant::Short {
            UNITS_SHORT
        } else {
            UNITS_LONG
        };

        // Find the appropriate unit
        while value >= 999.0 && unit_index < units.len() - 1 {
            value /= 1024.0;
            unit_index += 1;
        }

        if value < 10.0 {
            format!("{:.2} {}", value, units[unit_index])
        } else if value < 99.0 {
            format!("{:.1} {}", value, units[unit_index])
        } else {
            format!("{:.0} {}", value, units[unit_index])
        }
    }

    // Get bits per second
    pub fn get_bitrate_dl(&self, ticks_per_sec: usize, format: UnitVariant) -> String {
        let len = self.download.len();
        let start = len.saturating_sub(ticks_per_sec);
        // Sum the last `ticks` elements
        let bps = self.download.iter().skip(start).sum();
        Network::makestr(bps, format)
    }

    // Get bits per second
    pub fn get_bitrate_ul(&self, ticks_per_sec: usize, format: UnitVariant) -> String {
        let len = self.upload.len();
        let start = len.saturating_sub(ticks_per_sec);
        // Sum the last `ticks` elements
        let bps = self.upload.iter().skip(start).sum();
        Network::makestr(bps, format)
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
