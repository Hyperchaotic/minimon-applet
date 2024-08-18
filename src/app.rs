use cosmic::cosmic_config::CosmicConfigEntry;
use cosmic::cosmic_theme::palette::Srgb;
use cosmic::iced::alignment::Horizontal;
use std::time;
use sysinfo::System;

use cosmic::app::{Command, Core};
use cosmic::iced::wayland::popup::{destroy_popup, get_popup};
use cosmic::iced::window::Id;
use cosmic::iced::{subscription, Limits};
use cosmic::iced_style::application;
use cosmic::widget::{self, settings};
use cosmic::{Element, Theme};

//use chrono::{Datelike, DurationRound, Timelike};
use cosmic::{
    applet::cosmic_panel_config::PanelAnchor,
    iced::{
        widget::{column, row, vertical_space},
        Alignment, Length, Subscription,
    },
    iced_widget::{Column, Row},
    widget::{container, horizontal_space},
};

use crate::config::{GraphColorVariant, GraphColors, GraphKind};
use crate::svgstat::SvgStat;
use crate::{config::MinimonConfig, fl};

const TICK: u64 = 250;

const RED_RECT: &str = "<svg width=\"50\" height=\"50\" xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"50\" height=\"50\" x=\"0\" y=\"0\" rx=\"15\" ry=\"15\" fill=\"red\" /></svg>";
const GREEN_RECT: &str = "<svg width=\"50\" height=\"50\" xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"50\" height=\"50\" x=\"0\" y=\"0\" rx=\"15\" ry=\"15\" fill=\"green\" /></svg>";
const BLUE_RECT: &str = "<svg width=\"50\" height=\"50\" xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"50\" height=\"50\" x=\"0\" y=\"0\" rx=\"15\" ry=\"15\" fill=\"blue\" /></svg>";

#[derive(Debug, Clone, PartialEq)]
pub struct ColorPicker {
    /// If dialog is active this is not None
    pub active: bool,
    /// Type current displaying
    pub graph_kind: GraphKind,
    /// Colors for the example svg @todo should the svg class embed the graphcolors class?
    pub graph_colors: GraphColors,
    // Current field being adjusted
    pub color_variant: GraphColorVariant,
    /// An example SVG to show the changes
    pub example_svg: SvgStat,
    ///Current slider values
    pub slider_red_val: u8,
    pub slider_green_val: u8,
    pub slider_blue_val: u8,
}

impl Default for ColorPicker {
    fn default() -> Self {
        let mut dev = SvgStat::new(100);
        dev.set_variable(50.0);
        Self {
            active: false,
            graph_kind: GraphKind::Cpu,
            graph_colors: GraphColors::default(),
            color_variant: GraphColorVariant::RingFront,
            example_svg: dev,
            slider_red_val: 0,
            slider_green_val: 0,
            slider_blue_val: 0,
        }
    }
}

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
    colorpicker: ColorPicker,
    config: MinimonConfig,
    /// Countdown timer, as the subscription tick is 250ms
    /// this counter can be set higher and controls refresh/update rate.
    /// Refreshes machine stats when reaching 0 and is reset to configured rate.
    tick_timer: u64,
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
}

impl cosmic::Application for Minimon {
    //    type Executor = cosmic::SingleThreadExecutor;
    type Executor = cosmic::executor::Default;

    type Flags = ();

    type Message = Message;

    const APP_ID: &'static str = "com.github.hyperchaotic.cosmic-applet-minimon";

