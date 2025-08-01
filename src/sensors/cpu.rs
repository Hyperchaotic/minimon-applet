use crate::{
    barchart::StackedBarSvg,
    colorpicker::DemoGraph,
    config::{ColorVariant, CpuConfig, DeviceKind, GraphColors, GraphKind},
    fl,
    svg_graph::SvgColors,
};
use cosmic::{
    Element, Renderer, Theme, iced::Alignment::Center, iced_widget::Column, widget::Container,
};
use std::{any::Any, sync::LazyLock};

use cosmic::widget;
use cosmic::widget::{settings, toggler};

use cosmic::{
    iced::{
        Alignment,
        widget::{column, row},
    },
    iced_widget::Row,
};

use crate::app::Message;

use std::{
    collections::{HashMap, VecDeque},
    fmt::Write,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use super::Sensor;

const MAX_SAMPLES: usize = 21;

static GRAPH_OPTIONS_RING_LINE_BARS: LazyLock<[&'static str; 3]> = LazyLock::new(|| {
    [
        fl!("graph-type-ring").leak(),
        fl!("graph-type-line").leak(),
        fl!("graph-type-bars").leak(),
    ]
});

pub static COLOR_CHOICES_BARS: LazyLock<[(&'static str, ColorVariant); 4]> = LazyLock::new(|| {
    [
        (fl!("graph-bars-system").leak(), ColorVariant::Color4),
        (fl!("graph-bars-user").leak(), ColorVariant::Color3),
        (fl!("graph-line-back").leak(), ColorVariant::Color1),
        (fl!("graph-line-frame").leak(), ColorVariant::Color2),
    ]
});

#[derive(Debug)]
struct CpuTimes {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct CpuLoad {
    pub user_pct: f64,
    pub system_pct: f64,
}

#[derive(Debug)]
pub struct Cpu {
    // Total CPU load since last update split into user and system
    total_cpu_load: CpuLoad,
    // Load per core since last update split into user and system
    core_loads: HashMap<usize, CpuLoad>,
    // Load per core since last update split into values read from /proc
    prev_core_times: HashMap<usize, CpuTimes>,
    // Total CPU load for the last MAX_SAMPLES updates
    samples_sum: VecDeque<f64>,
    // CPU load for the last MAX_SAMPLES updates, split into user and system
    samples_split: VecDeque<CpuLoad>,
    graph_options: Vec<&'static str>,
    /// colors cached so we don't need to convert to string every time
    svg_colors: SvgColors,
    bar_svg_colors: SvgColors,
    config: CpuConfig,
}

impl DemoGraph for Cpu {
    fn demo(&self) -> String {
        match self.config.kind {
            GraphKind::Ring => {
                // show a number of 40% of max
                let val = 40;
                let percentage: u64 = 40;
                crate::svg_graph::ring(
                    &format!("{val}"),
                    &format!("{percentage}"),
                    &self.svg_colors,
                )
            }
            GraphKind::Line => {
                crate::svg_graph::line(&VecDeque::from(DEMO_SAMPLES), 100.0, &self.svg_colors)
            }
            GraphKind::Heat => panic!("Wrong graph choice!"),
            GraphKind::StackedBars => {
                let mut map = HashMap::new();
                map.insert(
                    0,
                    CpuLoad {
                        user_pct: 15.5,
                        system_pct: 8.2,
                    },
                );
                map.insert(
                    1,
                    CpuLoad {
                        user_pct: 42.1,
                        system_pct: 12.7,
                    },
                );
                map.insert(
                    2,
                    CpuLoad {
                        user_pct: 78.9,
                        system_pct: 18.3,
                    },
                );
                map.insert(
                    3,
                    CpuLoad {
                        user_pct: 25.6,
                        system_pct: 5.4,
                    },
                );
                StackedBarSvg::default().svg(&map, &self.bar_svg_colors)
            }
        }
    }

    fn colors(&self) -> GraphColors {
        if self.config.kind == GraphKind::StackedBars {
            self.config.bar_colors
        } else {
            self.config.colors
        }
    }

    fn set_colors(&mut self, colors: GraphColors) {
        if self.config.kind == GraphKind::StackedBars {
            self.config.bar_colors = colors;
            self.bar_svg_colors.set_colors(&colors);
        } else {
            self.config.colors = colors;
            self.svg_colors.set_colors(&colors);
        }
    }

    fn color_choices(&self) -> Vec<(&'static str, ColorVariant)> {
        match self.config.kind {
            GraphKind::Line => (*super::COLOR_CHOICES_LINE).into(),
            GraphKind::Ring => (*super::COLOR_CHOICES_RING).into(),
            GraphKind::StackedBars => (*COLOR_CHOICES_BARS).into(),
            _ => panic!("CPU color_choices {:?} wrong chart type!", self.config.kind),
        }
    }

