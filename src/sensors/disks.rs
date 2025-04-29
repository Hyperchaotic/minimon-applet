use std::collections::VecDeque;

use sysinfo::{DiskRefreshKind, Disks as DisksInfo};

use crate::{
    colorpicker::DemoGraph,
    config::{ColorVariant, DeviceKind, GraphColors, GraphKind, MinimonConfig},
    fl,
    svg_graph::SvgColors,
};

use cosmic::{widget::Column, Element};

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
use crate::config::DisksVariant;

use lazy_static::lazy_static;

use super::Sensor;

const MAX_SAMPLES: usize = 30;
const GRAPH_SAMPLES: usize = 21;
const UNITS_SHORT: [&str; 5] = ["B", "K", "M", "G", "T"];
const UNITS_LONG: [&str; 5] = ["B/s", "KB/s", "MB/s", "GB/s", "TB/s"];

lazy_static! {
    /// Translated color choices.
    ///
    /// The string values are intentionally leaked (`.leak()`) to convert them
    /// into `'static str` because:
    /// - These strings are only initialized once at program startup.
    /// - They are never deallocated since they are used globally.
    static ref COLOR_CHOICES_COMBINED: [(&'static str, ColorVariant); 4] = [
        (fl!("graph-disks-write").leak(), ColorVariant::Color2),
        (fl!("graph-disks-read").leak(), ColorVariant::Color3),
        (fl!("graph-disks-back").leak(), ColorVariant::Color1),
        (fl!("graph-disks-frame").leak(), ColorVariant::Color4),
    ];
    static ref COLOR_CHOICES_WRITE: [(&'static str, ColorVariant); 3] = [
        (fl!("graph-disks-write").leak(), ColorVariant::Color2),
        (fl!("graph-disks-back").leak(), ColorVariant::Color1),
        (fl!("graph-disks-frame").leak(), ColorVariant::Color4),
    ];
    static ref COLOR_CHOICES_READ: [(&'static str, ColorVariant); 3] = [
        (fl!("graph-disks-read").leak(), ColorVariant::Color3),
        (fl!("graph-disks-back").leak(), ColorVariant::Color1),
        (fl!("graph-disks-frame").leak(), ColorVariant::Color4),
    ];
}

macro_rules! disks_select {
    ($self:ident, $variant:expr) => {
        match $variant {
            DisksVariant::Combined | DisksVariant::Write => &$self.disks1,
            _ => &$self.disks2,
        }
    };
}

#[derive(Debug, PartialEq, Eq)]
pub enum UnitVariant {
    Short,
    Long,
}

#[derive(Debug)]
pub struct Disks {
    disks: DisksInfo,
    write: VecDeque<u64>,
    read: VecDeque<u64>,
    max_y: Option<u64>,
    colors: GraphColors,
    svg_colors: SvgColors,
    pub kind: DisksVariant,
}

impl DemoGraph for Disks {
    fn demo(&self) -> String {
        let write = VecDeque::from(DL_DEMO);
        let read = VecDeque::from(UL_DEMO);

        match self.kind {
            DisksVariant::Combined => {
                crate::svg_graph::double_line(&write, &read, GRAPH_SAMPLES, &self.svg_colors, None)
            }
            DisksVariant::Write => {
                crate::svg_graph::line_adaptive(&write, GRAPH_SAMPLES, &self.svg_colors, None)
            }
            DisksVariant::Read => {
                let mut cols = self.svg_colors.clone();
                cols.color2 = cols.color3.clone();
                crate::svg_graph::line_adaptive(&read, GRAPH_SAMPLES, &cols, None)
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
            DisksVariant::Combined => (*COLOR_CHOICES_COMBINED).into(),
            DisksVariant::Write => (*COLOR_CHOICES_WRITE).into(),
            DisksVariant::Read => (*COLOR_CHOICES_READ).into(),
        }
    }

    fn id(&self) -> Option<String> {
        None
    }

}

impl Sensor for Disks {
    fn graph_kind(&self) -> GraphKind {
        GraphKind::Line
    }

    fn set_graph_kind(&mut self, kind: GraphKind) {
        assert!(kind == GraphKind::Line);
    }

    /// Retrieve the amount of data transmitted since last update.
    fn update(&mut self) {
        let r = DiskRefreshKind::nothing().with_io_usage();
        self.disks.refresh_specifics(true, r);
        let mut wr = 0;
        let mut rd = 0;

        for disk in self.disks.list() {
            let usage = disk.usage();
            wr += usage.written_bytes;
            rd += usage.read_bytes;
        }

        if self.write.len() >= MAX_SAMPLES {
            self.write.pop_front();
        }
        self.write.push_back(wr);

        if self.read.len() >= MAX_SAMPLES {
            self.read.pop_front();
        }
        self.read.push_back(rd);
    }

    fn demo_graph(&self, colors: GraphColors) -> Box<dyn DemoGraph> {
        let mut dmo = Disks::new(self.kind);
        dmo.set_colors(colors);
        Box::new(dmo)
    }

    fn graph(&self) -> String {
        match self.kind {
            DisksVariant::Combined => crate::svg_graph::double_line(
                &self.write,
                &self.read,
                GRAPH_SAMPLES,
                &self.svg_colors,
                self.max_y,
            ),
            DisksVariant::Write => crate::svg_graph::line_adaptive(
                &self.write,
                GRAPH_SAMPLES,
                &self.svg_colors,
                self.max_y,
            ),
            DisksVariant::Read => {
                let mut cols = self.svg_colors.clone();
                cols.color2 = cols.color3.clone();
                crate::svg_graph::line_adaptive(&self.read, GRAPH_SAMPLES, &cols, self.max_y)
            }
        }
    }

    fn settings_ui(&self, mmconfig: &MinimonConfig) -> Element<crate::app::Message> {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();
        let mut disk_elements = Vec::new();

        let sample_rate_ms = mmconfig.refresh_rate;

        let wrrate = format!("W {}", &self.write_label(sample_rate_ms, UnitVariant::Long));

        let rdrate = format!("R {}", &self.read_label(sample_rate_ms, UnitVariant::Long));

        let config = disks_select!(mmconfig, self.kind);
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
            DisksVariant::Combined => {
                rate = rate.push(cosmic::widget::text::body(wrrate));
                rate = rate.push(cosmic::widget::text::body(rdrate));
            }
            DisksVariant::Write => {
                rate = rate.push(cosmic::widget::text::body(wrrate));
            }
            DisksVariant::Read => {
                rate = rate.push(cosmic::widget::text::body(rdrate));
            }
        };
        disk_elements.push(Element::from(rate));

        let mut disk_bandwidth_items = Vec::new();

        let title = match self.kind {
            DisksVariant::Combined => fl!("disks-title-combined"),
            DisksVariant::Write => fl!("disks-title-write"),
            DisksVariant::Read => fl!("disks-title-read"),
        };

        disk_bandwidth_items.push(Element::from(widget::text::title4(title)));

        disk_bandwidth_items.push(
            settings::item(
                fl!("enable-disks-chart"),
                widget::toggler(config.chart).on_toggle(move |t| Message::ToggleDisksChart(k, t)),
            )
            .into(),
        );
        disk_bandwidth_items.push(
            settings::item(
                fl!("enable-disks-label"),
                widget::toggler(config.label).on_toggle(move |t| Message::ToggleDisksLabel(k, t)),
            )
            .into(),
        );

        disk_bandwidth_items.push(
            row!(
                widget::horizontal_space(),
                widget::button::standard(fl!("change-colors"))
                    .on_press(Message::ColorPickerOpen(DeviceKind::Disks(self.kind), None)),
                widget::horizontal_space()
            )
            .into(),
        );

        let disk_right_column = Column::with_children(disk_bandwidth_items);

        disk_elements.push(Element::from(disk_right_column.spacing(cosmic.space_xs())));

        Row::with_children(disk_elements)
            .align_y(Alignment::Center)
            .spacing(0)
            .into()
    }
}

impl Disks {
    pub fn new(kind: DisksVariant) -> Self {
        let disks = DisksInfo::new_with_refreshed_list();
        let colors = GraphColors::new(DeviceKind::Disks(kind));
        Disks {
            disks,
            write: VecDeque::from(vec![0; MAX_SAMPLES]),
            read: VecDeque::from(vec![0; MAX_SAMPLES]),
            max_y: None,
            colors,
            kind,
            svg_colors: SvgColors::new(&colors),
        }
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

        if format == UnitVariant::Long {
            if s.len() == 3 {
                formatted.push(' ');
            }
            if unit_index == 0 {
                formatted.push(' ');
            }
        }
        formatted.push_str(s);

        if format == UnitVariant::Long {
            formatted.push(' ');
        }

        formatted.push_str(units[unit_index]);

        if format == UnitVariant::Long {
            let padding = 9usize.saturating_sub(formatted.len());
            if padding > 0 {
                formatted = " ".repeat(padding) + &formatted;
            }
        }

        formatted
    }

    // If the sample rate doesn't match exactly one second (more or less),
    // we grab enough samples to cover it and average the value of samples cover a longer duration.
    fn last_second_rate(samples: &VecDeque<u64>, sample_interval_ms: u64) -> u64 {
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

    // Get bytes per second
    pub fn write_label(&self, sample_interval_ms: u64, format: UnitVariant) -> String {
        let val = Disks::last_second_rate(&self.write, sample_interval_ms);
        Disks::makestr(val, format)
    }

    // Get bytes per second
    pub fn read_label(&self, sample_interval_ms: u64, format: UnitVariant) -> String {
        let val = Disks::last_second_rate(&self.read, sample_interval_ms);
        Disks::makestr(val, format)
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