    fn init(core: Core, _flags: Self::Flags) -> (Self, Command<Self::Message>) {
        println!("=====init=====");
        let mut system = System::new();
        system.refresh_memory();
        let mem_physical = system.total_memory(); // To GB

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
            tick_timer: 1000,
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
        fn time_subscription() -> Subscription<()> {
            subscription::unfold("time-sub", (), move |()| async move {
                let duration = time::Duration::from_millis(TICK);
                tokio::time::sleep(duration).await;
                ((), ())
            })
        }

        Subscription::batch(vec![
            time_subscription().map(|()| Message::Tick),
            self.core
                .watch_config(Self::APP_ID)
                .map(|u| Message::ConfigChanged(u.config)),
        ])
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<Message> {
        // println!("=====view=====");

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
        //println!("=====view_window=====");
        if !self.colorpicker.active {
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

            let button_plus =
                cosmic::widget::button(Element::from(self.core.applet.text(" - "))).on_press(
                    Message::RefreshRateDown,
                );
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
                .padding(5)
                .spacing(0)
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

                ).padding(10));

            return self.core.applet.popup_container(content_list).into();
        }
        let title = format!("{:?} colors", self.colorpicker.graph_kind);
        let red_val = format!("  {}", self.colorpicker.slider_red_val);
        let green_val = format!("  {}", self.colorpicker.slider_green_val);
        let blue_val = format!("  {}", self.colorpicker.slider_blue_val);

        let current_variant = self.colorpicker.color_variant;

        let c = widget::list_column()
            .padding(5)
            .spacing(5)
            .add(
                widget::text::title2(title)
                    .width(Length::Fill)
                    .horizontal_alignment(Horizontal::Center),
            )
            .add(
                widget::svg(widget::svg::Handle::from_memory(
                    self.colorpicker.example_svg.to_string().into_bytes(),
                ))
                .width(Length::Fill)
                .height(100),
            )
            .add(row!(
                widget::svg(widget::svg::Handle::from_memory(RED_RECT.as_bytes()))
                    .width(Length::Fill)
                    .height(20),
                widget::slider(
                    0..=255,
                    self.colorpicker.slider_red_val,
                    Message::ColorPickerSliderRedChanged
                )
                .width(Length::Fixed(250.0))
                .height(38),
                widget::text("  "),
                widget::text_input("", red_val).width(50)
            ))
            .add(row!(
                widget::svg(widget::svg::Handle::from_memory(GREEN_RECT.as_bytes()))
                    
                    .height(20),
                widget::slider(
                    0..=255,
                    self.colorpicker.slider_green_val,
                    Message::ColorPickerSliderGreenChanged
                )
                .width(Length::Fixed(250.0))
                .height(38),
                widget::text("  "),
                widget::text_input("", green_val).width(50)
            ))
            .add(row!(
                widget::svg(widget::svg::Handle::from_memory(BLUE_RECT.as_bytes()))
                    .width(Length::Fill)
                    .height(20),
                widget::slider(
                    0..=255,
                    self.colorpicker.slider_blue_val,
                    Message::ColorPickerSliderBlueChanged
                )
                .width(Length::Fixed(250.0))
                .height(38),
                widget::text("  "),
                widget::text_input("", blue_val).width(50)
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
        //        println!("=====update {:?}=====", message);
        match message {
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    self.colorpicker.active = false;
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
                    self.colorpicker.graph_colors = self.config.cpu_colors;
                } else {
                    self.colorpicker.graph_colors = self.config.mem_colors;
                }
                
                self.colorpicker.graph_kind = kind;
                self.colorpicker.active = true;
                let col = self
                    .colorpicker
                    .graph_colors
                    .to_srgb(self.colorpicker.color_variant);
                self.colorpicker.slider_red_val = col.red;
                self.colorpicker.slider_green_val = col.green;
                self.colorpicker.slider_blue_val = col.blue;
                self.colorpicker
                    .example_svg
                    .set_colors(&self.colorpicker.graph_colors);
            }

            Message::ColorPickerClose(save) => {
                self.colorpicker.active = false;

                if save {
                    if self.colorpicker.graph_kind == GraphKind::Cpu {
                        self.config.cpu_colors = self.colorpicker.graph_colors;
                        self.svgstat_cpu.set_colors(&self.config.cpu_colors);
                    } else {
                        self.config.mem_colors = self.colorpicker.graph_colors;
                        self.svgstat_mem.set_colors(&self.config.mem_colors);
                    }
                }

                self.save_config();
            }

            Message::ColorPickerDefaults => {
                self.colorpicker.graph_colors = GraphColors::new(self.colorpicker.graph_kind);
                self.colorpicker
                    .example_svg
                    .set_colors(&self.colorpicker.graph_colors);
                let col = self
                    .colorpicker
                    .graph_colors
                    .to_srgb(self.colorpicker.color_variant);
                self.colorpicker.slider_red_val = col.red;
                self.colorpicker.slider_green_val = col.green;
                self.colorpicker.slider_blue_val = col.blue;
            }

            Message::ColorPickerSliderRedChanged(val) => {
                self.colorpicker.slider_red_val = val;
                self.colorpicker.graph_colors.set_color(
                    Srgb::from_components((
                        self.colorpicker.slider_red_val,
                        self.colorpicker.slider_green_val,
                        self.colorpicker.slider_blue_val,
                    )),
                    self.colorpicker.color_variant,
                );
                self.colorpicker
                    .example_svg
                    .set_colors(&self.colorpicker.graph_colors);
            }

            Message::ColorPickerSliderGreenChanged(val) => {
                self.colorpicker.slider_green_val = val;
                self.colorpicker.graph_colors.set_color(
                    Srgb::from_components((
                        self.colorpicker.slider_red_val,
                        self.colorpicker.slider_green_val,
                        self.colorpicker.slider_blue_val,
                    )),
                    self.colorpicker.color_variant,
                );
                self.colorpicker
                    .example_svg
                    .set_colors(&self.colorpicker.graph_colors);
            }

            Message::ColorPickerSliderBlueChanged(val) => {
                self.colorpicker.slider_blue_val = val;
                self.colorpicker.graph_colors.set_color(
                    Srgb::from_components((
                        self.colorpicker.slider_red_val,
                        self.colorpicker.slider_green_val,
                        self.colorpicker.slider_blue_val,
                    )),
                    self.colorpicker.color_variant,
                );
                self.colorpicker
                    .example_svg
                    .set_colors(&self.colorpicker.graph_colors);
            }

            Message::ColorPickerSelectVariant(e) => {
                self.colorpicker.color_variant = e;
                let col = self.colorpicker.graph_colors.to_srgb(e);
                println!("e: {e:?}. col: {col:?}");
                self.colorpicker.slider_red_val = col.red;
                self.colorpicker.slider_green_val = col.green;
                self.colorpicker.slider_blue_val = col.blue;
            }
            Message::Tick => {
                if self.tick_timer > 0 {
                    self.tick_timer -= TICK;
                } else {
                    self.refresh_stats();
                    self.tick_timer = self.config.refresh_rate;
                }
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
                    self.config.refresh_rate += TICK;
                }
                self.save_config();
            }
            Message::RefreshRateDown => {
                if self.config.refresh_rate >= 2 * TICK {
                    self.config.refresh_rate -= TICK;
                }
                self.save_config();
            }

