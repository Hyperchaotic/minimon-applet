use cosmic::applet::PanelType;
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
use crate::netmon::{NetMon, UnitVariant};
use crate::svgstat::SvgStat;
use crate::{config::MinimonConfig, fl};
use cosmic::widget::Id as WId;

static AUTOSIZE_MAIN_ID: Lazy<WId> = Lazy::new(|| WId::new("autosize-main"));

const TICK: i64 = 250;

const ICON: &str = "com.github.hyperchaotic.cosmic-applet-minimon";
/// This is the struct that represents your application.
/// It is used to define the data that will be used by your application.
pub struct Minimon {
    /// Application state which is managed by the COSMIC runtime.
    core: Core,
    /// The svg image to draw for the CPU load
    svgstat_cpu: super::svgstat::SvgStat,
    /// The svg image to draw for the Memory load
    svgstat_mem: super::svgstat::SvgStat,
    /// The popup id.
    popup: Option<Id>,
    /// The color picker dialog
    colorpicker: ColorPicker,
    dropdown_options: Vec<&'static str>,
    graph_options: Vec<&'static str>,

    /// The network monitor
    netmon: NetMon,
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

    SelectGraphType(DeviceKind, usize),
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
            svgstat_cpu: super::svgstat::SvgStat::new(DeviceKind::Cpu(GraphKind::Ring)),
            svgstat_mem: super::svgstat::SvgStat::new(DeviceKind::Memory(GraphKind::Line)),
            popup: None,
            colorpicker: ColorPicker::new(),
            dropdown_options: ["b", "Kb", "Mb", "Gb", "Tb"].into(),
            graph_options: ["Ring", "Line"].into(),
            netmon: NetMon::new(),
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

        let formated_cpu = if self.svgstat_cpu.latest_sample() < 10.0 {
            format!("{:.2}%", self.svgstat_cpu.latest_sample())
        } else {
            format!("{:.1}%", self.svgstat_cpu.latest_sample())
        };

        let formated_mem = format!("{:.1}GB", self.svgstat_mem.latest_sample());

        // vertical layout
        let mut elements: Vec<Element<Message>> = Vec::new();

        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();

        if self.config.enable_cpu_label {
            elements.push(self.core.applet.text(formated_cpu).into());
        }

        if self.config.enable_cpu_chart {
            let content = self
                .core
                .applet
                .icon_button_from_handle(Minimon::make_icon_handle(&self.svgstat_cpu));

            elements.push(content.into());
        }

        if self.config.enable_mem_label {
            elements.push(self.core.applet.text(formated_mem).into());
        }

        if self.config.enable_mem_chart {
            let content = self
                .core
                .applet
                .icon_button_from_handle(Minimon::make_icon_handle(&self.svgstat_mem));

            elements.push(content.into());
        }

        // Network
        if self.config.enable_net_label {
            let ticks_per_sec = (1000 / self.tick.clone().load(atomic::Ordering::Relaxed)) as usize;
            if horizontal {
                // DL
                let mut dlstr = String::with_capacity(10);
                dlstr.push('↓');
                dlstr.push_str(&self.netmon.get_bitrate_dl(ticks_per_sec, UnitVariant::Long));
                elements.push(self.core.applet.text(dlstr).into());
                // UL
                let mut ulstr = String::with_capacity(10);
                ulstr.push('↑');
                ulstr.push_str(&self.netmon.get_bitrate_ul(ticks_per_sec, UnitVariant::Long));
                elements.push(self.core.applet.text(ulstr).into());
            } else {
                elements.push(
                    self.core
                        .applet
                        .text(
                            self.netmon
                                .get_bitrate_dl(ticks_per_sec, UnitVariant::Short),
                        )
                        .into(),
                );
                elements.push(
                    self.core
                        .applet
                        .text(
                            self.netmon
                                .get_bitrate_ul(ticks_per_sec, UnitVariant::Short),
                        )
                        .into(),
                );
            }
        }