    fn id(&self) -> Option<String> {
        None
    }
}

impl Sensor for Cpu {
    fn update_config(&mut self, config: &dyn Any, _refresh_rate: u32) {
        if let Some(cfg) = config.downcast_ref::<CpuConfig>() {
            self.config = cfg.clone();
            self.svg_colors.set_colors(&cfg.colors);
            self.bar_svg_colors.set_colors(&cfg.bar_colors);
        }
    }

    fn graph_kind(&self) -> GraphKind {
        self.config.kind
    }

    fn set_graph_kind(&mut self, kind: GraphKind) {
        assert!(
            kind == GraphKind::Line || kind == GraphKind::Ring || kind == GraphKind::StackedBars
        );
        self.config.kind = kind;
    }

    fn update(&mut self) {
        self.update_stats();

        if self.samples_split.len() >= MAX_SAMPLES {
            self.samples_split.pop_front();
        }
        self.samples_split.push_back(self.total_cpu_load);

        let new_sum = self.total_cpu_load.user_pct + self.total_cpu_load.system_pct;
        if self.samples_sum.len() >= MAX_SAMPLES {
            self.samples_sum.pop_front();
        }
        self.samples_sum.push_back(new_sum);
    }

    fn demo_graph(&self) -> Box<dyn DemoGraph> {
        let mut dmo = Cpu::new(true);
        dmo.update_config(&self.config, 0);
        Box::new(dmo)
    }

    #[cfg(feature = "lyon_charts")]
    fn chart<'a>(&self) -> cosmic::widget::Container<crate::app::Message, Theme, Renderer> {
        if self.config.kind == GraphKind::Ring {
            let latest = self.latest_sample();
            let mut value = String::with_capacity(10);

            if self.config.no_decimals {
                write!(value, "{}%", latest.round()).unwrap();
            } else if latest < 10.0 {
                write!(value, "{latest:.2}").unwrap()
            } else if latest <= 99.9 {
                write!(value, "{latest:.1}").unwrap();
            } else {
                write!(value, "100").unwrap();
            }
            chart_container!(crate::charts::ring::RingChart::new(
                latest as f32,
                &value,
                &self.config.colors,
            ))
        } else {
            chart_container!(crate::charts::line::LineChart::new(
                MAX_SAMPLES,
                &self.samples_sum,
                &VecDeque::new(),
                Some(100.0),
                &self.config.colors,
            ))
        }
    }

    #[cfg(not(feature = "lyon_charts"))]
    fn chart(
        &self,
        height_hint: u16,
        _width_hint: u16,
    ) -> cosmic::widget::Container<crate::app::Message, Theme, Renderer> {
        let svg = match self.config.kind {
            GraphKind::Ring => {
                let latest = self.latest_sample();
                let mut value = String::with_capacity(10);
                let mut percentage = String::with_capacity(10);

                if self.config.no_decimals {
                    write!(value, "{}%", latest.round()).unwrap();
                } else if latest < 10.0 {
                    write!(value, "{latest:.2}").unwrap()
                } else if latest <= 99.9 {
                    write!(value, "{latest:.1}").unwrap();
                } else {
                    write!(value, "100").unwrap();
                }

                write!(percentage, "{latest}").unwrap();

                crate::svg_graph::ring(&value, &percentage, &self.svg_colors)
            }
            GraphKind::Line => crate::svg_graph::line(&self.samples_sum, 100.0, &self.svg_colors),
            GraphKind::StackedBars => {
                StackedBarSvg::new(self.config.bar_width, height_hint, self.config.bar_spacing)
                    .svg(&self.core_loads, &self.bar_svg_colors)
            }
            GraphKind::Heat => panic!("Heat not supported!"),
        };

        let icon = cosmic::widget::icon::from_svg_bytes(svg.into_bytes());
        Container::new(
            icon.icon()
                .height(cosmic::iced::Length::Fill)
                .width(cosmic::iced::Length::Fill),
        )
    }

