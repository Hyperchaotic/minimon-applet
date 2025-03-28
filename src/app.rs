use cosmic::applet::cosmic_panel_config::PanelSize;
use cosmic::applet::{PanelType, Size};
use cosmic::cosmic_config::CosmicConfigEntry;
use cosmic::cosmic_theme::palette::{FromColor, WithAlpha};
use std::time;

use cosmic::app::{Core, Task};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::window::Id;
use cosmic::iced::Limits;
use cosmic::iced::{self, Subscription};
use cosmic::widget::{settings, spin_button, toggler};
use cosmic::Element;
use cosmic::{widget, widget::autosize};

use once_cell::sync::Lazy;
use std::sync::atomic::{self, AtomicI64};
use std::sync::Arc;

use cosmic::{
    applet::cosmic_panel_config::PanelAnchor,
    iced::{
        widget::{column, row},
        Alignment,
    },
    iced_widget::{Column, Row},
    widget::container,
};

use crate::colorpicker::{ColorPicker, DemoGraph};
use crate::config::{ColorVariant, DeviceKind, GraphColors, GraphKind};
use crate::sensors::cpu::Cpu;
use crate::sensors::memory::Memory;
use crate::sensors::network::{Network, UnitVariant};
use crate::sensors::Sensor;
use crate::{config::MinimonConfig, fl};
use cosmic::widget::Id as WId;

static AUTOSIZE_MAIN_ID: Lazy<WId> = Lazy::new(|| WId::new("autosize-main"));

const TICK: i64 = 250;

const ICON: &str = "com.github.hyperchaotic.cosmic-applet-minimon";

pub struct Minimon {
    /// Application state which is managed by the COSMIC runtime.
    core: Core,
    /// The svg image to draw for the CPU load
    cpu: Cpu,
    /// The svg image to draw for the Memory load
    memory: Memory,
    /// The popup id.
    popup: Option<Id>,
    /// The color picker dialog
    colorpicker: ColorPicker,
    dropdown_options: Vec<&'static str>,
    graph_options: Vec<&'static str>,

    /// The network monitor
    network: Network,
    /// Settings stored on disk, including refresh rate, colors, etc.
    config: MinimonConfig,
    /// Countdown timer, as the subscription tick is 250ms
    /// this counter can be set higher and controls refresh/update rate.
    /// Refreshes machine stats when reaching 0 and is reset to configured rate.
    tick_timer: i64,
    /// tick can be 250, 500 or 1000, depending on refresh rate modolu tick
    tick: Arc<AtomicI64>,
    /// System Monitor Application
    sysmon: Option<(String, String)>,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,

    ColorPickerOpen(DeviceKind),
    ColorPickerClose(bool),
    ColorPickerDefaults,
    ColorPickerAccent,

    ColorPickerSliderRedChanged(u8),
    ColorPickerSliderGreenChanged(u8),
    ColorPickerSliderBlueChanged(u8),
    ColorPickerSliderAlphaChanged(u8),
    ColorPickerSelectVariant(ColorVariant),

    ColorTextInputRedChanged(String),
    ColorTextInputGreenChanged(String),
    ColorTextInputBlueChanged(String),
    ColorTextInputAlphaChanged(String),

    ToggleAdaptiveNet(bool),
    NetworkSelectUnit(usize),
    TextInputBandwidthChanged(String),

    SelectGraphType(DeviceKind),
    Tick,
    PopupClosed(Id),

    ToggleNetChart(bool),
    ToggleNetLabel(bool),
    ToggleCpuChart(bool),
    ToggleCpuLabel(bool),
    ToggleMemoryChart(bool),
    ToggleMemoryLabel(bool),
    ConfigChanged(MinimonConfig),
    LaunchSystemMonitor(),
    RefreshRateChanged(f64),
    LabelSizeChanged(u16),
}

const APP_ID_DOCK: &str = "com.github.hyperchaotic.cosmic-applet-minimon-dock";
const APP_ID_PANEL: &str = "com.github.hyperchaotic.cosmic-applet-minimon-panel";
const APP_ID_OTHER: &str = "com.github.hyperchaotic.cosmic-applet-minimon-other";