        if self.config.enable_net_chart {
            let svg = self.netmon.svg();
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
            self.core
                .applet
                .popup_container(self.colorpicker.view_colorpicker())
                .into()
        } else {
            let theme = cosmic::theme::active();
            let cosmic = theme.cosmic();

            let mut cpu_elements = Vec::new();

            let cpu = self.svgstat_cpu.to_string();
            cpu_elements.push(Element::from(
                column!(
                    widget::svg(widget::svg::Handle::from_memory(
                        self.svgstat_cpu.svg().as_bytes().to_owned(),
                    ))
                    .width(80)
                    .height(60),
                    cosmic::widget::text::body(cpu),
                )
                .padding(5)
                .align_x(Alignment::Center),
            ));

            let selected: Option<usize> = match self.svgstat_cpu.kind() {
                DeviceKind::Cpu(m) => Some(m.into()),
                _ => None,
            };

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
                        widget::dropdown(&self.graph_options, selected, |m| {
                            Message::SelectGraphType(self.svgstat_cpu.kind(), m)
                        },)
                        .width(70),
                        widget::horizontal_space(),
                        widget::button::standard(fl!("change-colors"))
                            .on_press(Message::ColorPickerOpen(self.svgstat_cpu.kind())),
                    )
                )
                .spacing(cosmic.space_xs()),
            ));

            let cpu_row = Row::with_children(cpu_elements)
                .align_y(Alignment::Center)
                .spacing(0);

            let mut mem_elements = Vec::new();
            let mem = self.svgstat_mem.to_string();
            mem_elements.push(Element::from(
                column!(
                    widget::svg(widget::svg::Handle::from_memory(
                        self.svgstat_mem.svg().as_bytes().to_owned(),
                    ))
                    .width(80)
                    .height(60),
                    cosmic::widget::text::body(mem),
                )
                .padding(5)
                .align_x(Alignment::Center),
            ));

            let selected: Option<usize> = match self.svgstat_mem.kind() {
                DeviceKind::Memory(m) => Some(m.into()),
                _ => None,
            };

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
                        widget::dropdown(&self.graph_options, selected, |m| {
                            Message::SelectGraphType(self.svgstat_mem.kind(), m)
                        },)
                        .width(70),
                        widget::horizontal_space(),
                        widget::button::standard(fl!("change-colors"))
                            .on_press(Message::ColorPickerOpen(self.svgstat_mem.kind())),
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

            let mut dlrate = '↓'.to_string();
            dlrate.push_str(&self.netmon.get_bitrate_dl(ticks_per_sec, UnitVariant::Long));
            let mut ulrate = '↑'.to_string();
            ulrate.push_str(&self.netmon.get_bitrate_ul(ticks_per_sec, UnitVariant::Long));

            net_elements.push(Element::from(
                column!(
                    widget::svg(widget::svg::Handle::from_memory(
                        self.netmon.svg().as_bytes().to_owned(),
                    ))
                    .width(80)
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
                        .on_toggle(|value| Message::ToggleNetChart(value)),
                )
                .into(),
            );
            net_bandwidth_items.push(
                settings::item(
                    fl!("enable-net-label"),
                    widget::toggler(self.config.enable_net_label)
                        .on_toggle(|value| Message::ToggleNetLabel(value)),
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
                        .on_press(Message::ColorPickerOpen(self.netmon.kind())),
                    widget::horizontal_space()
                )
                .into(),
            );

            let net_right_column = Column::with_children(net_bandwidth_items);

            net_elements.push(Element::from(net_right_column.spacing(cosmic.space_xs())));

            let net_row = Row::with_children(net_elements)
                .align_y(Alignment::Center)
                .spacing(0);

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
                ));
            self.core.applet.popup_container(content_list).into()
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
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(400.0)
                        .min_width(200.0)
                        .min_height(200.0)
                        .max_height(720.0);
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
                            .activate(kind, Box::new(SvgStat::new(kind)));
                        self.colorpicker.set_colors(self.config.cpu_colors);
                    }
                    DeviceKind::Memory(_) => {
                        self.colorpicker
                            .activate(kind, Box::new(SvgStat::new(kind)));
                        self.colorpicker.set_colors(self.config.mem_colors);
                    }
                    DeviceKind::Network(_) => {
                        self.colorpicker.activate(kind, Box::new(NetMon::new()));
                        self.colorpicker.set_colors(self.config.net_colors);
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
                self.colorpicker
                    .set_colors(GraphColors::new(self.colorpicker.kind()));
            }

            Message::ColorPickerAccent => {
                if let Some(theme) = self.core.applet.theme() {
                    let accent = theme.cosmic().accent_color().color;
                    let srgba = cosmic::cosmic_theme::palette::Srgba::from_color(accent);
                    self.colorpicker
                        .set_sliders(srgba.opaque().into());
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
                    self.netmon.set_max_y(None);
                }
                self.save_config();
            }

            Message::NetworkSelectUnit(unit) => {
                if !self.config.enable_adaptive_net {
                    self.config.net_unit = Some(unit);
                    self.set_max_y();
                    self.save_config();
                }
            }

            Message::SelectGraphType(dev, selection) => {
                match dev {
                    DeviceKind::Cpu(_) => {
                        self.svgstat_cpu.set_kind(DeviceKind::Cpu(selection.into()));
                        self.config.set_cpu_kind(selection.into());
                    }
                    DeviceKind::Memory(_) => {
                        self.svgstat_mem
                            .set_kind(DeviceKind::Memory(selection.into()));
                        self.config.set_memory_kind(selection.into());
                    }
                    DeviceKind::Network(_) => {
                        self.netmon.set_kind(DeviceKind::Network(selection.into()));
                    }
                }
                self.save_config();
            }

            Message::TextInputBandwidthChanged(string) => {
                if string.is_empty() {
                    self.config.net_bandwidth = 0;
                    self.set_max_y();
                    self.save_config();
                } else if !self.config.enable_adaptive_net {
                    if let Ok(val) = string.parse::<u64>() {
                        self.config.net_bandwidth = val;
                        self.set_max_y();
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
                self.svgstat_cpu.set_colors(self.config.cpu_colors);
                self.svgstat_cpu.set_kind(self.config.cpu_kind());
                self.svgstat_mem.set_colors(self.config.mem_colors);
                self.svgstat_mem.set_kind(self.config.memory_kind());
                self.netmon.set_colors(self.config.net_colors);
                self.set_max_y();
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
        }
        Task::none()
    }
}