            Message::ConfigChanged(config) => {
                println!("Message::ConfigChanged {config:?}");
                self.config = config;
                self.tick_timer = self.config.refresh_rate;
                self.svgstat_cpu.set_colors(&self.config.cpu_colors);
                self.svgstat_mem.set_colors(&self.config.mem_colors);
            }
        }
        //println!("=====/update=====");
        Command::none()
    }
}

use cosmic::Application;
impl Minimon {
    fn make_icon_handle(svgstat: &SvgStat) -> cosmic::widget::icon::Handle {
        cosmic::widget::icon::from_svg_bytes(svgstat.to_string().as_bytes().to_owned())
    }

    fn save_config(&self) {
        if let Ok(helper) = cosmic::cosmic_config::Config::new(Self::APP_ID, MinimonConfig::VERSION)
        {
            if let Err(err) = self.config.write_entry(&helper) {
                println!("Error writing config {err}");
            }
        }
    }

    fn refresh_stats(&mut self) {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();

        self.cpu_load = self
            .system
            .cpus()
            .iter()
            .map(|p| f64::from(p.cpu_usage()))
            .sum::<f64>()
            / self.system.cpus().len() as f64;

        self.mem_usage = self.system.used_memory() as f64 / 1_073_741_824.0;

        self.svgstat_cpu.set_variable(self.cpu_load);
        self.svgstat_mem.set_variable(self.mem_usage);
    }
}
