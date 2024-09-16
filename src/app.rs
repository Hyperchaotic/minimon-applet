use cosmic::applet::PanelType;
use cosmic::cosmic_config::CosmicConfigEntry;
use std::time;

use cosmic::app::{Command, Core};
use cosmic::iced::wayland::popup::{destroy_popup, get_popup};
use cosmic::iced::window::Id;
use cosmic::iced::{subscription, Limits};
use cosmic::iced_style::application;
use cosmic::widget::{settings, text};
use cosmic::{iced::Length, widget};
use cosmic::{Element, Theme};

use std::sync::atomic::{self, AtomicI64};
use std::sync::Arc;

use cosmic::{
    applet::cosmic_panel_config::PanelAnchor,
    iced::{
        widget::{column, row, vertical_space},
        Alignment, Subscription,
    },
    iced_widget::{Column, Row},
    widget::{container, horizontal_space},
};

use crate::colorpicker::{ColorPicker, DemoSvg};
use crate::config::{SvgColorVariant, SvgColors, SvgDevKind, SvgGraphKind};
use crate::netmon::NetMon;
use crate::svgstat::SvgStat;
use crate::{config::MinimonConfig, fl};

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
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,

    ColorPickerOpen(SvgDevKind),
    ColorPickerClose(bool),
    ColorPickerDefaults,

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
    RefreshRateUp,
    RefreshRateDown,
}

const APP_ID_DOCK: &str = "com.github.hyperchaotic.cosmic-applet-minimon-dock";
const APP_ID_PANEL: &str = "com.github.hyperchaotic.cosmic-applet-minimon-panel";
const APP_ID_OTHER: &str = "com.github.hyperchaotic.cosmic-applet-minimon-other";

impl cosmic::Application for Minimon {
    type Executor = cosmic::executor::Default;

    type Flags = ();

    type Message = Message;

    const APP_ID: &'static str = "com.github.hyperchaotic.cosmic-applet-minimon";

    fn init(core: Core, _flags: Self::Flags) -> (Self, Command<Self::Message>) {
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
        };

