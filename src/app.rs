use cosmic::cosmic_config::CosmicConfigEntry;
use cosmic::iced::alignment::Horizontal;
use std::time;
use sysinfo::System;

use cosmic::app::{Command, Core};
use cosmic::iced::wayland::popup::{destroy_popup, get_popup};
use cosmic::iced::window::Id;
use cosmic::iced::{subscription, Limits};
use cosmic::iced_style::application;
use cosmic::widget::settings;
use cosmic::{
    iced::{gradient::ColorStop, Color, Length},
    widget,
};
use cosmic::{Element, Theme};

use std::sync::atomic::{self, AtomicU64};
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

use crate::colorpicker::ColorPicker;
use crate::config::{GraphColorVariant, GraphColors, GraphKind};
use crate::svgstat::SvgStat;
use crate::{config::MinimonConfig, fl};

const TICK: u64 = 250;

pub const RED_RECT: &str = "<svg width=\"50\" height=\"50\" xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"50\" height=\"50\" x=\"0\" y=\"0\" rx=\"15\" ry=\"15\" fill=\"red\" /></svg>";
pub const GREEN_RECT: &str = "<svg width=\"50\" height=\"50\" xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"50\" height=\"50\" x=\"0\" y=\"0\" rx=\"15\" ry=\"15\" fill=\"green\" /></svg>";
pub const BLUE_RECT: &str = "<svg width=\"50\" height=\"50\" xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"50\" height=\"50\" x=\"0\" y=\"0\" rx=\"15\" ry=\"15\" fill=\"blue\" /></svg>";

const COLOR_STOPS_RED: [ColorStop; 2] = [
    ColorStop {
        offset: 0.0,
        color: Color::from_rgb(0.0, 0.0, 0.0),
    },
    ColorStop {
        offset: 1.0,
        color: Color::from_rgb(1.0, 0.0, 0.0),
    },
];
const COLOR_STOPS_GREEN: [ColorStop; 2] = [
    ColorStop {
        offset: 0.0,
        color: Color::from_rgb(0.0, 0.0, 0.0),
    },
    ColorStop {
        offset: 1.0,
        color: Color::from_rgb(0.0, 1.0, 0.0),
    },
];
const COLOR_STOPS_BLUE: [ColorStop; 2] = [
    ColorStop {
        offset: 0.0,
        color: Color::from_rgb(0.0, 0.0, 0.0),
    },
    ColorStop {
        offset: 1.0,
        color: Color::from_rgb(0.0, 0.0, 1.0),
    },
];

/// This is the struct that represents your application.
/// It is used to define the data that will be used by your application.
pub struct Minimon {
    /// Application state which is managed by the COSMIC runtime.
    core: Core,
    /// Lib for retrieving system stats
    system: System,
    /// Current Total Load Avg in %
    cpu_load: f64,
    /// Current Mem usage in bytes
    mem_usage: f64,
    /// The svg image to draw for the CPU load
    svgstat_cpu: super::svgstat::SvgStat,
    /// The svg image to draw for the Memory load
    svgstat_mem: super::svgstat::SvgStat,
    /// The popup id.
    popup: Option<Id>,
    /// The color picker dialog
    colorpicker: ColorPicker,
    /// Settings stored on disk, including refresh rate, colors, etc.
    config: MinimonConfig,
    /// Countdown timer, as the subscription tick is 250ms
    /// this counter can be set higher and controls refresh/update rate.
    /// Refreshes machine stats when reaching 0 and is reset to configured rate.
    tick_timer: u64,
    /// tick can be 250, 500 or 1000, depending on refresh rate modolu tick
    tick: Arc<AtomicU64>,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,

    ColorPickerOpen(GraphKind),
    ColorPickerClose(bool),
    ColorPickerDefaults,

    ColorPickerSliderRedChanged(u8),
    ColorPickerSliderGreenChanged(u8),
    ColorPickerSliderBlueChanged(u8),
    ColorPickerSelectVariant(GraphColorVariant),

    Tick,
    PopupClosed(Id),
    ToggleTextMode(bool),
    ToggleCpu(bool),
    ToggleMemory(bool),
    ConfigChanged(MinimonConfig),
    RefreshRateUp,
    RefreshRateDown,

    TextInputRedChanged(String),
    TextInputGreenChanged(String),
    TextInputBlueChanged(String),
}

impl cosmic::Application for Minimon {
    type Executor = cosmic::executor::Default;

    type Flags = ();

    type Message = Message;

    const APP_ID: &'static str = "com.github.hyperchaotic.cosmic-applet-minimon";