    fn settings_ui(&self) -> Element<crate::app::Message> {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();

        let mut cpu_elements = Vec::new();
        let mut cpu_column = Vec::new();

        if self.graph_kind() != GraphKind::StackedBars {
            let cpu = self.to_string();
            cpu_elements.push(Element::from(
                column!(
                    Container::new(self.chart(60, 60).width(60).height(60))
                        .width(90)
                        .align_x(Alignment::Center),
                    cosmic::widget::text::body(cpu.to_string())
                        .width(90)
                        .align_x(Alignment::Center)
                )
                .padding(5)
                .align_x(Alignment::Center),
            ));
        } else {
            let width = StackedBarSvg::new(self.config.bar_width, 60, self.config.bar_spacing)
                .width(self.core_count());
            cpu_column.push(Element::from(row!(
                widget::horizontal_space(),
                self.chart(60, width).height(60).width(width),
                widget::horizontal_space()
            )));
        };

        // A bit ugly and error prone, the Heat type is not supported here so bars takes its place
        // in numbering for the dropdown
        let selected: Option<usize> = if self.graph_kind() == GraphKind::StackedBars {
            Some(2)
        } else {
            Some(self.graph_kind().into())
        };

        let config = &self.config;
        let cpu_kind = self.graph_kind();

        cpu_column.push(
            settings::item(
                fl!("enable-chart"),
                toggler(config.chart).on_toggle(Message::ToggleCpuChart),
            )
            .into(),
        );

        if self.graph_kind() == GraphKind::StackedBars {
            cpu_column.push(
                settings::item(
                    fl!("graph-bar-width"),
                    widget::spin_button(
                        self.config.bar_width.to_string(),
                        self.config.bar_width,
                        1,
                        1,
                        16,
                        Message::CpuBarSizeChanged,
                    ),
                )
                .into(),
            );

            let narrow = config.bar_spacing == 0;
            cpu_column.push(
                settings::item(
                    fl!("graph-bar-spacing"),
                    toggler(narrow).on_toggle(Message::CpuNarrowBarSpacing),
                )
                .into(),
            );
        }

        cpu_column.push(
            settings::item(
                fl!("enable-label"),
                toggler(config.label).on_toggle(Message::ToggleCpuLabel),
            )
            .into(),
        );
        if self.config.label {
            cpu_column.push(
                settings::item(
                    fl!("cpu-no-decimals"),
                    row!(
                        widget::checkbox("", config.no_decimals)
                            .on_toggle(Message::ToggleCpuNoDecimals)
                    ),
                )
                .into(),
            );
        }
        cpu_column.push(
            row!(
                widget::text::body(fl!("chart-type")),
                widget::dropdown(&self.graph_options, selected, move |m| {
                    let mut choice: GraphKind = m.into();
                    if choice != GraphKind::Ring && choice != GraphKind::Line {
                        choice = GraphKind::StackedBars
                    };
                    Message::SelectGraphType(DeviceKind::Cpu, choice)
                })
                .width(70),
                widget::horizontal_space(),
                widget::button::standard(fl!("change-colors")).on_press(Message::ColorPickerOpen(
                    DeviceKind::Cpu,
                    cpu_kind,
                    None
                )),
            )
            .align_y(Center)
            .into(),
        );

        cpu_elements.push(Element::from(
            Column::with_children(cpu_column).spacing(cosmic.space_xs()),
        ));

        Row::with_children(cpu_elements)
            .align_y(Alignment::Center)
            .spacing(0)
            .into()
    }
}

impl Cpu {
    pub fn new(is_horizontal: bool) -> Self {
        // value and percentage are pre-allocated and reused as they're changed often.
        let mut percentage = String::with_capacity(6);
        write!(percentage, "0").unwrap();

        let mut value = String::with_capacity(6);
        write!(value, "0").unwrap();

        let graph_opts: Vec<&'static str> = if is_horizontal {
            (*GRAPH_OPTIONS_RING_LINE_BARS).into()
        } else {
            (*super::GRAPH_OPTIONS_RING_LINE).into()
        };

