use std::collections::VecDeque;

use cosmic::{iced_widget::Column, Element};
use sysinfo::Networks;

use crate::{
    colorpicker::DemoGraph,
    config::{ColorVariant, DeviceKind, GraphColors, GraphKind, MinimonConfig, NetworkVariant},
    fl,
    svg_graph::SvgColors,
};

use cosmic::widget;
use cosmic::widget::settings;

use cosmic::{
    iced::{
        widget::{column, row},
        Alignment,
    },
    iced_widget::Row,
};

use crate::app::Message;

use lazy_static::lazy_static;

use super::Sensor;

const MAX_SAMPLES: usize = 30;
const GRAPH_SAMPLES: usize = 21;
const UNITS_SHORT: [&str; 5] = ["b", "K", "M", "G", "T"];
const UNITS_LONG: [&str; 5] = ["bps", "Kbps", "Mbps", "Gbps", "Tbps"];

lazy_static! {
    /// Translated color choices.
    ///
    /// The string values are intentionally leaked (`.leak()`) to convert them
    /// into `'static str` because:
    /// - These strings are only initialized once at program startup.
    /// - They are never deallocated since they are used globally.
    static ref COLOR_CHOICES_COMBINED: [(&'static str, ColorVariant); 4] = [
        (fl!("graph-network-download").leak(), ColorVariant::Color2),
        (fl!("graph-network-upload").leak(), ColorVariant::Color3),
        (fl!("graph-network-back").leak(), ColorVariant::Color1),
        (fl!("graph-network-frame").leak(), ColorVariant::Color4),
    ];
    static ref COLOR_CHOICES_DL: [(&'static str, ColorVariant); 3] = [
        (fl!("graph-network-download").leak(), ColorVariant::Color2),
        (fl!("graph-network-back").leak(), ColorVariant::Color1),
        (fl!("graph-network-frame").leak(), ColorVariant::Color4),
    ];
    static ref COLOR_CHOICES_UL: [(&'static str, ColorVariant); 3] = [
        (fl!("graph-network-upload").leak(), ColorVariant::Color3),
        (fl!("graph-network-back").leak(), ColorVariant::Color1),
        (fl!("graph-network-frame").leak(), ColorVariant::Color4),
    ];
}

macro_rules! network_select {
    ($self:ident, $variant:expr) => {
        match $variant {
            NetworkVariant::Combined | NetworkVariant::Download => &$self.network1,
            _ => &$self.network2,
        }
    };
}

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
    pub kind: NetworkVariant,
    dropdown_options: Vec<&'static str>,
}

impl DemoGraph for Network {
    fn demo(&self) -> String {
        let download = VecDeque::from(DL_DEMO);
        let upload = VecDeque::from(UL_DEMO);

        match self.kind {
            NetworkVariant::Combined => crate::svg_graph::double_line(
                &download,
                &upload,
                GRAPH_SAMPLES,
                &self.svg_colors,
                None,
            ),
            NetworkVariant::Download => {
                crate::svg_graph::line_adaptive(&download, GRAPH_SAMPLES, &self.svg_colors, None)
            }
            NetworkVariant::Upload => {
                let mut cols = self.svg_colors.clone();
                cols.color2 = cols.color3.clone();
                crate::svg_graph::line_adaptive(&upload, GRAPH_SAMPLES, &cols, None)
            }
        }
    }

    fn colors(&self) -> GraphColors {
        self.colors
    }

    fn set_colors(&mut self, colors: GraphColors) {
        self.colors = colors;
        self.svg_colors.set_colors(&colors);
    }

    fn color_choices(&self) -> Vec<(&'static str, ColorVariant)> {
        match self.kind {
            NetworkVariant::Combined => (*COLOR_CHOICES_COMBINED).into(),
            NetworkVariant::Download => (*COLOR_CHOICES_DL).into(),
            NetworkVariant::Upload => (*COLOR_CHOICES_UL).into(),
        }
    }
}

impl Sensor for Network {
    fn graph_kind(&self) -> GraphKind {
        GraphKind::Line
    }

    fn set_graph_kind(&mut self, kind: GraphKind) {
        assert!(kind == GraphKind::Line);
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
        match self.kind {
            NetworkVariant::Combined => crate::svg_graph::double_line(
                &self.download,
                &self.upload,
                GRAPH_SAMPLES,
                &self.svg_colors,
                self.max_y,
            ),
            NetworkVariant::Download => crate::svg_graph::line_adaptive(
                &self.download,
                GRAPH_SAMPLES,
                &self.svg_colors,
                self.max_y,
            ),
            NetworkVariant::Upload => {
                let mut cols = self.svg_colors.clone();
                cols.color2 = cols.color3.clone();
                crate::svg_graph::line_adaptive(&self.upload, GRAPH_SAMPLES, &cols, self.max_y)
            }
        }
    }

    fn settings_ui(&self, mmconfig: &MinimonConfig) -> Element<crate::app::Message> {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();
        let mut net_elements = Vec::new();

        let sample_rate_ms = mmconfig.refresh_rate;

        let dlrate = format!(
            "↓ {}",
            &self.download_label(sample_rate_ms, UnitVariant::Long)
        );

        let ulrate = format!(
            "↑ {}",
            &self.upload_label(sample_rate_ms, UnitVariant::Long)
        );

        let config = network_select!(mmconfig, self.kind);
        let k = self.kind;

        let mut rate = column!(Element::from(
            widget::svg(widget::svg::Handle::from_memory(
                self.graph().as_bytes().to_owned(),
            ))
            .width(90)
            .height(60)
        ));

        rate = rate.push(Element::from(cosmic::widget::text::body("")));

        match self.kind {
            NetworkVariant::Combined => {
                rate = rate.push(cosmic::widget::text::body(dlrate));
                rate = rate.push(cosmic::widget::text::body(ulrate));
            }
            NetworkVariant::Download => {
                rate = rate.push(cosmic::widget::text::body(dlrate));
            }
            NetworkVariant::Upload => {
                rate = rate.push(cosmic::widget::text::body(ulrate));
            }
        };
        net_elements.push(Element::from(rate));

        let mut net_bandwidth_items = Vec::new();

        let title = match self.kind {
            NetworkVariant::Combined => fl!("net-title-combined"),
            NetworkVariant::Download => fl!("net-title-dl"),
            NetworkVariant::Upload => fl!("net-title-ul"),
        };

        net_bandwidth_items.push(Element::from(widget::text::title4(title)));

        net_bandwidth_items.push(
            settings::item(
                fl!("enable-net-chart"),
                widget::toggler(config.chart).on_toggle(move |t| Message::ToggleNetChart(k, t)),
            )
            .into(),
        );
        net_bandwidth_items.push(
            settings::item(
                fl!("enable-net-label"),
                widget::toggler(config.label).on_toggle(move |t| Message::ToggleNetLabel(k, t)),
            )
            .into(),
        );
        net_bandwidth_items.push(
            settings::item(
                fl!("use-adaptive"),
                row!(widget::checkbox("", config.adaptive)
                    .on_toggle(move |t| Message::ToggleAdaptiveNet(k, t))),
            )
            .into(),
        );

        if !config.adaptive {
            net_bandwidth_items.push(
                settings::item(
                    fl!("net-bandwidth"),
                    row!(
                        widget::text_input("", config.bandwidth.to_string())
                            .width(100)
                            .on_input(move |b| Message::TextInputBandwidthChanged(k, b)),
                        widget::dropdown(&self.dropdown_options, config.unit, move |u| {
                            Message::NetworkSelectUnit(k, u)
                        },)
                        .width(50)
                    ),
                )
                .into(),
            );
        }

        net_bandwidth_items.push(
            row!(
                widget::horizontal_space(),
                widget::button::standard(fl!("change-colors"))
                    .on_press(Message::ColorPickerOpen(DeviceKind::Network(self.kind))),
                widget::horizontal_space()
            )
            .into(),
        );

        let net_right_column = Column::with_children(net_bandwidth_items);

        net_elements.push(Element::from(net_right_column.spacing(cosmic.space_xs())));

        Row::with_children(net_elements)
            .align_y(Alignment::Center)
            .spacing(0)
            .into()
    }
}

impl Network {
    pub fn new(kind: NetworkVariant) -> Self {
        let networks = Networks::new_with_refreshed_list();
        let colors = GraphColors::new(DeviceKind::Network(kind));
        Network {
            networks,
            download: VecDeque::from(vec![0; MAX_SAMPLES]),
            upload: VecDeque::from(vec![0; MAX_SAMPLES]),
            max_y: None,
            colors,
            kind,
            dropdown_options: ["b", "Kb", "Mb", "Gb", "Tb"].into(),
            svg_colors: SvgColors::new(&colors),
        }
    }

    pub fn set_max_y(&mut self, max: Option<u64>) {
        self.max_y = max;
    }

    fn makestr(val: u64, format: UnitVariant) -> String {
        let mut formatted = String::with_capacity(20);

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

        let s = if value < 10.0 {
            &format!("{:.2}", value)
        } else if value < 99.0 {
            &format!("{:.1}", value)
        } else {
            &format!("{:.0}", value)
        };

        formatted.push_str(s);
        if format == UnitVariant::Long {
            formatted.push(' ');
            if s.len() == 3 {
                formatted.push(' ');
            }
        }

        if unit_index == 0 && format == UnitVariant::Long {
            formatted.push(' ');
        }
        formatted.push_str(units[unit_index]);

        if formatted.len() < 9 && format == UnitVariant::Long {
            formatted.insert(0, ' ');
        }

        formatted
    }

    // If the sample rate doesn't match exactly one second (more or less),
    // we grab enough samples to cover it and average the value of samples cover a longer duration.
    fn last_second_bitrate(samples: &VecDeque<u64>, sample_interval_ms: u64) -> u64 {
        let mut total_duration = 0u64;
        let mut total_bitrate = 0u64;

        // Iterate from newest to oldest
        for &bitrate in samples.iter().rev() {
            if total_duration >= 1000 {
                break;
            }

            total_bitrate += bitrate;
            total_duration += sample_interval_ms;
        }

        // Scale to exactly 1000ms
        let scale = 1000.0 / total_duration as f64;

        (total_bitrate as f64 * scale).floor() as u64
    }

    // Get bits per second
    pub fn download_label(&self, sample_interval_ms: u64, format: UnitVariant) -> String {
        let rate = Network::last_second_bitrate(&self.download, sample_interval_ms);
        Network::makestr(rate, format)
    }

    // Get bits per second
    pub fn upload_label(&self, sample_interval_ms: u64, format: UnitVariant) -> String {
        let rate = Network::last_second_bitrate(&self.upload, sample_interval_ms);
        Network::makestr(rate, format)
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
