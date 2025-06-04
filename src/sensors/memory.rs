use cosmic::Element;
use sysinfo::{MemoryRefreshKind, System};

use crate::{
    colorpicker::DemoGraph,
    config::{ColorVariant, DeviceKind, GraphColors, GraphKind, MemoryConfig},
    fl,
    svg_graph::SvgColors,
};

use cosmic::widget;
use cosmic::widget::{settings, toggler};
use std::any::Any;

use cosmic::{
    iced::{
        Alignment,
        widget::{column, row},
    },
    iced_widget::Row,
};

use crate::app::Message;

use std::{collections::VecDeque, fmt::Write};

use super::Sensor;

const GRAPH_OPTIONS: [&str; 2] = ["Ring", "Line"];

const MAX_SAMPLES: usize = 21;

#[derive(Debug)]
pub struct Memory {
    samples: VecDeque<f64>,
    max_val: f64,
    system: System,
    graph_options: Vec<&'static str>,
    /// colors cached so we don't need to convert to string every time
    svg_colors: SvgColors,
    config: MemoryConfig,
}

impl DemoGraph for Memory {
    fn demo(&self) -> String {
        match self.config.kind {
            GraphKind::Ring => {
                // show a number of 40% of max
                let val = self.max_val * 0.4;
                let percentage: u64 = ((val / self.max_val) * 100.0) as u64;
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
            GraphKind::Heat => panic!("Wrong graph choice!"),
        }
    }

    fn colors(&self) -> GraphColors {
        self.config.colors
    }

    fn set_colors(&mut self, colors: GraphColors) {
        self.config.colors = colors;
        self.svg_colors.set_colors(&colors);
    }

    fn color_choices(&self) -> Vec<(&'static str, ColorVariant)> {
        if self.config.kind == GraphKind::Line {
            (*super::COLOR_CHOICES_LINE).into()
        } else {
            (*super::COLOR_CHOICES_RING).into()
        }
    }

    fn id(&self) -> Option<String> {
        None
    }
}

impl Sensor for Memory {
    fn update_config(&mut self, config: &dyn Any, _refresh_rate: u32) {
        if let Some(cfg) = config.downcast_ref::<MemoryConfig>() {
            self.config = cfg.clone();
            self.svg_colors.set_colors(&cfg.colors);
        }
    }

    fn graph_kind(&self) -> GraphKind {
        self.config.kind
    }

    fn set_graph_kind(&mut self, kind: GraphKind) {
        assert!(kind == GraphKind::Line || kind == GraphKind::Ring);
        self.config.kind = kind;
    }

    fn update(&mut self) {
        let r = MemoryRefreshKind::nothing().with_ram();

        self.system.refresh_memory_specifics(r);
        let new_val: f64 = self.system.used_memory() as f64 / 1_073_741_824.0;

        if self.samples.len() >= MAX_SAMPLES {
            self.samples.pop_front();
        }
        self.samples.push_back(new_val);
    }

    fn demo_graph(&self) -> Box<dyn DemoGraph> {
        let mut dmo = Memory::default();
        dmo.update_config(&self.config, 0);
        Box::new(dmo)
    }

    fn graph(&self) -> String {
        if self.config.kind == GraphKind::Ring {
            let mut latest = self.latest_sample();
            let mut value = String::with_capacity(10);
            let mut percentage = String::with_capacity(10);

            let mut pct: u64 = ((latest / self.max_val) * 100.0) as u64;
            if pct > 100 {
                pct = 100;
            }

            write!(percentage, "{pct}").unwrap();

            // If set, convert to percentage
            if self.config.percentage {
                latest = (latest * 100.0) / self.max_val as f64;
            }

            if latest < 10.0 {
                write!(value, "{latest:.2}").unwrap();
            } else if latest <= 99.9 {
                write!(value, "{latest:.1}").unwrap();
            } else {
                write!(value, "100").unwrap();
            }

            crate::svg_graph::ring(&value, &percentage, &self.svg_colors)
        } else {
            crate::svg_graph::line(&self.samples, self.max_val, &self.svg_colors)
        }
    }

    fn settings_ui(&self) -> Element<crate::app::Message> {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();

        let mut mem_elements = Vec::new();
        let mem = self.to_string(false);
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

        let config = &self.config;
        let selected: Option<usize> = Some(self.graph_kind().into());
        let mem_kind = self.graph_kind();
        mem_elements.push(Element::from(
            column!(
                settings::item(
                    fl!("enable-chart"),
                    toggler(config.chart).on_toggle(|value| { Message::ToggleMemoryChart(value) }),
                ),
                settings::item(
                    fl!("enable-label"),
                    toggler(config.label).on_toggle(|value| { Message::ToggleMemoryLabel(value) }),
                ),
                settings::item(
                    fl!("memory-as-percentage"),
                    toggler(config.percentage).on_toggle(Message::ToggleMemoryPercentage),
                ),
                row!(
                    widget::dropdown(&self.graph_options, selected, move |m| {
                        Message::SelectGraphType(DeviceKind::Memory, m.into())
                    },)
                    .width(70),
                    widget::horizontal_space(),
                    widget::button::standard(fl!("change-colors"))
                        .on_press(Message::ColorPickerOpen(DeviceKind::Memory, mem_kind, None)),
                )
            )
            .spacing(cosmic.space_xs()),
        ));

        Row::with_children(mem_elements)
            .align_y(Alignment::Center)
            .spacing(0)
            .into()
    }
}

impl Default for Memory {
    fn default() -> Self {
        let mut system = System::new();
        system.refresh_memory();

        let max_val: f64 = system.total_memory() as f64 / 1_073_741_824.0;
        log::info!(
            "System memory: {} / {:.2} GB",
            system.total_memory(),
            max_val
        );

        // value and percentage are pre-allocated and reused as they're changed often.
        let mut percentage = String::with_capacity(6);
        write!(percentage, "0").unwrap();

        let mut value = String::with_capacity(6);
        write!(value, "0").unwrap();

        let mut memory = Memory {
            samples: VecDeque::from(vec![0.0; MAX_SAMPLES]),
            max_val,
            system,
            config: MemoryConfig::default(),
            graph_options: GRAPH_OPTIONS.to_vec(),
            svg_colors: SvgColors::new(&GraphColors::default()),
        };
        memory.set_colors(GraphColors::default());
        memory
    }
}

impl Memory {
    pub fn set_percentage(&mut self, percentage: bool) {
        self.config.percentage = percentage;
    }

    pub fn latest_sample(&self) -> f64 {
        *self.samples.back().unwrap_or(&0f64)
    }

    pub fn total(&self) -> f64 {
        self.max_val
    }

    pub fn to_string(&self, vertical_panel: bool) -> String {
        let mut current_val = self.latest_sample();
        let unit: &str;

        if self.config.percentage {
            current_val = (current_val * 100.0) / self.max_val;
            unit = "%";
        } else if !vertical_panel {
            unit = " GB";
        } else {
            unit = "GB";
        }

        if current_val < 10.0 {
            format!("{current_val:.2}{unit}")
        } else if current_val < 100.0 {
            format!("{current_val:.1}{unit}")
        } else {
            format!("{current_val}{unit}")
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