impl Minimon {
    fn make_icon_handle(svgstat: &SvgStat) -> cosmic::widget::icon::Handle {
        cosmic::widget::icon::from_svg_bytes(svgstat.svg().into_bytes())
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
                self.svgstat_cpu.set_colors(colors);
            }
            DeviceKind::Memory(_) => {
                self.config.mem_colors = colors;
                self.svgstat_mem.set_colors(colors);
            }
            DeviceKind::Network(_) => {
                self.config.net_colors = colors;
                self.netmon.set_colors(colors);
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

    fn set_max_y(&mut self) {
        if self.config.enable_adaptive_net {
            self.netmon.set_max_y(None);
        } else {
            let unit = self.config.net_unit.unwrap_or(1);
            let multiplier: [u64; 5] = [1, 1000, 1_000_000, 1_000_000_000, 1_000_000_000_000];

            let sec_per_tic: f64 = self.config.refresh_rate as f64 / 1000.0;
            let new_y = (self.config.net_bandwidth * multiplier[unit]) as f64 * sec_per_tic;

            self.netmon.set_max_y(Some(new_y.round() as u64));
        }
    }

    fn refresh_stats(&mut self) {
        self.svgstat_cpu.update();
        self.svgstat_mem.update();
        self.netmon.update();
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
}
