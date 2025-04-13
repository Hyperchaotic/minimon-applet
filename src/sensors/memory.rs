use cosmic::Element;
use sysinfo::System;

use crate::{
    colorpicker::DemoGraph,
    config::{ColorVariant, DeviceKind, GraphColors, GraphKind, MinimonConfig},
    fl,
    svg_graph::SvgColors,
};

use cosmic::widget::{settings, toggler};
use cosmic::widget;

use cosmic::{
    iced::{
        widget::{column, row},
        Alignment,
    },
    iced_widget::Row,
};


use crate::app::Message;

use lazy_static::lazy_static;
use std::{collections::VecDeque, fmt::Write};

use super::Sensor;

const GRAPH_OPTIONS: [&'static str; 2] = ["Ring", "Line"];

const MAX_SAMPLES: usize = 21;

lazy_static! {
    /// Translated color choices.
    ///
    /// The string values are intentionally leaked (`.leak()`) to convert them
    /// into `'static str` because:
    /// - These strings are only initialized once at program startup.
    /// - They are never deallocated since they are used globally.
    static ref COLOR_CHOICES_RING: [(&'static str, ColorVariant); 4] = [
        (fl!("graph-ring-r1").leak(), ColorVariant::Color4),
        (fl!("graph-ring-r2").leak(), ColorVariant::Color3),
        (fl!("graph-ring-back").leak(), ColorVariant::Color1),
        (fl!("graph-ring-text").leak(), ColorVariant::Color2),
    ];
}

lazy_static! {
    /// Translated color choices.
    ///
    /// The string values are intentionally leaked (`.leak()`) to convert them
    /// into `'static str` because:
    /// - These strings are only initialized once at program startup.
    /// - They are never deallocated since they are used globally.
    static ref COLOR_CHOICES_LINE: [(&'static str, ColorVariant); 3] = [
        (fl!("graph-line-graph").leak(), ColorVariant::Color4),
        (fl!("graph-line-back").leak(), ColorVariant::Color1),
        (fl!("graph-line-frame").leak(), ColorVariant::Color2),
    ];
}

#[derive(Debug)]
pub struct Memory {
    samples: VecDeque<f64>,
    max_val: u64,
    colors: GraphColors,
    system: System,
    kind: GraphKind,
    graph_options: Vec<&'static str>,

    /// current value ram load shown.
    value: String,
    /// the percentage of the ring to be filled
    percentage: String,

    /// colors cached so we don't need to convert to string every time
    svg_colors: SvgColors,
}

impl DemoGraph for Memory {
    fn demo(&self) -> String {
        match self.kind {
            GraphKind::Ring => {
                // show a number of 40% of max
                let val = self.max_val as f64 * 0.4;
                let percentage: u64 = ((val / self.max_val as f64) * 100.0) as u64;
                crate::svg_graph::ring(
                    &format!("{val}"),
                    &format!("{percentage}"),
                    &self.svg_colors,
                )
            }
            GraphKind::Line => crate::svg_graph::line(
                &VecDeque::from(DEMO_SAMPLES),
                self.max_val,
                &self.svg_colors,
            ),
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
        if self.kind == GraphKind::Line {
            (*COLOR_CHOICES_LINE).into()
        } else {
            (*COLOR_CHOICES_RING).into()
        }
    }
}

impl Sensor for Memory {
    fn graph_kind(&self) -> GraphKind {
        self.kind
    }

    fn set_graph_kind(&mut self, kind: GraphKind) {
        assert!(kind == GraphKind::Line || kind == GraphKind::Ring);
        self.kind = kind;
    }

    fn update(&mut self) {
        self.system.refresh_memory();
        let new_val: f64 = self.system.used_memory() as f64 / 1_073_741_824.0;

        if self.samples.len() >= MAX_SAMPLES {
            self.samples.pop_front();
        }
        self.samples.push_back(new_val);

        if self.kind == GraphKind::Ring {
            self.value.clear();
            let current_val = self.latest_sample();
            if current_val < 10.0 {
                write!(self.value, "{:.2}", current_val).unwrap();
            } else if current_val < 100.0 {
                write!(self.value, "{:.1}", current_val).unwrap();
            } else {
                write!(self.value, "{}", current_val).unwrap();
            }

            let percentage: u64 = ((current_val / self.max_val as f64) * 100.0) as u64;
            self.percentage.clear();
            write!(self.percentage, "{percentage}").unwrap();
        }
    }

    fn demo_graph(&self, colors: GraphColors) -> Box<dyn DemoGraph> {
        let mut dmo = Memory::new(self.kind);
        dmo.set_colors(colors);
        Box::new(dmo)
    }

    fn graph(&self) -> String {
        if self.kind == GraphKind::Ring {
            crate::svg_graph::ring(&self.value, &self.percentage, &self.svg_colors)
        } else {
            crate::svg_graph::line(&self.samples, self.max_val, &self.svg_colors)
        }
    }

    fn settings_ui(&self, config: &MinimonConfig) -> Element<crate::app::Message> {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();

        let mut mem_elements = Vec::new();
        let mem = self.to_string();
        mem_elements.push(Element::from(
            column!(
                widget::svg(widget::svg::Handle::from_memory(
                    self.graph().as_bytes().to_owned(),
                ))
                .width(90)
                .height(60),
                cosmic::widget::text::body(mem),
            )
            .padding(5)
            .align_x(Alignment::Center),
        ));

        let selected: Option<usize> = Some(self.graph_kind().into());

        let mem_kind = self.graph_kind();
        mem_elements.push(Element::from(
            column!(
                widget::text::title4(fl!("memory-title")),
                settings::item(
                    fl!("enable-memory-chart"),
                    toggler(config.memory.chart)
                        .on_toggle(|value| { Message::ToggleMemoryChart(value) }),
                ),
                settings::item(
                    fl!("enable-memory-label"),
                    toggler(config.memory.label)
                        .on_toggle(|value| { Message::ToggleMemoryLabel(value) }),
                ),
                row!(
                    widget::dropdown(&self.graph_options, selected, move |m| {
                        Message::SelectGraphType(DeviceKind::Memory(m.into()))
                    },)
                    .width(70),
                    widget::horizontal_space(),
                    widget::button::standard(fl!("change-colors"))
                        .on_press(Message::ColorPickerOpen(DeviceKind::Memory(mem_kind))),
                )
            )
            .spacing(cosmic.space_xs()),
        ));

        Row::with_children(mem_elements)
            .align_y(Alignment::Center)
            .spacing(0).into()
    }
}

impl Memory {
    pub fn new(kind: GraphKind) -> Self {
        let mut system = System::new();
        system.refresh_memory();

        let max_val = system.total_memory() / 1_073_741_824;

        // value and percentage are pre-allocated and reused as they're changed often.
        let mut percentage = String::with_capacity(6);
        write!(percentage, "0").unwrap();

        let mut value = String::with_capacity(6);
        write!(value, "0").unwrap();

        let mut memory = Memory {
            samples: VecDeque::from(vec![0.0; MAX_SAMPLES]),
            max_val,
            colors: GraphColors::default(),
            system,
            kind,
            graph_options: GRAPH_OPTIONS.to_vec(),
            value,
            percentage,
            svg_colors: SvgColors::new(&GraphColors::default()),
        };
        memory.set_colors(GraphColors::default());
        memory
    }

    pub fn latest_sample(&self) -> f64 {
        *self.samples.back().unwrap_or(&0f64)
    }

    pub fn to_string(&self) -> String {
        let current_val = self.latest_sample();
        let unit = " GB";

        if current_val < 10.0 {
            format!("{:.2}{}", current_val, unit)
        } else if current_val < 100.0 {
            format!("{:.1}{}", current_val, unit)
        } else {
            format!("{}{}", current_val, unit)
        }
    }
}

const DEMO_SAMPLES: [f64; 21] = [
    0.0,
    12.689857482910156,
    12.642768859863281,
    12.615306854248047,
    12.658184051513672,
    12.65273666381836,
    12.626102447509766,
    12.624862670898438,
    12.613967895507813,
    12.619949340820313,
    19.061111450195313,
    21.691085815429688,
    21.810935974121094,
    21.28915786743164,
    22.041973114013672,
    21.764171600341797,
    21.89263916015625,
    15.258216857910156,
    14.770732879638672,
    14.496528625488281,
    13.892818450927734,
];