        (app, Command::none())
    }

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn style(&self) -> Option<<Theme as application::StyleSheet>::Style> {
        Some(cosmic::applet::style())
    }

    fn subscription(&self) -> Subscription<Message> {
        fn time_subscription(tick: std::sync::Arc<AtomicI64>) -> Subscription<()> {
            subscription::unfold("time-sub", (), move |()| {
                let atomic = tick.clone();
                async move {
                    let val = atomic.load(atomic::Ordering::Relaxed);
                    let duration = time::Duration::from_millis(val as u64);
                    tokio::time::sleep(duration).await;
                    ((), ())
                }
            })
        }

        Subscription::batch(vec![
            time_subscription(self.tick.clone()).map(|()| Message::Tick),
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
                let cpu_widget = Element::from(
                    self.core
                        .applet
                        .icon_button_from_handle(Minimon::make_icon_handle(&self.svgstat_cpu))
                        .on_press(Message::TogglePopup)
                        .style(cosmic::theme::Button::AppletIcon),
                );
                elements.push(cpu_widget);
            }

            if self.config.enable_mem {
                let mem_widget = Element::from(
                    self.core
                        .applet
                        .icon_button_from_handle(Minimon::make_icon_handle(&self.svgstat_mem))
                        .on_press(Message::TogglePopup)
                        .style(cosmic::theme::Button::AppletIcon),
                );

                elements.push(mem_widget);
            }

            if self.config.enable_net {
                let svg = self.netmon.svg();
                let handle = cosmic::widget::icon::from_svg_bytes(svg.into_bytes());

                let net_widget = Element::from(
                    self.core
                        .applet
                        .icon_button_from_handle(handle)
                        .on_press(Message::TogglePopup)
                        .style(cosmic::theme::Button::AppletIcon),
                );
                elements.push(net_widget);
            }

            if horizontal {
                let row = Row::with_children(elements)
                    .align_items(Alignment::Center)
                    .spacing(0);

                return Element::from(row!(row));
            }

            let col = Column::with_children(elements)
                .align_items(Alignment::Center)
                .spacing(0);

            return Element::from(row!(col)); // returning SVG elements here
        }

        // If using text only mode instead we go here and just make a button
        let button = cosmic::widget::button(if horizontal {
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

            Element::from(
                row!(
                    self.core.applet.text(formated),
                    container(vertical_space(Length::Fixed(f32::from(
                        self.core.applet.suggested_size(true).1
                            + 2 * self.core.applet.suggested_padding(true)
                    ))))
                )
                .align_items(Alignment::Center),
            )
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
                .align_items(Alignment::Center)
                .spacing(0);

            Element::from(
                column!(
                    col,
                    horizontal_space(Length::Fixed(f32::from(
                        self.core.applet.suggested_size(true).0
                            + 2 * self.core.applet.suggested_padding(true)
                    )))
                )
                .align_items(Alignment::Center),
            )
        })
        .padding(if horizontal {
            [0, self.core.applet.suggested_padding(true)]
        } else {
            [self.core.applet.suggested_padding(true), 0]
        })
        .on_press(Message::TogglePopup)
        .style(cosmic::theme::Button::AppletIcon);

        button.into()
    }

    fn view_window(&self, _id: Id) -> Element<Self::Message> {
        if self.colorpicker.active() {
            self.colorpicker.view_colorpicker()
        } else {
            let mut cpu_elements = Vec::new();

            let cpu = self.svgstat_cpu.to_string();
            cpu_elements.push(Element::from(
                column!(
                    widget::svg(widget::svg::Handle::from_memory(
                        self.svgstat_cpu.svg().as_bytes().to_owned(),
                    ))
                    .width(60)
                    .height(60),
                    cosmic::widget::text::body(cpu),
                )
                .padding(5)
                .align_items(Alignment::Center),
            ));

            let selected: Option<usize> = match self.svgstat_cpu.kind() {
                SvgDevKind::Cpu(m) => Some(m.into()),
                _ => None,
            };

            cpu_elements.push(Element::from(column!(
                Element::from(
                    settings::item(
                        fl!("enable-cpu"),
                        widget::toggler(None, self.config.enable_cpu, |value| {
                            Message::ToggleCpu(value)
                        }),
                    )
                    .padding(5)
                ),
                row!(
                    widget::horizontal_space(Length::Fill),
                    widget::dropdown(&self.graph_options, selected, |m| {
                        Message::SelectGraphType(self.svgstat_cpu.kind(), m)
                    },)
                    .width(70),
                    widget::horizontal_space(Length::Fill),
                    widget::button::standard(fl!("change-colors"))
                        .on_press(Message::ColorPickerOpen(self.svgstat_cpu.kind())),
                    widget::horizontal_space(Length::Fill)
                )
            )));

            let cpu_row = Row::with_children(cpu_elements)
                .align_items(Alignment::Center)
                .spacing(0);

            let mut mem_elements = Vec::new();
            let mem = self.svgstat_mem.to_string();
            mem_elements.push(Element::from(
                column!(
                    widget::svg(widget::svg::Handle::from_memory(
                        self.svgstat_mem.svg().as_bytes().to_owned(),
                    ))
                    .width(60)
                    .height(60),
                    cosmic::widget::text::body(mem),
                )
                .padding(5)
                .align_items(Alignment::Center),
            ));

            let selected: Option<usize> = match self.svgstat_mem.kind() {
                SvgDevKind::Memory(m) => Some(m.into()),
                _ => None,
            };

            mem_elements.push(Element::from(column!(
                Element::from(
                    settings::item(
                        fl!("enable-memory"),
                        widget::toggler(None, self.config.enable_mem, |value| {
                            Message::ToggleMemory(value)
                        }),
                    )
                    .padding(5)
                ),
                row!(
                    widget::horizontal_space(Length::Fill),
                    widget::dropdown(&self.graph_options, selected, |m| {
                        Message::SelectGraphType(self.svgstat_mem.kind(), m)
                    },)
                    .width(70),
                    widget::horizontal_space(Length::Fill),
                    widget::button::standard(fl!("change-colors"))
                        .on_press(Message::ColorPickerOpen(self.svgstat_mem.kind())),
                    widget::horizontal_space(Length::Fill)
                )
            )));

            let mem_row = Row::with_children(mem_elements)
                .align_items(Alignment::Center)
                .spacing(0);

            let mut refresh_elements = Vec::new();

            let button_plus = widget::button::standard("-").on_press(Message::RefreshRateDown);
            let button_minus = widget::button::standard("+").on_press(Message::RefreshRateUp);

            let rate_str = format!(" {:.2} ", self.config.refresh_rate as f64 / 1000.0);
            refresh_elements.push(button_plus.into());
            refresh_elements.push(text::body(rate_str).into());
            refresh_elements.push(button_minus.into());

            let refresh_row = Row::with_children(refresh_elements)
                .align_items(Alignment::Center)
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
                    .width(60)
                    .height(60),
                    cosmic::widget::text::body(""),
                    cosmic::widget::text::body(dlrate),
                    cosmic::widget::text::body(ulrate),
                )
                .padding(5)
                .align_items(Alignment::Center),
            ));

            net_elements.push(Element::from(column!(
                Element::from(
                    settings::item(
                        fl!("enable-net"),
                        widget::toggler(None, self.config.enable_net, |value| {
                            Message::ToggleNet(value)
                        }),
                    )
                    .padding(5)
                ),
                Element::from(
                    settings::item(
                        fl!("use-adaptive"),
                        row!(
                            widget::checkbox("", self.config.enable_adaptive_net, |v| {
                                Message::ToggleAdaptiveNet(v)
                            }),
                            widget::horizontal_space(15)
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
                    widget::horizontal_space(Length::Fill),
                    widget::button::standard(fl!("change-colors"))
                        .on_press(Message::ColorPickerOpen(self.netmon.kind())),
                    widget::horizontal_space(Length::Fill)
                ),
            )));

            let net_row = Row::with_children(net_elements)
                .align_items(Alignment::Center)
                .spacing(0);

            let content_list = widget::list_column()
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
                    widget::toggler(None, self.config.text_only, |value| {
                        Message::ToggleTextOnly(value)
                    }),
                ));

            return self.core.applet.popup_container(content_list).into();
        }
    }

    /// Application messages are handled here. The application state can be modified based on
    /// what message was received. Commands may be returned for asynchronous execution on a
    /// background thread managed by the application's executor.
    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    self.colorpicker.active();
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings =
                        self.core
                            .applet
                            .get_popup_settings(Id::MAIN, new_id, None, None, None);
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(372.0)
                        .min_width(300.0)
                        .min_height(200.0)
                        .max_height(1080.0);
                    get_popup(popup_settings)
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
                self.colorpicker.active();
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

            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
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
            Message::RefreshRateUp => {
                if self.config.refresh_rate < 10000 {
                    self.config.refresh_rate += 250;
                }
                self.set_tick();
                self.save_config();
            }
            Message::RefreshRateDown => {
                if self.config.refresh_rate >= 500 {
                    self.config.refresh_rate -= 250;
                }
                self.set_tick();
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
        }
        Command::none()
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
            let unit = if let Some(u) = self.config.net_unit {
                u
            } else {
                1
            };
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
}