impl cosmic::Application for Minimon {
    type Executor = cosmic::executor::Default;

    type Flags = ();

    type Message = Message;

    const APP_ID: &'static str = "com.github.hyperchaotic.cosmic-applet-minimon";

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let app = Minimon {
            core,
            cpu: Cpu::new(GraphKind::Ring),
            memory: Memory::new(GraphKind::Line),
            popup: None,
            colorpicker: ColorPicker::new(),
            dropdown_options: ["b", "Kb", "Mb", "Gb", "Tb"].into(),
            graph_options: ["Ring", "Line"].into(),
            network: Network::new(GraphKind::Line),
            config: MinimonConfig::default(),
            tick_timer: TICK,
            tick: Arc::new(AtomicI64::new(TICK)),
            sysmon: Minimon::get_sysmon(),
        };

        (app, Task::none())
    }

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }

    fn subscription(&self) -> Subscription<Message> {
        fn time_subscription(tick: std::sync::Arc<AtomicI64>) -> Subscription<time::Instant> {
            let atomic = tick.clone();
            let val = atomic.load(atomic::Ordering::Relaxed);
            iced::time::every(time::Duration::from_millis(val as u64))
        }

        Subscription::batch(vec![
            time_subscription(self.tick.clone()).map(|_| Message::Tick),
            self.core
                .watch_config(match self.core.applet.panel_type {
                    PanelType::Panel => APP_ID_PANEL,
                    PanelType::Dock => APP_ID_DOCK,
                    PanelType::Other(_) => APP_ID_OTHER,
                })
                .map(|u| Message::ConfigChanged(u.config)),
        ])
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }
    fn view(&self) -> Element<Message> {
        let horizontal = matches!(
            self.core.applet.anchor,
            PanelAnchor::Top | PanelAnchor::Bottom
        );

        let mut limits = Limits::NONE.min_width(1.).min_height(1.);

        if let Some(b) = self.core.applet.suggested_bounds {
            if b.width as i32 > 0 {
                limits = limits.max_width(b.width);
            }
            if b.height as i32 > 0 {
                limits = limits.max_height(b.height);
            }
        }

        if !self.config.enable_cpu_chart
            && !self.config.enable_cpu_label
            && !self.config.enable_mem_chart
            && !self.config.enable_mem_label
            && !self.config.enable_net_chart
            && !self.config.enable_net_label
        {
            return self
                .core
                .applet
                .icon_button(ICON)
                .on_press(Message::TogglePopup)
                .into();
        }

        // If we are below 10% and horizontal layout we can show another decimal
        let formated_cpu = if self.cpu.latest_sample() < 10.0 && horizontal {
            format!("{:.2}%", self.cpu.latest_sample())
        } else {
            format!("{:.1}%", self.cpu.latest_sample())
        };

        let formated_mem = format!("{:.1} GB", self.memory.latest_sample());

        // vertical layout
        let mut elements: Vec<Element<Message>> = Vec::new();

        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();

        if self.config.enable_cpu_label {
            elements.push(self.figure_label(formated_cpu).into());
        }

        if self.config.enable_cpu_chart {
            let content = self
                .core
                .applet
                .icon_button_from_handle(Minimon::make_icon_handle(&self.cpu));

            elements.push(content.into());
        }

        if self.config.enable_mem_label {
            elements.push(self.figure_label(formated_mem).into());
        }

        if self.config.enable_mem_chart {
            let content = self
                .core
                .applet
                .icon_button_from_handle(Minimon::make_icon_handle(&self.memory));

            elements.push(content.into());
        }

        // Network

        if self.config.enable_net_label {
            let ticks_per_sec = (1000 / self.tick.clone().load(atomic::Ordering::Relaxed)) as usize;

            let mut network_labels: Vec<Element<Message>> = Vec::new();

            // DL
            let dl_label = match horizontal {
                true => self.figure_label(format!(
                    "↓ {}",
                    &self.network.get_bitrate_dl(ticks_per_sec, UnitVariant::Long)
                )),
                false => self.figure_label(
                    self.network
                        .get_bitrate_dl(ticks_per_sec, UnitVariant::Short),
                ),
            };
            network_labels.push(widget::vertical_space().into());
            network_labels.push(dl_label.into());
            // UL
            let ul_label = match horizontal {
                true => self.figure_label(format!(
                    "↑ {}",
                    &self.network.get_bitrate_ul(ticks_per_sec, UnitVariant::Long)
                )),
                false => self.figure_label(
                    self.network
                        .get_bitrate_ul(ticks_per_sec, UnitVariant::Short),
                ),
            };
            network_labels.push(ul_label.into());
            network_labels.push(widget::vertical_space().into());

            elements.push(Column::from_vec(network_labels).into());
        }

        if self.config.enable_net_chart {
            let svg = self.network.graph();
            let handle = cosmic::widget::icon::from_svg_bytes(svg.into_bytes());
            let content = self.core.applet.icon_button_from_handle(handle);

            elements.push(content.into());
        }

        let wrapper: Element<Message> = match horizontal {
            true => Row::from_vec(elements)
                .align_y(Alignment::Center)
                .spacing(cosmic.space_xxs())
                .into(),
            false => Column::from_vec(elements)
                .align_x(Alignment::Center)
                .spacing(cosmic.space_xxs())
                .into(),
        };

        let button = widget::button::custom(wrapper)
            .padding(if horizontal {
                [0, self.core.applet.suggested_padding(true)]
            } else {
                [self.core.applet.suggested_padding(true), 0]
            })
            .class(cosmic::theme::Button::AppletIcon)
            .on_press(Message::TogglePopup);

        autosize::autosize(container(button), AUTOSIZE_MAIN_ID.clone())
            .limits(limits)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<Self::Message> {
        if self.colorpicker.active() {
            let limits = Limits::NONE
            .max_width(400.0)
            .min_width(400.0)
            .min_height(200.0)
            .max_height(750.0);

            self.core
                .applet
                .popup_container(self.colorpicker.view_colorpicker()).limits(limits)
                .into()
        } else {
            let theme = cosmic::theme::active();
            let cosmic = theme.cosmic();

            let mut cpu_elements = Vec::new();

            let cpu = self.cpu.to_string();
            cpu_elements.push(Element::from(
                column!(
                    widget::svg(widget::svg::Handle::from_memory(
                        self.cpu.graph().as_bytes().to_owned(),
                    ))
                    .width(90)
                    .height(60),
                    cosmic::widget::text::body(cpu),
                )
                .padding(5)
                .align_x(Alignment::Center),
            ));

            let selected: Option<usize> = Some(self.cpu.kind().into());

            let cpu_kind = self.cpu.kind();
            cpu_elements.push(Element::from(
                column!(
                    widget::text::title4(fl!("cpu-title")),
                    settings::item(
                        fl!("enable-cpu-chart"),
                        toggler(self.config.enable_cpu_chart)
                            .on_toggle(|value| { Message::ToggleCpuChart(value) }),
                    ),
                    settings::item(
                        fl!("enable-cpu-label"),
                        toggler(self.config.enable_cpu_label)
                            .on_toggle(|value| { Message::ToggleCpuLabel(value) }),
                    ),
                    row!(
                        widget::dropdown(&self.graph_options, selected, move |m| {
                            Message::SelectGraphType(DeviceKind::Cpu(m.into()))
                        },)
                        .width(70),
                        widget::horizontal_space(),
                        widget::button::standard(fl!("change-colors"))
                            .on_press(Message::ColorPickerOpen(DeviceKind::Cpu(cpu_kind))),
                    )
                )
                .spacing(cosmic.space_xs()),
            ));

            let cpu_row = Row::with_children(cpu_elements)
                .align_y(Alignment::Center)
                .spacing(0);

            let mut mem_elements = Vec::new();
            let mem = self.memory.to_string();
            mem_elements.push(Element::from(
                column!(
                    widget::svg(widget::svg::Handle::from_memory(
                        self.memory.graph().as_bytes().to_owned(),
                    ))
                    .width(90)
                    .height(60),
                    cosmic::widget::text::body(mem),
                )
                .padding(5)
                .align_x(Alignment::Center),
            ));

            let selected: Option<usize> = Some(self.memory.kind().into());

            let mem_kind = self.memory.kind();
            mem_elements.push(Element::from(
                column!(
                    widget::text::title4(fl!("memory-title")),
                    settings::item(
                        fl!("enable-memory-chart"),
                        toggler(self.config.enable_mem_chart)
                            .on_toggle(|value| { Message::ToggleMemoryChart(value) }),
                    ),
                    settings::item(
                        fl!("enable-memory-label"),
                        toggler(self.config.enable_mem_label)
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

            let mem_row = Row::with_children(mem_elements)
                .align_y(Alignment::Center)
                .spacing(0);

            let mut refresh_elements = Vec::new();
            let refresh_rate = self.config.refresh_rate as f64 / 1000.0;
            refresh_elements.push(Element::from(spin_button(
                format!("{:.2}", refresh_rate),
                refresh_rate,
                0.250,
                0.250,
                5.00,
                Message::RefreshRateChanged,
            )));
            let refresh_row = Row::with_children(refresh_elements)
                .align_y(Alignment::Center)
                .spacing(0);

            let mut net_elements = Vec::new();

            let ticks_per_sec = (1000 / self.tick.clone().load(atomic::Ordering::Relaxed)) as usize;

            let dlrate = format!(
                "↓ {}",
                &self.network.get_bitrate_dl(ticks_per_sec, UnitVariant::Long)
            );
            let ulrate = format!(
                "↑ {}",
                &self.network.get_bitrate_ul(ticks_per_sec, UnitVariant::Long)
            );

            net_elements.push(Element::from(
                column!(
                    widget::svg(widget::svg::Handle::from_memory(
                        self.network.graph().as_bytes().to_owned(),
                    ))
                    .width(90)
                    .height(60),
                    cosmic::widget::text::body(""),
                    cosmic::widget::text::body(dlrate),
                    cosmic::widget::text::body(ulrate),
                )
                .padding(5)
                .align_x(Alignment::Center),
            ));

            let mut net_bandwidth_items = Vec::new();
            net_bandwidth_items.push(Element::from(widget::text::title4(fl!("net-title"))));
            net_bandwidth_items.push(
                settings::item(
                    fl!("enable-net-chart"),
                    widget::toggler(self.config.enable_net_chart)
                        .on_toggle(Message::ToggleNetChart),
                )
                .into(),
            );
            net_bandwidth_items.push(
                settings::item(
                    fl!("enable-net-label"),
                    widget::toggler(self.config.enable_net_label)
                        .on_toggle(Message::ToggleNetLabel),
                )
                .into(),
            );
            net_bandwidth_items.push(
                settings::item(
                    fl!("use-adaptive"),
                    row!(widget::checkbox("", self.config.enable_adaptive_net)
                        .on_toggle(|v| { Message::ToggleAdaptiveNet(v) }),),
                )
                .into(),
            );

            if !self.config.enable_adaptive_net {
                net_bandwidth_items.push(
                    settings::item(
                        fl!("net-bandwidth"),
                        row!(
                            widget::text_input("", self.config.net_bandwidth.to_string())
                                .width(100)
                                .on_input(Message::TextInputBandwidthChanged),
                            widget::dropdown(
                                &self.dropdown_options,
                                self.config.net_unit,
                                Message::NetworkSelectUnit,
                            )
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
                        .on_press(Message::ColorPickerOpen(DeviceKind::Network(self.network.kind()))),
                    widget::horizontal_space()
                )
                .into(),
            );

            let net_right_column = Column::with_children(net_bandwidth_items);

            net_elements.push(Element::from(net_right_column.spacing(cosmic.space_xs())));

            let net_row = Row::with_children(net_elements)
                .align_y(Alignment::Center)
                .spacing(0);

            let change_label_setting = settings::item(
                fl!("change-label-size"),
                spin_button(
                    self.config.label_size_default.to_string(),
                    self.config.label_size_default,
                    1,
                    5,
                    20,
                    Message::LabelSizeChanged,
                ),
            );

            let mut content_list = widget::list_column();
            if let Some((_exec, application)) = self.sysmon.as_ref() {
                content_list = content_list.add(row!(
                    widget::horizontal_space(),
                    widget::button::standard(application)
                        .on_press(Message::LaunchSystemMonitor())
                        .trailing_icon(widget::button::link::icon()),
                    widget::horizontal_space()
                ));
            }
            let content_list = content_list // = widget::list_column()
                .spacing(5)
                .add(Element::from(cpu_row))
                .add(Element::from(mem_row))
                .add(Element::from(net_row))
                .add(settings::item(
                    fl!("refresh-rate"),
                    Element::from(refresh_row),
                ))
                .add(change_label_setting);

                let limits = Limits::NONE
                .max_width(420.0)
                .min_width(360.0)
                .min_height(200.0)
                .max_height(750.0);

                self.core.applet.popup_container(content_list).limits(limits).into()
        }
    }

    /// Application messages are handled here. The application state can be modified based on
    /// what message was received. Commands may be returned for asynchronous execution on a
    /// background thread managed by the application's executor.
    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    self.colorpicker.deactivate();
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    popup_settings.positioner.size_limits = Limits::NONE; 

                    get_popup(popup_settings)
                };
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.colorpicker.deactivate();
                    self.popup = None;
                }
            }
            Message::ColorPickerOpen(kind) => {
                match kind {
                    DeviceKind::Cpu(_) => {
                        self.colorpicker
                        .activate(kind, self.cpu.demo_graph(self.config.cpu_colors));
                    }
                    DeviceKind::Memory(_) => {
                        self.colorpicker
                            .activate(kind, self.memory.demo_graph(self.config.mem_colors));
                    }
                    DeviceKind::Network(_) => {
                        self.colorpicker.activate(kind, self.network.demo_graph(self.config.net_colors));
                    }
                }
                self.colorpicker.set_variant(ColorVariant::Color1);
                let col = self
                    .colorpicker
                    .colors()
                    .get_color(self.colorpicker.variant());
                self.colorpicker.set_sliders(col);
            }

            Message::ColorPickerClose(save) => {
                if save {
                    self.set_colors(self.colorpicker.colors(), self.colorpicker.kind());
                    self.save_config();
                }
                self.colorpicker.deactivate();
            }

            Message::ColorPickerDefaults => {
                self.colorpicker.default_colors();
            }

            Message::ColorPickerAccent => {
                if let Some(theme) = self.core.applet.theme() {
                    let accent = theme.cosmic().accent_color().color;
                    let srgba = cosmic::cosmic_theme::palette::Srgba::from_color(accent);
                    self.colorpicker.set_sliders(srgba.opaque().into());
                }
            }

            Message::ColorPickerSliderRedChanged(val) => {
                let mut col = self.colorpicker.sliders();
                col.red = val;
                self.colorpicker.set_sliders(col);
            }

            Message::ColorPickerSliderGreenChanged(val) => {
                let mut col = self.colorpicker.sliders();
                col.green = val;
                self.colorpicker.set_sliders(col);
            }

            Message::ColorPickerSliderBlueChanged(val) => {
                let mut col = self.colorpicker.sliders();
                col.blue = val;
                self.colorpicker.set_sliders(col);
            }

            Message::ColorPickerSliderAlphaChanged(val) => {
                let mut col = self.colorpicker.sliders();
                col.alpha = val;
                self.colorpicker.set_sliders(col);
            }

            Message::ColorPickerSelectVariant(variant) => {
                self.colorpicker.set_variant(variant);
            }

            Message::ToggleAdaptiveNet(toggle) => {
                self.config.enable_adaptive_net = toggle;
                if toggle {
                    self.network.set_max_y(None);
                }
                self.save_config();
            }

            Message::NetworkSelectUnit(unit) => {
                if !self.config.enable_adaptive_net {
                    self.config.net_unit = Some(unit);
                    self.set_network_max_y();
                    self.save_config();
                }
            }

            Message::SelectGraphType(dev) => {
                match dev {
                    DeviceKind::Cpu(kind) => {
                        self.cpu.set_kind(kind);
                        self.config.cpu_type = kind;
                    }
                    DeviceKind::Memory(kind) => {
                        self.memory
                            .set_kind(kind);
                        self.config.mem_type = kind;
                    }
                    DeviceKind::Network(kind) => {
                        self.network.set_kind(kind);
                    }
                }
                self.save_config();
            }

            Message::TextInputBandwidthChanged(string) => {
                if string.is_empty() {
                    self.config.net_bandwidth = 0;
                    self.set_network_max_y();
                    self.save_config();
                } else if !self.config.enable_adaptive_net {
                    if let Ok(val) = string.parse::<u64>() {
                        self.config.net_bandwidth = val;
                        self.set_network_max_y();
                        self.save_config();
                    }
                }
            }

            Message::Tick => {
                let tick = self.tick.load(atomic::Ordering::Relaxed);

                if self.tick_timer <= 0 {
                    self.tick_timer = self.config.refresh_rate as i64;
                    self.refresh_stats();
                }

                if self.tick_timer >= tick {
                    self.tick_timer -= tick;
                } else {
                    self.tick_timer = 0;
                };
            }

            Message::ToggleCpuChart(toggled) => {
                self.config.enable_cpu_chart = toggled;
                self.save_config();
            }
            Message::ToggleMemoryChart(toggled) => {
                self.config.enable_mem_chart = toggled;
                self.save_config();
            }
            Message::ToggleNetChart(toggled) => {
                self.config.enable_net_chart = toggled;
                self.save_config();
            }

            Message::ToggleCpuLabel(toggled) => {
                self.config.enable_cpu_label = toggled;
                self.save_config();
            }

            Message::ToggleMemoryLabel(toggled) => {
                self.config.enable_mem_label = toggled;
                self.save_config();
            }

            Message::ToggleNetLabel(toggled) => {
                self.config.enable_net_label = toggled;
                self.save_config();
            }

            Message::ConfigChanged(config) => {
                self.config = config;
                self.tick_timer = self.config.refresh_rate as i64;
                self.cpu.set_colors(self.config.cpu_colors);
                self.cpu.set_kind(self.config.cpu_type);
                self.memory.set_colors(self.config.mem_colors);
                self.memory.set_kind(self.config.mem_type);
                self.network.set_colors(self.config.net_colors);
                self.set_network_max_y();
                self.set_tick();
            }

            Message::ColorTextInputRedChanged(value) => {
                let mut col = self.colorpicker.sliders();
                Minimon::set_color(&value, &mut col.red);
                self.colorpicker.set_sliders(col);
            }

            Message::ColorTextInputGreenChanged(value) => {
                let mut col = self.colorpicker.sliders();
                Minimon::set_color(&value, &mut col.green);
                self.colorpicker.set_sliders(col);
            }

            Message::ColorTextInputBlueChanged(value) => {
                let mut col = self.colorpicker.sliders();
                Minimon::set_color(&value, &mut col.blue);
                self.colorpicker.set_sliders(col);
            }

            Message::ColorTextInputAlphaChanged(value) => {
                let mut col = self.colorpicker.sliders();
                Minimon::set_color(&value, &mut col.alpha);
                self.colorpicker.set_sliders(col);
            }

            Message::LaunchSystemMonitor() => {
                self.spawn_sysmon();
            }

            Message::RefreshRateChanged(rate) => {
                self.config.refresh_rate = (rate * 1000.0) as u64;
                self.set_tick();
                self.save_config();
            }

            Message::LabelSizeChanged(size) => {
                self.config.label_size_default = size;
                self.save_config();
            }
        }
        Task::none()
    }
}

impl Minimon {
    fn make_icon_handle<T: Sensor>(sensor: &T) -> cosmic::widget::icon::Handle {
        cosmic::widget::icon::from_svg_bytes(sensor.graph().into_bytes())
    }

    /// Set to 0 if empty, value if valid, but leave unchanged in value is not valid
    fn set_color(value: &str, color: &mut u8) {
        if value.is_empty() {
            *color = 0;
        } else if let Ok(num) = value.parse::<u8>() {
            *color = num;
        }
    }

    fn save_config(&self) {
        if let Ok(helper) = cosmic::cosmic_config::Config::new(
            match self.core.applet.panel_type {
                PanelType::Panel => APP_ID_PANEL,
                PanelType::Dock => APP_ID_DOCK,
                PanelType::Other(_) => APP_ID_OTHER,
            },
            MinimonConfig::VERSION,
        ) {
            if let Err(err) = self.config.write_entry(&helper) {
                println!("Error writing config {err}");
            }
        }
    }

    fn set_colors(&mut self, colors: GraphColors, kind: DeviceKind) {
        match kind {
            DeviceKind::Cpu(_) => {
                self.config.cpu_colors = colors;
                self.cpu.set_colors(colors);
            }
            DeviceKind::Memory(_) => {
                self.config.mem_colors = colors;
                self.memory.set_colors(colors);
            }
            DeviceKind::Network(_) => {
                self.config.net_colors = colors;
                self.network.set_colors(colors);
            }
        }
    }

    fn set_tick(&mut self) {
        self.tick.store(
            if self.config.refresh_rate % 1000 == 0 {
                1000
            } else if self.config.refresh_rate % 500 == 0 {
                500
            } else {
                250
            },
            atomic::Ordering::Relaxed,
        );
    }

    fn set_network_max_y(&mut self) {
        if self.config.enable_adaptive_net {
            self.network.set_max_y(None);
        } else {
            let unit = self.config.net_unit.unwrap_or(1);
            let multiplier: [u64; 5] = [1, 1000, 1_000_000, 1_000_000_000, 1_000_000_000_000];

            let sec_per_tic: f64 = self.config.refresh_rate as f64 / 1000.0;
            let new_y = (self.config.net_bandwidth * multiplier[unit]) as f64 * sec_per_tic;

            self.network.set_max_y(Some(new_y.round() as u64));
        }
    }

    fn refresh_stats(&mut self) {
        self.cpu.update();
        self.memory.update();
        self.network.update();
    }

    fn spawn_sysmon(&self) {
        if let Some((exec, _application)) = self.sysmon.as_ref() {
            let _child = std::process::Command::new(exec).spawn();
        }
    }

    /// Check if a system monitor application exist
    fn get_sysmon() -> Option<(String, String)> {
        let system_monitors = vec![
            ("observatory", "COSMIC Observatory"),
            ("gnome-system-monitor", "System Monitor"),
            ("xfce4-taskmanager", "Task Manager"),
            ("plasma-systemmonitor", "System Monitor"),
            ("mate-system-monitor", "System Monitor"),
            ("lxqt-taskmanager", "Task Manager"),
        ];

        for (command, name) in system_monitors {
            if let Ok(o) = std::process::Command::new("which").arg(command).output() {
                if !o.stdout.is_empty() {
                    return Some((command.to_string(), name.to_string()));
                }
            }
        }
        None
    }

    fn figure_label<'a>(&self, text: String) -> widget::Text<'a, cosmic::Theme> {
        let size = match self.core.applet.size {
            Size::PanelSize(PanelSize::XL) => self.config.label_size_default + 5,
            Size::PanelSize(PanelSize::L) => self.config.label_size_default + 3,
            Size::PanelSize(PanelSize::M) => self.config.label_size_default + 2,
            Size::PanelSize(PanelSize::S) => self.config.label_size_default + 1,
            Size::PanelSize(PanelSize::XS) => self.config.label_size_default,
            _ => self.config.label_size_default,
        };
        widget::text(text).size(size)
    }
}
