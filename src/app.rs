use cosmic::applet::PanelType;
use cosmic::cosmic_config::CosmicConfigEntry;
use std::time;

use cosmic::app::{Core, Task};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::window::Id;
use cosmic::iced::Limits;
use cosmic::iced::{self, Subscription};
use cosmic::widget::{settings, spin_button, toggler};
use cosmic::Element;
use cosmic::{iced::Length, widget, widget::autosize};

use once_cell::sync::Lazy;
use std::sync::atomic::{self, AtomicI64};
use std::sync::Arc;

use cosmic::{
    applet::cosmic_panel_config::PanelAnchor,
    iced::{
        widget::{column, row, vertical_space},
        Alignment,
    },
    iced_widget::{Column, Row},
    widget::{container, horizontal_space},
};

use crate::colorpicker::{ColorPicker, DemoSvg};
use crate::config::{SvgColorVariant, SvgColors, SvgDevKind, SvgGraphKind};
use crate::netmon::NetMon;
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

    ColorPickerOpen(SvgDevKind),
    ColorPickerClose(bool),
    ColorPickerDefaults,
    ColorPickerAccent,

    ColorPickerSliderRedChanged(u8),
    ColorPickerSliderGreenChanged(u8),
    ColorPickerSliderBlueChanged(u8),
    ColorPickerSelectVariant(SvgColorVariant),

    ColorTextInputRedChanged(String),
    ColorTextInputGreenChanged(String),
    ColorTextInputBlueChanged(String),

    ToggleAdaptiveNet(bool),
    NetworkSelectUnit(usize),
    TextInputBandwidthChanged(String),

    SelectGraphType(SvgDevKind, usize),
    Tick,
    PopupClosed(Id),
    ToggleTextOnly(bool),
    ToggleNet(bool),
    ToggleCpu(bool),
    ToggleMemory(bool),
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
            svgstat_cpu: super::svgstat::SvgStat::new(SvgDevKind::Cpu(SvgGraphKind::Ring)),
            svgstat_mem: super::svgstat::SvgStat::new(SvgDevKind::Memory(SvgGraphKind::Line)),
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

        let suggested_total =
            self.core.applet.suggested_size(true).0 + self.core.applet.suggested_padding(true) * 2;
        let suggested_window_size = self.core.applet.suggested_window_size();
        let (width, height) = if self.core.applet.is_horizontal() {
            (suggested_total as f32, suggested_window_size.1.get() as f32)
        } else {
            (suggested_window_size.0.get() as f32, suggested_total as f32)
        };
        let mut limits = Limits::NONE.min_width(1.).min_height(1.);

        if !self.config.enable_cpu && !self.config.enable_mem && !self.config.enable_net {
            return self
                .core
                .applet
                .icon_button(ICON)
                .on_press(Message::TogglePopup)
                .into();
        }

        // If using SVG we go here and return from within this block
        if !self.config.text_only {
            let mut elements = Vec::new();

            if self.config.enable_cpu {
                let content = self
                    .core
                    .applet
                    .icon_button_from_handle(Minimon::make_icon_handle(&self.svgstat_cpu))
                    .on_press(Message::TogglePopup)
                    .padding(0);
                let content = row!(content, vertical_space().height(Length::Fixed(height)))
                    .align_y(Alignment::Center)
                    .padding(0);
                let content = column!(content, horizontal_space().width(Length::Fixed(width)))
                    .align_x(Alignment::Center)
                    .padding(0);
                elements.push(Element::from(content));
            }

            if self.config.enable_mem {
                let content = self
                    .core
                    .applet
                    .icon_button_from_handle(Minimon::make_icon_handle(&self.svgstat_mem))
                    .on_press(Message::TogglePopup)
                    .padding(0);
                let content = row!(content, vertical_space().height(Length::Fixed(height)))
                    .align_y(Alignment::Center)
                    .padding(0);
                let content = column!(content, horizontal_space().width(Length::Fixed(width)))
                    .align_x(Alignment::Center)
                    .padding(0);
                elements.push(Element::from(content));
            }

            if self.config.enable_net {
                let svg = self.netmon.svg();
                let handle = cosmic::widget::icon::from_svg_bytes(svg.into_bytes());
                let content = self
                    .core
                    .applet
                    .icon_button_from_handle(handle)
                    .on_press(Message::TogglePopup)
                    .padding(0);
                let content = row!(content, vertical_space().height(Length::Fixed(height)))
                    .align_y(Alignment::Center)
                    .padding(0);
                let content = column!(content, horizontal_space().width(Length::Fixed(width)))
                    .align_x(Alignment::Center)
                    .padding(0);
                elements.push(Element::from(content));
            }

            if let Some(b) = self.core.applet.suggested_bounds {
                if b.width as i32 > 0 {
                    limits = limits.max_width(b.width);
                }
                if b.height as i32 > 0 {
                    limits = limits.max_height(b.height);
                }
            }

            if horizontal {
                let row = Row::with_children(elements)
                    .align_y(Alignment::Center)
                    .spacing(0)
                    .padding(0);

                return autosize::autosize(container(row).padding(0), AUTOSIZE_MAIN_ID.clone())
                    .limits(limits)
                    .into();
            }

            let col = Column::with_children(elements)
                .align_x(Alignment::Center)
                .spacing(0)
                .padding(0);

            return autosize::autosize(container(col).padding(0), AUTOSIZE_MAIN_ID.clone())
                .limits(limits)
                .into();
        }

        // If using text only mode instead we go here and just make a button
        let button = widget::button::custom(if horizontal {
            let mut formated = String::new();
            if self.config.enable_cpu {
                formated = format!("{:.2}%", self.svgstat_cpu.latest_sample());
            }

            if self.config.enable_mem {
                if !formated.is_empty() {
                    formated.push(' ');
                }
                formated.push_str(&format!("{:.1}GB", self.svgstat_mem.latest_sample()));
            }

            if self.config.enable_net {
                if !formated.is_empty() {
                    formated.push(' ');
                }
                formated.push_str(&self.netmon.dl_to_string());
                formated.push(' ');
                formated.push_str(&self.netmon.ul_to_string());
            }

            Element::from(row!(self.core.applet.text(formated)).align_y(Alignment::Center))
        } else {
            let formated_cpu = if self.svgstat_cpu.latest_sample() < 10.0 {
                format!("{:.2}%", self.svgstat_cpu.latest_sample())
            } else {
                format!("{:.1}%", self.svgstat_cpu.latest_sample())
            };

            let formated_mem = format!("{:.1}GB", self.svgstat_mem.latest_sample());

            // vertical layout
            let mut elements = Vec::new();

            if self.config.enable_cpu {
                elements.push(self.core.applet.text(formated_cpu).into());
            }

            if self.config.enable_mem {
                elements.push(self.core.applet.text(formated_mem).into());
            }

            if self.config.enable_net {
                elements.push(self.core.applet.text(self.netmon.dl_to_string()).into());
                elements.push(self.core.applet.text(self.netmon.ul_to_string()).into());
            }

            let col = Column::with_children(elements)
                .align_x(Alignment::Center)
                .spacing(0);

            Element::from(column!(col,).align_x(Alignment::Center))
        })
        .padding(if horizontal {
            [0, self.core.applet.suggested_padding(true)]
        } else {
            [self.core.applet.suggested_padding(true), 0]
        })
        .on_press(Message::TogglePopup);

        autosize::autosize(container(button).padding(0), AUTOSIZE_MAIN_ID.clone())
            .limits(limits)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<Self::Message> {
        if self.colorpicker.active() {
            return self
                .core
                .applet
                .popup_container(self.colorpicker.view_colorpicker())
                .into();
        } else {
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
                SvgDevKind::Cpu(m) => Some(m.into()),
                _ => None,
            };

            cpu_elements.push(Element::from(column!(
                Element::from(
                    settings::item(
                        fl!("enable-cpu"),
                        toggler(self.config.enable_cpu)
                            .on_toggle(|value| { Message::ToggleCpu(value) }),
                    )
                    .padding(5)
                ),
                row!(
                    widget::horizontal_space(),
                    widget::dropdown(&self.graph_options, selected, |m| {
                        Message::SelectGraphType(self.svgstat_cpu.kind(), m)
                    },)
                    .width(70),
                    widget::horizontal_space(),
                    widget::button::standard(fl!("change-colors"))
                        .on_press(Message::ColorPickerOpen(self.svgstat_cpu.kind())),
                    widget::horizontal_space()
                )
            )));

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
                SvgDevKind::Memory(m) => Some(m.into()),
                _ => None,
            };

            mem_elements.push(Element::from(column!(
                Element::from(
                    settings::item(
                        fl!("enable-memory"),
                        toggler(self.config.enable_mem)
                            .on_toggle(|value| { Message::ToggleMemory(value) }),
                    )
                    .padding(5)
                ),
                row!(
                    widget::horizontal_space(),
                    widget::dropdown(&self.graph_options, selected, |m| {
                        Message::SelectGraphType(self.svgstat_mem.kind(), m)
                    },)
                    .width(70),
                    widget::horizontal_space(),
                    widget::button::standard(fl!("change-colors"))
                        .on_press(Message::ColorPickerOpen(self.svgstat_mem.kind())),
                    widget::horizontal_space()
                )
            )));

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
            let dlrate = self.netmon.get_bitrate_dl(ticks_per_sec);
            let ulrate = self.netmon.get_bitrate_ul(ticks_per_sec);

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

            net_elements.push(Element::from(column!(
                Element::from(
                    settings::item(
                        fl!("enable-net"),
                        widget::toggler(self.config.enable_net)
                            .on_toggle(|value| { Message::ToggleNet(value) }),
                    )
                    .padding(5)
                ),
                Element::from(
                    settings::item(
                        fl!("use-adaptive"),
                        row!(
                            widget::checkbox("", self.config.enable_adaptive_net)
                                .on_toggle(|v| { Message::ToggleAdaptiveNet(v) }),
                            widget::horizontal_space()
                        ),
                    )
                    .padding(5)
                ),
                Element::from(
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
                        )
                    )
                    .padding(5)
                ),
                row!(
                    widget::horizontal_space(),
                    widget::button::standard(fl!("change-colors"))
                        .on_press(Message::ColorPickerOpen(self.netmon.kind())),
                    widget::horizontal_space()
                ),
            )));

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
                ))
                .add(settings::item(
                    fl!("text-only"),
                    widget::toggler(self.config.text_only).on_toggle(Message::ToggleTextOnly),
                ));
            return self.core.applet.popup_container(content_list).into();
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
                    SvgDevKind::Cpu(_) => {
                        self.colorpicker
                            .activate(kind, Box::new(SvgStat::new(kind)));
                        self.colorpicker.set_colors(self.config.cpu_colors);
                    }
                    SvgDevKind::Memory(_) => {
                        self.colorpicker
                            .activate(kind, Box::new(SvgStat::new(kind)));
                        self.colorpicker.set_colors(self.config.mem_colors);
                    }
                    SvgDevKind::Network(_) => {
                        self.colorpicker.activate(kind, Box::new(NetMon::new()));
                        self.colorpicker.set_colors(self.config.net_colors);
                    }
                }
                self.colorpicker.set_variant(SvgColorVariant::Color1);
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
                    .set_colors(SvgColors::new(self.colorpicker.kind()));
            }

            Message::ColorPickerAccent => {
                let accent = self.core.applet.theme().unwrap().cosmic().accent_color();
                self.colorpicker.set_sliders(accent.color.into());
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
                    SvgDevKind::Cpu(_) => {
                        self.svgstat_cpu.set_kind(SvgDevKind::Cpu(selection.into()));
                        self.config.set_cpu_kind(selection.into());
                    }
                    SvgDevKind::Memory(_) => {
                        self.svgstat_mem
                            .set_kind(SvgDevKind::Memory(selection.into()));
                        self.config.set_memory_kind(selection.into());
                    }
                    SvgDevKind::Network(_) => {
                        self.netmon.set_kind(SvgDevKind::Network(selection.into()));
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
            Message::ToggleTextOnly(toggled) => {
                self.config.text_only = toggled;
                self.save_config();
            }
            Message::ToggleCpu(toggled) => {
                self.config.enable_cpu = toggled;
                self.save_config();
            }
            Message::ToggleMemory(toggled) => {
                self.config.enable_mem = toggled;
                self.save_config();
            }
            Message::ToggleNet(toggled) => {
                self.config.enable_net = toggled;
                self.save_config();
            }

            Message::ConfigChanged(config) => {
                self.config = config;
                self.tick_timer = self.config.refresh_rate as i64;
                self.svgstat_cpu.svg_set_colors(self.config.cpu_colors);
                self.svgstat_cpu.set_kind(self.config.cpu_kind());
                self.svgstat_mem.svg_set_colors(self.config.mem_colors);
                self.svgstat_mem.set_kind(self.config.memory_kind());
                self.netmon.svg_set_colors(self.config.net_colors);
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

    fn set_colors(&mut self, colors: SvgColors, kind: SvgDevKind) {
        match kind {
            SvgDevKind::Cpu(_) => {
                self.config.cpu_colors = colors;
                self.svgstat_cpu.svg_set_colors(colors);
            }
            SvgDevKind::Memory(_) => {
                self.config.mem_colors = colors;
                self.svgstat_mem.svg_set_colors(colors);
            }
            SvgDevKind::Network(_) => {
                self.config.net_colors = colors;
                self.netmon.svg_set_colors(colors);
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
            self.netmon
                .set_max_y(Some(self.config.net_bandwidth * multiplier[unit]));
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