    fn init(core: Core, _flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let mut system = System::new();
        system.refresh_memory();
        system.refresh_cpu_all();
        let mem_physical = system.total_memory();

        let app = Minimon {
            core,
            system,
            cpu_load: 0.0,
            mem_usage: 0.0,
            svgstat_cpu: super::svgstat::SvgStat::new(100),
            svgstat_mem: super::svgstat::SvgStat::new(mem_physical / 1_073_741_824),
            popup: None,
            colorpicker: ColorPicker::default(),
            config: MinimonConfig::default(),
            tick_timer: TICK,
            tick: Arc::new(AtomicU64::new(TICK)),
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
        fn time_subscription(tick: std::sync::Arc<AtomicU64>) -> Subscription<()> {
            subscription::unfold("time-sub", (), move |()| {
                let atomic = tick.clone();
                async move {
                    let val = atomic.load(atomic::Ordering::Relaxed);
                    let duration = time::Duration::from_millis(val);
                    tokio::time::sleep(duration).await;
                    ((), ())
                }
            })
        }

        Subscription::batch(vec![
            time_subscription(self.tick.clone()).map(|()| Message::Tick),
            self.core
                .watch_config(Self::APP_ID)
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
                formated = format!("{:.2}%", self.cpu_load);
            }

            if !formated.is_empty() {
                formated.push(' ');
            }

            if self.config.enable_mem {
                formated.push_str(&format!("{:.1}GB", self.mem_usage));
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
            let formated_cpu = if self.cpu_load < 10.0 {
                format!("{:.2}%", self.cpu_load)
            } else {
                format!("{:.1}%", self.cpu_load)
            };

            let formated_mem = format!("{:.1}GB", self.mem_usage);

            // vertical layout
            let mut elements = Vec::new();

            if self.config.enable_cpu {
                elements.push(self.core.applet.text(formated_cpu).into());
            }

            if self.config.enable_mem {
                elements.push(self.core.applet.text(formated_mem).into());
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
        if !self.colorpicker.is_active() {
            let mut cpu_elements = Vec::new();
            cpu_elements.push(Element::from(
                self.core
                    .applet
                    .icon_button_from_handle(Minimon::make_icon_handle(&self.svgstat_cpu))
                    .on_press(Message::TogglePopup)
                    .style(cosmic::theme::Button::AppletIcon),
            ));
            cpu_elements.push(Element::from(settings::item(
                fl!("enable-cpu"),
                widget::toggler(None, self.config.enable_cpu, |value| {
                    Message::ToggleCpu(value)
                }),
            )));
            let cpu_row = Row::with_children(cpu_elements)
                .align_items(Alignment::Center)
                .spacing(0);

            let mut mem_elements = Vec::new();
            mem_elements.push(Element::from(
                self.core
                    .applet
                    .icon_button_from_handle(Minimon::make_icon_handle(&self.svgstat_mem))
                    .on_press(Message::TogglePopup)
                    .style(cosmic::theme::Button::AppletIcon),
            ));
            mem_elements.push(Element::from(settings::item(
                fl!("enable-memory"),
                widget::toggler(None, self.config.enable_mem, |value| {
                    Message::ToggleMemory(value)
                }),
            )));
            let mem_row = Row::with_children(mem_elements)
                .align_items(Alignment::Center)
                .spacing(0);

            let mut refresh_elements = Vec::new();

            let button_plus = cosmic::widget::button(Element::from(self.core.applet.text(" - ")))
                .on_press(Message::RefreshRateDown);
            let button_minus = cosmic::widget::button(Element::from(self.core.applet.text(" + ")))
                .on_press(Message::RefreshRateUp);
            let rate_str = format!(" {:.2} ", self.config.refresh_rate as f64 / 1000.0);
            refresh_elements.push(button_plus.into());
            refresh_elements.push(Element::from(self.core.applet.text(rate_str)));
            refresh_elements.push(button_minus.into());

            let refresh_row = Row::with_children(refresh_elements)
                .align_items(Alignment::Center)
                .spacing(0);

            let content_list = widget::list_column()
                .spacing(5)
                .add(settings::item(
                    fl!("text-only"),
                    widget::toggler(None, self.config.text_only, |value| {
                        Message::ToggleTextMode(value)
                    }),
                ))
                .add(Element::from(cpu_row))
                .add(Element::from(mem_row))
                .add(settings::item(
                    fl!("refresh-rate"),
                    Element::from(refresh_row),
                ))
                .add(row!(
                    widget::horizontal_space(Length::Fill),
                    cosmic::widget::button(Element::from(self.core.applet.text(" CPU colors ")))
                        .on_press(Message::ColorPickerOpen(GraphKind::Cpu)),
                    widget::horizontal_space(Length::Fill),
                    cosmic::widget::button(Element::from(self.core.applet.text(" Mem colors ")))
                        .on_press(Message::ColorPickerOpen(GraphKind::Memory)),
                    widget::horizontal_space(Length::Fill)
                ));

            return self.core.applet.popup_container(content_list).into();
        }
        let color = self.colorpicker.sliders();

        let title = format!("{} colors", self.colorpicker.graph_kind);

        let current_variant = self.colorpicker.color_variant;

        let c = widget::list_column()
            .padding(5)
            .spacing(0)
            .add(
                widget::text::title2(title)
                    .width(Length::Fill)
                    .horizontal_alignment(Horizontal::Center),
            )
            .add(
                widget::svg(widget::svg::Handle::from_memory(
                    self.colorpicker.example_svg.svg().into_bytes(),
                ))
                .width(Length::Fill)
                .height(100),
            )
            .add(column!(
                Element::from(
                    row!(
                        widget::horizontal_space(Length::Fill),
                        widget::svg(widget::svg::Handle::from_memory(RED_RECT.as_bytes()))
                            .height(20),
                        widget::horizontal_space(Length::Fill),
                        ColorPicker::color_slider(
                            0..=255,
                            color.red,
                            Message::ColorPickerSliderRedChanged,
                            &COLOR_STOPS_RED
                        ),
                        widget::horizontal_space(Length::Fill),
                        widget::text_input("", color.red.to_string())
                            .width(50)
                            .on_input(Message::TextInputRedChanged),
                        widget::horizontal_space(Length::Fill),
                    )
                    .align_items(Alignment::Center)
                ),
                Element::from(
                    row!(
                        widget::horizontal_space(Length::Fill),
                        widget::svg(widget::svg::Handle::from_memory(GREEN_RECT.as_bytes()))
                            .height(20),
                        widget::horizontal_space(Length::Fill),
                        ColorPicker::color_slider(
                            0..=255,
                            color.green,
                            Message::ColorPickerSliderGreenChanged,
                            &COLOR_STOPS_GREEN
                        ),
                        widget::horizontal_space(Length::Fill),
                        widget::text_input("", color.green.to_string())
                            .width(50)
                            .on_input(Message::TextInputGreenChanged),
                        widget::horizontal_space(Length::Fill),
                    )
                    .align_items(Alignment::Center)
                ),
                Element::from(
                    row!(
                        widget::horizontal_space(Length::Fill),
                        widget::svg(widget::svg::Handle::from_memory(BLUE_RECT.as_bytes()))
                            .height(20),
                        widget::horizontal_space(Length::Fill),
                        ColorPicker::color_slider(
                            0..=255,
                            color.blue,
                            Message::ColorPickerSliderBlueChanged,
                            &COLOR_STOPS_BLUE
                        ),
                        widget::horizontal_space(Length::Fill),
                        widget::text_input("", color.blue.to_string())
                            .width(50)
                            .on_input(Message::TextInputBlueChanged),
                        widget::horizontal_space(Length::Fill),
                    )
                    .align_items(Alignment::Center)
                ),
            ))
            .add(row!(
                widget::radio(
                    GraphColorVariant::RingFront.as_str(),
                    GraphColorVariant::RingFront,
                    if current_variant == GraphColorVariant::RingFront {
                        Some(GraphColorVariant::RingFront)
                    } else {
                        None
                    },
                    |m| { Message::ColorPickerSelectVariant(m) }
                ),
                widget::radio(
                    GraphColorVariant::RingBack.as_str(),
                    GraphColorVariant::RingBack,
                    if current_variant == GraphColorVariant::RingBack {
                        Some(GraphColorVariant::RingBack)
                    } else {
                        None
                    },
                    |m| { Message::ColorPickerSelectVariant(m) }
                ),
                widget::radio(
                    GraphColorVariant::Background.as_str(),
                    GraphColorVariant::Background,
                    if current_variant == GraphColorVariant::Background {
                        Some(GraphColorVariant::Background)
                    } else {
                        None
                    },
                    |m| { Message::ColorPickerSelectVariant(m) }
                ),
                widget::radio(
                    GraphColorVariant::Text.as_str(),
                    GraphColorVariant::Text,
                    if current_variant == GraphColorVariant::Text {
                        Some(GraphColorVariant::Text)
                    } else {
                        None
                    },
                    |m| { Message::ColorPickerSelectVariant(m) }
                )
            ))
            .spacing(10)
            .add(
                row!(
                    widget::button::standard("Defaults").on_press(Message::ColorPickerDefaults),
                    row!(
                        widget::horizontal_space(Length::Fill),
                        widget::button::destructive("Cancel")
                            .on_press(Message::ColorPickerClose(false)),
                        widget::button::suggested("Save").on_press(Message::ColorPickerClose(true))
                    )
                    .width(Length::Fill)
                    .spacing(5)
                    .align_items(Alignment::End)
                )
                .padding(5)
                .spacing(5)
                .width(Length::Fill),
            );

        return self.core.applet.popup_container(c).into();
    }

    /// Application messages are handled here. The application state can be modified based on
    /// what message was received. Commands may be returned for asynchronous execution on a
    /// background thread managed by the application's executor.
    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    self.colorpicker.set_active(false);
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
                if kind == GraphKind::Cpu {
                    self.colorpicker.set_colors(self.config.cpu_colors);
                } else {
                    self.colorpicker.set_colors(self.config.mem_colors);
                }

                self.colorpicker.graph_kind = kind;
                self.colorpicker.set_active(true);

                let col = self
                    .colorpicker
                    .colors()
                    .to_srgb(self.colorpicker.color_variant);
                self.colorpicker.set_sliders(col);
            }

            Message::ColorPickerClose(save) => {
                self.colorpicker.set_active(false);

                if save {
                    self.set_colors(self.colorpicker.colors(), self.colorpicker.graph_kind);
                    self.save_config();
                }
            }

            Message::ColorPickerDefaults => {
                self.colorpicker
                    .set_colors(GraphColors::new(self.colorpicker.graph_kind));

                let col = self
                    .colorpicker
                    .colors()
                    .to_srgb(self.colorpicker.color_variant);
                self.colorpicker.set_sliders(col);
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
            Message::Tick => {
                let tick = self.tick.load(atomic::Ordering::Relaxed);
                if self.tick_timer == 0 {
                    self.tick_timer = self.config.refresh_rate;
                    self.refresh_stats();
                } else if self.tick_timer > tick {
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
            Message::ToggleTextMode(toggled) => {
                self.config.text_only = toggled;
                self.save_config();
            }
            Message::ToggleCpu(toggled) => {
                self.config.enable_cpu = toggled;
                if !toggled {
                    self.config.enable_mem = true;
                }
                self.save_config();
            }
            Message::ToggleMemory(toggled) => {
                self.config.enable_mem = toggled;
                if !toggled {
                    self.config.enable_cpu = true;
                }
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
                self.tick_timer = self.config.refresh_rate;
                self.svgstat_cpu.set_colors(self.config.cpu_colors);
                self.svgstat_mem.set_colors(self.config.mem_colors);
                self.set_tick();
            }

            Message::TextInputRedChanged(value) => {
                Minimon::update_slider_val(&value, &mut self.colorpicker.slider_red_val);
            }
            Message::TextInputGreenChanged(value) => {
                Minimon::update_slider_val(&value, &mut self.colorpicker.slider_green_val);
            }
            Message::TextInputBlueChanged(value) => {
                Minimon::update_slider_val(&value, &mut self.colorpicker.slider_blue_val);
            }
        }
        Command::none()
    }
}

use cosmic::Application;
impl Minimon {
    fn make_icon_handle(svgstat: &SvgStat) -> cosmic::widget::icon::Handle {
        cosmic::widget::icon::from_svg_bytes(svgstat.svg().into_bytes())
    }

    fn save_config(&self) {
        if let Ok(helper) = cosmic::cosmic_config::Config::new(Self::APP_ID, MinimonConfig::VERSION)
        {
            if let Err(err) = self.config.write_entry(&helper) {
                println!("Error writing config {err}");
            }
        }
    }

    fn set_colors(&mut self, colors: GraphColors, kind: GraphKind) {
        match kind {
            GraphKind::Cpu => {
                self.config.cpu_colors = colors;
                self.svgstat_cpu.set_colors(colors);
            }
            GraphKind::Memory => {
                self.config.mem_colors = colors;
                self.svgstat_mem.set_colors(colors);
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

    fn update_slider_val(value: &str, slider: &mut u8) {
        if value.is_empty() {
            *slider = 0;
        } else if let Ok(num) = value.parse::<u8>() {
            *slider = num;
        }
    }

    fn refresh_stats(&mut self) {
        if self.config.enable_cpu {
            self.system.refresh_cpu_usage();
            self.cpu_load = self
                .system
                .cpus()
                .iter()
                .map(|p| f64::from(p.cpu_usage()))
                .sum::<f64>()
                / self.system.cpus().len() as f64;

            self.mem_usage = self.system.used_memory() as f64 / 1_073_741_824.0;
            self.svgstat_cpu.set_variable(self.cpu_load);
        }

        if self.config.enable_cpu {
            self.system.refresh_memory();
            self.svgstat_mem.set_variable(self.mem_usage);
        }
    }
}
