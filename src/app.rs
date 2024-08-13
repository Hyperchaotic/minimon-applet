use cosmic::cosmic_config::CosmicConfigEntry;
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

use crate::svgstat::SvgStat;
use crate::{config::MinimonConfig, fl};

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
    config: MinimonConfig,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    Tick,
    PopupClosed(Id),
    ToggleTextMode(bool),
    ToggleCpu(bool),
    ToggleMemory(bool),
    ConfigChanged(MinimonConfig),
}

impl cosmic::Application for Minimon {
//    type Executor = cosmic::SingleThreadExecutor;
        type Executor = cosmic::executor::Default;

    type Flags = ();

    type Message = Message;

    const APP_ID: &'static str = "com.github.hyperchaotic.Minimon";

    fn init(core: Core, _flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let mut system = System::new();
        system.refresh_memory();
        let mem_physical = system.total_memory(); // To GB

        let app = Minimon {
            core,
            system,
            cpu_load: 0.0,
            mem_usage: 0.0,
            svgstat_cpu: super::svgstat::SvgStat::new("red", 100),
            svgstat_mem: super::svgstat::SvgStat::new("purple", mem_physical / 1_073_741_824),
            popup: None,
            config: MinimonConfig::default(),
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
                let duration = time::Duration::from_millis(1000);
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
                        .icon_button_from_handle(self.make_icon_handle(&self.svgstat_cpu))
                        .on_press(Message::TogglePopup)
                        .style(cosmic::theme::Button::AppletIcon),
                );
                elements.push(cpu_widget);
            }

            if self.config.enable_mem {
                let mem_widget = Element::from(
                    self.core
                        .applet
                        .icon_button_from_handle(self.make_icon_handle(&self.svgstat_mem))
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
        let mut cpu_elements = Vec::new();
        cpu_elements.push(Element::from(
            self.core
                .applet
                .icon_button_from_handle(self.make_icon_handle(&self.svgstat_cpu))
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
                .icon_button_from_handle(self.make_icon_handle(&self.svgstat_mem))
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
            .add(Element::from(mem_row));

        self.core.applet.popup_container(content_list).into()
    }

    /// Application messages are handled here. The application state can be modified based on
    /// what message was received. Commands may be returned for asynchronous execution on a
    /// background thread managed by the application's executor.
    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
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
            Message::Tick => {
                self.refresh_stats();
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
            Message::ConfigChanged(config) => {
                println!("Message::ConfigChanged {config:?}");
                self.config = config;
            }
        }
        Command::none()
    }
}

use cosmic::Application;
impl Minimon {
    
    fn make_icon_handle(&self, svgstat: &SvgStat) -> cosmic::widget::icon::Handle {
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