        let mut cpu = Cpu {
            total_cpu_load: CpuLoad {
                user_pct: 0.,
                system_pct: 0.,
            },
            core_loads: HashMap::new(),
            prev_core_times: Cpu::read_cpu_stats(),
            samples_sum: VecDeque::from(vec![0.0; MAX_SAMPLES]),
            samples_split: VecDeque::from(vec![
                CpuLoad {
                    user_pct: 0.,
                    system_pct: 0.
                };
                MAX_SAMPLES
            ]),
            graph_options: graph_opts.to_vec(),
            svg_colors: SvgColors::new(&GraphColors::default()),
            bar_svg_colors: SvgColors::new(&GraphColors::default()),
            config: CpuConfig::default(),
        };
        cpu.set_colors(GraphColors::default());
        cpu
    }

    pub fn latest_sample(&self) -> f64 {
        *self.samples_sum.back().unwrap_or(&0f64)
    }

    pub fn core_count(&self) -> usize {
        self.core_loads.len()
    }

    /* Simulated Threadripper
    fn read_cpu_stats() -> HashMap<usize, CpuTimes> {
        let mut combined_stats = HashMap::new();

        for batch in 0..5 {
            let batch_stats = Self::read_cpu_stats2();

            // Offset the core IDs by batch * 20 to avoid conflicts
            for (core_id, cpu_times) in batch_stats {
                let new_core_id = core_id + (batch * 20);
                combined_stats.insert(new_core_id, cpu_times);
            }
        }

        combined_stats
    }
    // Read CPU statistics from /proc/stat
    fn read_cpu_stats2() -> HashMap<usize, CpuTimes> {*/
    fn read_cpu_stats() -> HashMap<usize, CpuTimes> {
        let mut cpu_stats = HashMap::new();

        // Open /proc/stat file
        let Ok(file) = File::open(Path::new("/proc/stat")) else {
            return cpu_stats;
        };

        let reader = BufReader::new(file);

        // Read each line from the file
        for line in reader.lines() {
            let Ok(line) = line else { continue };
            // Split line into parts
            let parts: Vec<&str> = line.split_whitespace().collect();

            // Check if line starts with 'cpu' followed by a number
            if parts.is_empty() || !parts[0].starts_with("cpu") || parts[0] == "cpu" {
                continue;
            }

            // Extract CPU number
            let Ok(cpu_num) = parts[0].trim_start_matches("cpu").parse::<usize>() else {
                continue;
            };

            // Ensure we have enough parts for all fields
            if parts.len() < 9 {
                continue;
            }

            // Parse all CPU time values
            let user = parts[1].parse::<u64>().unwrap_or(0);
            let nice = parts[2].parse::<u64>().unwrap_or(0);
            let system = parts[3].parse::<u64>().unwrap_or(0);
            let idle = parts[4].parse::<u64>().unwrap_or(0);
            let iowait = parts[5].parse::<u64>().unwrap_or(0);
            let irq = parts[6].parse::<u64>().unwrap_or(0);
            let softirq = parts[7].parse::<u64>().unwrap_or(0);
            let steal = parts[8].parse::<u64>().unwrap_or(0);

            // Create CpuTimes struct and insert into HashMap
            let cpu_times = CpuTimes {
                user,
                nice,
                system,
                idle,
                iowait,
                irq,
                softirq,
                steal,
            };

            cpu_stats.insert(cpu_num, cpu_times);
        }

        cpu_stats
    }

    // Update current CPU load by comparing to previous samples
    fn update_stats(&mut self) {
        // Read current CPU stats
        let current_cpu_times = Cpu::read_cpu_stats();

        // Temporary storage for new per-core loads
        let mut new_cpu_loads = HashMap::with_capacity(current_cpu_times.len());

        // Running totals for average computation
        let mut total_user_pct = 0.0;
        let mut total_system_pct = 0.0;
        let mut counted_cores = 0;

        for (&cpu_num, current) in &current_cpu_times {
            if let Some(prev) = self.prev_core_times.get(&cpu_num) {
                // Compute time deltas
                let user = current.user.saturating_sub(prev.user);
                let nice = current.nice.saturating_sub(prev.nice);
                let system = current.system.saturating_sub(prev.system);
                let idle = current.idle.saturating_sub(prev.idle);
                let iowait = current.iowait.saturating_sub(prev.iowait);
                let irq = current.irq.saturating_sub(prev.irq);
                let softirq = current.softirq.saturating_sub(prev.softirq);
                let steal = current.steal.saturating_sub(prev.steal);

                let total = user + nice + system + idle + iowait + irq + softirq + steal;
                if total == 0 {
                    continue;
                }

                let total_f64 = total as f64;
                let user_pct = (user + nice) as f64 / total_f64 * 100.0;
                let system_pct = system as f64 / total_f64 * 100.0;

                new_cpu_loads.insert(
                    cpu_num,
                    CpuLoad {
                        user_pct,
                        system_pct,
                    },
                );

                total_user_pct += user_pct;
                total_system_pct += system_pct;
                counted_cores += 1;
            }
        }

        self.core_loads = new_cpu_loads;

        if counted_cores > 0 {
            let core_count_f64 = f64::from(counted_cores);
            self.total_cpu_load = CpuLoad {
                user_pct: total_user_pct / core_count_f64,
                system_pct: total_system_pct / core_count_f64,
            };
        }

        self.prev_core_times = current_cpu_times;
    }
}

use std::fmt;

impl fmt::Display for Cpu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let current_val = self.latest_sample();

        if self.config.no_decimals {
            write!(f, "{}%", current_val.round())
        } else if current_val < 10.0 {
            write!(f, "{current_val:.2}%")
        } else if current_val < 100.0 {
            write!(f, "{current_val:.1}%")
        } else {
            write!(f, "{current_val}%")
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
