use crate::{
    colorpicker::DemoGraph,
    config::{ColorVariant, DeviceKind, GraphColors, GraphKind, MinimonConfig},
    fl,
    svg_graph::SvgColors,
};
use cosmic::Element;

use cosmic::widget;
use cosmic::widget::{settings, toggler};

use cosmic::{
    iced::{
        Alignment,
        widget::{column, row},
    },
    iced_widget::Row,
};
use log::info;

use crate::app::Message;

use lazy_static::lazy_static;
use std::{
    collections::VecDeque,
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

use super::{Sensor, TempUnit};

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

const GRAPH_OPTIONS: [&str; 2] = ["Ring", "Line"];
const UNIT_OPTIONS: [&str; 4] = ["Celcius", "Farenheit", "Kelvin", "Rankine"];

#[derive(Debug)]
pub struct CpuTemp {
    hwmon_path: Option<PathBuf>,
    samples: VecDeque<f64>,
    max_val: f32,
    colors: GraphColors,
    kind: GraphKind,
    graph_options: Vec<&'static str>,
    unit_options: Vec<&'static str>,
    /// colors cached so we don't need to convert to string every time
    svg_colors: SvgColors,
    tempunit: TempUnit,
}

impl DemoGraph for CpuTemp {
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
            GraphKind::Line => {
                crate::svg_graph::line(&VecDeque::from(DEMO_SAMPLES), 100, &self.svg_colors)
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
        if self.kind == GraphKind::Line {
            (*COLOR_CHOICES_LINE).into()
        } else {
            (*COLOR_CHOICES_RING).into()
        }
    }

    fn id(&self) -> Option<String> {
        None
    }
}

impl Sensor for CpuTemp {
    fn graph_kind(&self) -> GraphKind {
        self.kind
    }

    fn set_graph_kind(&mut self, kind: GraphKind) {
        assert!(kind == GraphKind::Line || kind == GraphKind::Ring);
        self.kind = kind;
    }

    fn update(&mut self) {
        if let Some(temp) = self.read_temp() {
            if self.samples.len() >= MAX_SAMPLES {
                self.samples.pop_front();
            }
            self.samples.push_back(temp as f64);
        }
    }

    fn demo_graph(&self, colors: GraphColors) -> Box<dyn DemoGraph> {
        let mut dmo = CpuTemp::new(self.kind);
        dmo.set_colors(colors);
        Box::new(dmo)
    }

    fn graph(&self) -> String {
        if self.kind == GraphKind::Ring {
            let latest = self.latest_sample();
            let mut value = self.to_string();
            let _ = value.pop(); // remove the C/F/K unit
            let mut percentage = String::with_capacity(10);

            write!(percentage, "{latest}").unwrap();

            crate::svg_graph::ring(&value, &percentage, &self.svg_colors)
        } else {
            crate::svg_graph::line(&self.samples, self.max_val as u64, &self.svg_colors)
        }
    }

    fn settings_ui(&self, config: &MinimonConfig) -> Element<crate::app::Message> {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();

        let mut temp_elements = Vec::new();

        let temp = self.to_string();
        temp_elements.push(Element::from(
            column!(
                widget::svg(widget::svg::Handle::from_memory(
                    self.graph().as_bytes().to_owned(),
                ))
                .width(90)
                .height(60),
                cosmic::widget::text::body(temp),
            )
            .padding(5)
            .align_x(Alignment::Center),
        ));

        let selected_graph: Option<usize> = Some(self.graph_kind().into());
        let selected_unit: Option<usize> = Some(self.tempunit.into());

        let temp_kind = self.graph_kind();
        temp_elements.push(Element::from(
            column!(
                settings::item(
                    fl!("enable-cpu-temperature-chart"),
                    toggler(config.cputemp.chart)
                        .on_toggle(|value| { Message::ToggleCpuTempChart(value) }),
                ),
                settings::item(
                    fl!("enable-cpu-temperature-label"),
                    toggler(config.cputemp.label)
                        .on_toggle(|value| { Message::ToggleCpuTempLabel(value) }),
                ),
                settings::item(
                    fl!("cpu-temperature-unit"),
                    widget::dropdown(&self.unit_options, selected_unit, |m| {
                        Message::SelectCpuTempUnit(m.into())
                    },)
                ),
                row!(
                    widget::dropdown(&self.graph_options, selected_graph, |m| {
                        Message::SelectGraphType(DeviceKind::CpuTemp, m.into())
                    },)
                    .width(70),
                    widget::horizontal_space(),
                    widget::button::standard(fl!("change-colors")).on_press(
                        Message::ColorPickerOpen(DeviceKind::CpuTemp, temp_kind, None)
                    ),
                )
            )
            .spacing(cosmic.space_xs()),
        ));

        Row::with_children(temp_elements)
            .align_y(Alignment::Center)
            .spacing(0)
            .into()
    }
}

impl CpuTemp {
    pub fn new(kind: GraphKind) -> Self {
        let mut max_val = 100.0;
        let mut hwmon_path = None;

        match Self::detect() {
            Ok((path, max)) => {
                hwmon_path = path;
                if let Some(max) = max {
                    max_val = max;
                }
            }
            Err(e) => info!("CpuTemp:detect: No CPU Temp IF found. {:?}", e),
        }

        let mut cpu = CpuTemp {
            hwmon_path,
            samples: VecDeque::from(vec![0.0; MAX_SAMPLES]),
            max_val,
            colors: GraphColors::default(),
            kind,
            graph_options: GRAPH_OPTIONS.to_vec(),
            svg_colors: SvgColors::new(&GraphColors::default()),
            tempunit: TempUnit::Celcius,
            unit_options: UNIT_OPTIONS.to_vec(),
        };
        cpu.set_colors(GraphColors::default());
        cpu
    }

    pub fn detect() -> std::io::Result<(Option<PathBuf>, Option<f32>)> {
        let hwmon_base = Path::new("/sys/class/hwmon");

        for entry in fs::read_dir(hwmon_base)? {
            let entry = entry?;
            let path = entry.path();
            let name_path = path.join("name");

            if let Ok(sensor_name) = fs::read_to_string(&name_path) {
                info!("Found: {:?} named {}",name_path, sensor_name);
                let sensor_name = sensor_name.trim();
                if sensor_name.contains("coretemp")
                    || sensor_name.contains("zenpower")
                    || sensor_name.contains("k10temp")
                    || sensor_name.contains("cpu")
                {
                    // Try to find a critical or max temp once
                    let mut critical_temp = None;
                    for i in 1..=99 {
                        for suffix in &["crit", "max"] {
                            let input_path = path.join(format!("temp{}_{}", i, "input"));
                            let crit_path = path.join(format!("temp{}_{}", i, suffix));
                            if crit_path.exists() && input_path.exists() {
                                info!("Found data: {:?}, {:?}", input_path, crit_path);
                                if let Ok(raw) = fs::read_to_string(&crit_path) {
                                    if let Ok(millidegrees) = raw.trim().parse::<f32>() {
                                        critical_temp = Some(millidegrees / 1000.0);
                                        break;
                                    }
                                }
                            }
                        }
                        if critical_temp.is_some() {
                            break;
                        }
                    }

                    info!("CpuTemp::detect: CPUTemp IF found in {:?}", path);
                    return Ok((Some(path), critical_temp));
                }
            }
        }

        Ok((None, None))
    }

    /// Read current max CPU temperature across all cores
    pub fn read_temp(&self) -> Option<f32> {
        let mut max_temp = f32::MIN;

        if let Some(path) = &self.hwmon_path {
            for i in 1..=99 {
                let temp_path = path.join(format!("temp{}_input", i));
                if temp_path.exists() {
                    if let Ok(raw) = fs::read_to_string(&temp_path) {
                        if let Ok(millidegrees) = raw.trim().parse::<f32>() {
                            let degrees = millidegrees / 1000.0;
                            if degrees > max_temp {
                                max_temp = degrees;
                            }
                        }
                    }
                }
            }
        }
        if max_temp > f32::MIN {
            Some(max_temp)
        } else {
            None
        }
    }

    // true if a CPU temperature hwmon path was found
    pub fn is_found(&self) -> bool {
        self.hwmon_path.is_some()
    }

    pub fn set_unit(&mut self, unit: TempUnit) {
        self.tempunit = unit;

    }

    pub fn latest_sample(&self) -> f64 {
        *self.samples.back().unwrap_or(&0f64)
    }
}

use std::fmt;

impl fmt::Display for CpuTemp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let current_val = self.latest_sample();
        match self.tempunit {
            TempUnit::Celcius => write!(f, "{}C", current_val.trunc()),
            TempUnit::Farenheit => write!(f, "{}F", (current_val * 9.0 / 5.0 + 32.0).trunc()),
            TempUnit::Kelvin => write!(f, "{}K", (current_val + 273.15).trunc()),
            TempUnit::Rankine => write!(f, "{}R", (current_val * 9.0 / 5.0 + 491.67).trunc()),            
        }
    }
}

const DEMO_SAMPLES: [f64; 21] = [
    41.0, 42.0, 43.5, 45.0, 48.0, 51.0, 55.0, 57.0, 59.5, 62.0, 64.0, 67.0, 70.0, 74.0, 78.0, 83.0,
    87.0, 90.0, 95.0, 98.0, 100.0,
];
