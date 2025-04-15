use cosmic::applet::cosmic_panel_config::PanelSize;
use cosmic::applet::{PanelType, Size};
use cosmic::cosmic_config::CosmicConfigEntry;
use cosmic::cosmic_theme::palette::bool_mask::BoolMask;
use cosmic::cosmic_theme::palette::{FromColor, WithAlpha};
use std::time;

use cosmic::app::{Core, Task};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::window::Id;
use cosmic::iced::Limits;
use cosmic::iced::{self, Subscription};
use cosmic::widget::{container, list, settings, spin_button, text};
use cosmic::{widget, widget::autosize};
use cosmic::{Apply, Element};

use once_cell::sync::Lazy;
use std::sync::atomic::{self, AtomicI64};
use std::sync::Arc;

use cosmic::{
    applet::cosmic_panel_config::PanelAnchor,
    iced::{widget::column, widget::row, Alignment},
    iced_widget::{Column, Row},
};

use crate::colorpicker::{ColorPicker, DemoGraph};
use crate::config::{
    ColorVariant, DeviceKind, DisksVariant, GraphColors, GraphKind, NetworkVariant,
};
use crate::sensors::cpu::Cpu;
use crate::sensors::disks::{self, Disks};
use crate::sensors::memory::Memory;
use crate::sensors::network::{self, Network};
use crate::sensors::Sensor;
use crate::{config::MinimonConfig, fl};
use cosmic::widget::Id as WId;

static AUTOSIZE_MAIN_ID: Lazy<WId> = Lazy::new(|| WId::new("autosize-main"));

const TICK: i64 = 250;

const ICON: &str = "com.github.hyperchaotic.cosmic-applet-minimon";

use lazy_static::lazy_static;

lazy_static! {
    /// Translated color choices.
    ///
    /// The string values are intentionally leaked (`.leak()`) to convert them
    /// into `'static str` because:
    /// - These strings are only initialized once at program startup.
    /// - They are never deallocated since they are used globally.
    static ref SETTINGS_GENERAL: &'static str = fl!("settings-subpage-general").leak();
    static ref SETTINGS_BACK: &'static str = fl!("settings-subpage-back").leak();
    static ref SETTINGS_CPU: &'static str = fl!("cpu-title").leak();
    static ref SETTINGS_MEMORY: &'static str = fl!("memory-title").leak();
    static ref SETTINGS_NETWORK: &'static str = fl!("net-title").leak();
    static ref SETTINGS_DISKS: &'static str = fl!("disks-title").leak();

    // The UI require static lifetime of dropdown items
    static ref SYSMON_LIST: Vec<(String, String)> = Minimon::get_sysmon_list();

    static ref SYSMON_NAMES: Vec<&'static str> = SYSMON_LIST
    .iter()
    .map(|(_, name)| (name.as_str()))
    .collect();

}


macro_rules! network_select {
    ($self:ident, $variant:expr) => {
        match $variant {
            NetworkVariant::Combined | NetworkVariant::Download => {
                (&mut $self.network1, &mut $self.config.network1)
            }
            _ => (&mut $self.network2, &mut $self.config.network2),
        }
    };
}

macro_rules! disks_select {
    ($self:ident, $variant:expr) => {
        match $variant {
            DisksVariant::Combined | DisksVariant::Write => {
                (&mut $self.disks1, &mut $self.config.disks1)
            }
            _ => (&mut $self.disks2, &mut $self.config.disks2),
        }
    };
}

#[derive(Debug, Clone, Copy)]
enum SettingsVariant {
    General,
    Cpu,
    Memory,
    Network,
    Disks,
}

pub struct Minimon {
    /// Application state which is managed by the COSMIC runtime.
    core: Core,
    /// The svg image to draw for the CPU load
    cpu: Cpu,
    /// The svg image to draw for the Memory load
    memory: Memory,

    /// The network monitor, if combined we only use the first one
    network1: Network,
    network2: Network,

    /// The network monitor
    disks1: Disks,
    disks2: Disks,

    /// The popup id.
    popup: Option<Id>,

    /// Current settings sub page
    settings_page: Option<SettingsVariant>,

    /// The color picker dialog
    colorpicker: ColorPicker,

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

    ToggleNetCombined(bool),
    ToggleNetChart(NetworkVariant, bool),
    ToggleNetLabel(NetworkVariant, bool),
    ToggleAdaptiveNet(NetworkVariant, bool),
    NetworkSelectUnit(NetworkVariant, usize),
    TextInputBandwidthChanged(NetworkVariant, String),

    ToggleDisksCombined(bool),
    ToggleDisksChart(DisksVariant, bool),
    ToggleDisksLabel(DisksVariant, bool),

    SelectGraphType(DeviceKind),
    Tick,
    PopupClosed(Id),

    ToggleCpuChart(bool),
    ToggleCpuLabel(bool),
    ToggleMemoryChart(bool),
    ToggleMemoryLabel(bool),
    ConfigChanged(MinimonConfig),
    LaunchSystemMonitor(),
    RefreshRateChanged(f64),
    LabelSizeChanged(u16),
    ToggleMonospaceLabels(bool),

    SettingsBack,
    SettingsGeneral,
    SettingsCpu,
    SettingsMemory,
    SettingsNetwork,
    SettingsDisks,

    SysmonSelect(usize),
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
            network1: Network::new(NetworkVariant::Combined),
            network2: Network::new(NetworkVariant::Upload),
            disks1: Disks::new(DisksVariant::Combined),
            disks2: Disks::new(DisksVariant::Read),
            popup: None,
            settings_page: None,
            colorpicker: ColorPicker::new(),
            config: MinimonConfig::default(),
            tick_timer: TICK,
            tick: Arc::new(AtomicI64::new(TICK)),
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
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();
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

        // If nothing is showing, use symbolic icon
        if !self.config.cpu.chart
            && !self.config.cpu.label
            && !self.config.memory.chart
            && !self.config.memory.label
            && !self.config.network1.chart
            && !self.config.network1.label
            && !self.config.network2.chart
            && !self.config.network2.label
            && !self.config.disks1.chart
            && !self.config.disks1.label
            && !self.config.disks2.chart
            && !self.config.disks2.label
        {
            return self
                .core
                .applet
                .icon_button(ICON)
                .on_press(Message::TogglePopup)
                .into();
        }

        // Build the full list of panel elements
        let mut elements: Vec<Element<Message>> = Vec::new();
        elements.extend(self.cpu_panel_ui(horizontal));
        elements.extend(self.memory_panel_ui(horizontal));
        elements.extend(self.network_panel_ui(horizontal));
        elements.extend(self.disks_panel_ui(horizontal));

        // Layout horizontally or vertically
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
                .popup_container(self.colorpicker.view_colorpicker())
                .limits(limits)
                .into()
        } else {
            let theme = cosmic::theme::active();

            let padding = if self.core.is_condensed() {
                theme.cosmic().space_s()
            } else {
                theme.cosmic().space_l()
            };

            let mut content = Column::new();

            if let Some(variant) = self.settings_page {
                match variant {
                    SettingsVariant::Cpu => {
                        content = content.push(Minimon::sub_page_header(
                            &SETTINGS_CPU,
                            &SETTINGS_BACK,
                            Message::SettingsBack,
                        ));
                        content = content.push(self.cpu.settings_ui(&self.config));
                    }
                    SettingsVariant::Memory => {
                        content = content.push(Minimon::sub_page_header(
                            &SETTINGS_MEMORY,
                            &SETTINGS_BACK,
                            Message::SettingsBack,
                        ));
                        content = content.push(self.memory.settings_ui(&self.config));
                    }
                    SettingsVariant::Network => {
                        content = content.push(Minimon::sub_page_header(
                            &SETTINGS_NETWORK,
                            &SETTINGS_BACK,
                            Message::SettingsBack,
                        ));
                        content = content.push(settings::item(
                            fl!("enable-net-combined"),
                            widget::toggler(
                                self.config.network1.variant == NetworkVariant::Combined,
                            )
                            .on_toggle(Message::ToggleNetCombined),
                        ));
                        content = content.push(self.network1.settings_ui(&self.config));
                        if self.config.network1.variant == NetworkVariant::Download {
                            content = content.push(self.network2.settings_ui(&self.config));
                        }
                    }
                    SettingsVariant::Disks => {
                        content = content.push(Minimon::sub_page_header(
                            &SETTINGS_DISKS,
                            &SETTINGS_BACK,
                            Message::SettingsBack,
                        ));
                        content = content.push(settings::item(
                            fl!("enable-disks-combined"),
                            widget::toggler(self.config.disks1.variant == DisksVariant::Combined)
                                .on_toggle(move |t| Message::ToggleDisksCombined(t)),
                        ));
                        content = content.push(self.disks1.settings_ui(&self.config));
                        if self.config.disks1.variant == DisksVariant::Write {
                            content = content.push(self.disks2.settings_ui(&self.config));
                        }
                    }
                    SettingsVariant::General => {
                        content = content.push(Minimon::sub_page_header(
                            &SETTINGS_GENERAL,
                            &SETTINGS_BACK,
                            Message::SettingsBack,
                        ));
                        content = content.push(self.general_settings_ui());
                    }
                }
            } else {
                let list = &*SYSMON_NAMES;
                let safe_index = if self.config.sysmon < list.len() {
                    self.config.sysmon
                } else {
                    0
                };
                let name = list[safe_index];

                content = content.push(Element::from(row!(
                    widget::horizontal_space(),
                    widget::button::standard(name)
                        .on_press(Message::LaunchSystemMonitor())
                        .trailing_icon(widget::button::link::icon()),
                    widget::horizontal_space()
                )));

                let cpu = widget::text::body(self.cpu.to_string());
                let memory = widget::text::body(self.memory.to_string());

                let sample_rate_ms = self.config.refresh_rate;
                let network = widget::text::body(format!(
                    "↓ {} ↑ {}",
                    &self
                        .network1
                        .download_label(sample_rate_ms, network::UnitVariant::Long),
                    &self
                        .network1
                        .upload_label(sample_rate_ms, network::UnitVariant::Long)
                ));

                let disks = widget::text::body(format!(
                    "W {} R {}",
                    &self
                        .disks1
                        .write_label(sample_rate_ms, disks::UnitVariant::Long),
                    &self
                        .disks1
                        .read_label(sample_rate_ms, disks::UnitVariant::Long)
                ));

                let sensor_settings = list::ListColumn::new()
                    .add(Minimon::go_next_with_item(
                        "General settings",
                        text::body(""),
                        Message::SettingsGeneral,
                    ))
                    .add(Minimon::go_next_with_item("CPU", cpu, Message::SettingsCpu))
                    .add(Minimon::go_next_with_item(
                        "Memory",
                        memory,
                        Message::SettingsMemory,
                    ))
                    .add(Minimon::go_next_with_item(
                        "Network",
                        network,
                        Message::SettingsNetwork,
                    ))
                    .add(Minimon::go_next_with_item(
                        "Disks",
                        disks,
                        Message::SettingsDisks,
                    ))
                    .padding(0);

                content = content.push(sensor_settings);
            }

            content = content.padding(padding).spacing(padding);
            //let content = column!(sensor_settings);
            let limits = Limits::NONE
                .max_width(420.0)
                .min_width(360.0)
                .min_height(200.0)
                .max_height(750.0);

            self.core
                .applet
                .popup_container(content)
                .limits(limits)
                .into()
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
                            .activate(kind, self.cpu.demo_graph(self.config.cpu.colors));
                    }
                    DeviceKind::Memory(_) => {
                        self.colorpicker
                            .activate(kind, self.memory.demo_graph(self.config.memory.colors));
                    }
                    DeviceKind::Network(variant) => {
                        let (network, config) = network_select!(self, variant);
                        self.colorpicker
                            .activate(kind, network.demo_graph(config.colors));
                    }
                    DeviceKind::Disks(variant) => {
                        let (disks, _) = disks_select!(self, variant);
                        self.colorpicker
                            .activate(kind, disks.demo_graph(disks.colors()));
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

            Message::ToggleNetCombined(toggle) => {
                if toggle.is_true() {
                    self.network1.kind = NetworkVariant::Combined;
                    self.config.network1.variant = NetworkVariant::Combined;
                } else {
                    self.network1.kind = NetworkVariant::Download;
                    self.config.network1.variant = NetworkVariant::Download;
                }
                self.network2.kind = NetworkVariant::Upload;
                self.config.network2.variant = NetworkVariant::Upload;
                self.save_config();
            }

            Message::ToggleDisksCombined(toggle) => {
                if toggle.is_true() {
                    self.disks1.kind = DisksVariant::Combined;
                    self.config.disks1.variant = DisksVariant::Combined;
                } else {
                    self.disks1.kind = DisksVariant::Write;
                    self.config.disks1.variant = DisksVariant::Write;
                }
                self.disks2.kind = DisksVariant::Read;
                self.config.disks2.variant = DisksVariant::Read;
                self.save_config();
            }

            Message::ToggleDisksChart(variant, toggled) => {
                let (_, config) = disks_select!(self, variant);
                config.chart = toggled;
                self.save_config();
            }

            Message::ToggleDisksLabel(variant, toggled) => {
                let (_, config) = disks_select!(self, variant);
                config.label = toggled;
                self.save_config();
            }

            Message::ToggleAdaptiveNet(variant, toggle) => {
                let (network, config) = network_select!(self, variant);
                config.adaptive = toggle;
                if toggle {
                    network.set_max_y(None);
                }
                self.save_config();
            }

            Message::NetworkSelectUnit(variant, unit) => {
                let (_, config) = network_select!(self, variant);
                config.unit = Some(unit);
                self.set_network_max_y(variant);
                self.save_config();
            }

            Message::SelectGraphType(dev) => {
                match dev {
                    DeviceKind::Cpu(kind) => {
                        self.cpu.set_graph_kind(kind);
                        self.config.cpu.kind = kind;
                    }
                    DeviceKind::Memory(kind) => {
                        self.memory.set_graph_kind(kind);
                        self.config.memory.kind = kind;
                    }
                    _ => (), // Disks and Network don't have graph selection
                }
                self.save_config();
            }

            Message::TextInputBandwidthChanged(variant, string) => {
                let value = if string.is_empty() {
                    Some(0)
                } else {
                    string.parse::<u64>().ok()
                };

                if let Some(val) = value {
                    let (_, config) = network_select!(self, variant);
                    config.bandwidth = val;
                }

                self.set_network_max_y(variant);
                self.save_config();
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
                self.config.cpu.chart = toggled;
                self.save_config();
            }

            Message::ToggleMemoryChart(toggled) => {
                self.config.memory.chart = toggled;
                self.save_config();
            }

            Message::ToggleNetChart(variant, toggled) => {
                let (_, config) = network_select!(self, variant);
                config.chart = toggled;
                self.save_config();
            }

            Message::ToggleCpuLabel(toggled) => {
                self.config.cpu.label = toggled;
                self.save_config();
            }

            Message::ToggleMemoryLabel(toggled) => {
                self.config.memory.label = toggled;
                self.save_config();
            }

            Message::ToggleNetLabel(variant, toggled) => {
                let (_, config) = network_select!(self, variant);
                config.label = toggled;
                self.save_config();
            }

            Message::ConfigChanged(config) => {
                self.config = config;
                self.tick_timer = self.config.refresh_rate as i64;
                self.cpu.set_colors(self.config.cpu.colors);
                self.cpu.set_graph_kind(self.config.cpu.kind);
                self.memory.set_colors(self.config.memory.colors);
                self.memory.set_graph_kind(self.config.memory.kind);
                self.network1.set_colors(self.config.network1.colors);
                self.network2.set_colors(self.config.network2.colors);
                self.network1.kind = self.config.network1.variant;
                self.network2.kind = self.config.network2.variant;
                self.disks1.kind = self.config.disks1.variant;
                self.disks2.kind = self.config.disks2.variant;
                self.set_network_max_y(NetworkVariant::Download);
                self.set_network_max_y(NetworkVariant::Upload);
                self.set_tick();
                print!(
                    "disk1 - {:?}. disk2 - {:?}.",
                    self.config.disks1.variant, self.config.disks2.variant
                )
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
                Minimon::spawn_sysmon_by_index(self.config.sysmon);
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

            Message::ToggleMonospaceLabels(toggle) => {
                self.config.monospace_labels = toggle;
                self.save_config();
            }
            Message::SettingsBack => self.settings_page = None,
            Message::SettingsGeneral => self.settings_page = Some(SettingsVariant::General),
            Message::SettingsCpu => self.settings_page = Some(SettingsVariant::Cpu),
            Message::SettingsMemory => self.settings_page = Some(SettingsVariant::Memory),
            Message::SettingsNetwork => self.settings_page = Some(SettingsVariant::Network),
            Message::SettingsDisks => self.settings_page = Some(SettingsVariant::Disks),
            Message::SysmonSelect(sysmon) => {
                self.config.sysmon = sysmon;
                self.save_config();
            }
        }
        Task::none()
    }
}

impl Minimon {
    pub fn sub_page_header<'a, Message: 'static + Clone>(
        sub_page: &'a str,
        parent_page: &'a str,
        on_press: Message,
    ) -> Element<'a, Message> {
        let previous_button = widget::button::icon(widget::icon::from_name("go-previous-symbolic"))
            .extra_small()
            .padding(0)
            .label(parent_page)
            .spacing(4)
            .class(widget::button::ButtonClass::Link)
            .on_press(on_press);

        let sub_page_header = widget::row::with_capacity(2).push(text::title3(sub_page));

        widget::column::with_capacity(2)
            .push(previous_button)
            .push(sub_page_header)
            .spacing(6)
            .width(iced::Length::Shrink)
            .into()
    }

    pub fn go_next_with_item<'a, Msg: Clone + 'static>(
        description: &'a str,
        item: impl Into<cosmic::Element<'a, Msg>>,
        msg_opt: impl Into<Option<Msg>> + Clone,
    ) -> cosmic::Element<'a, Msg> {
        settings::item_row(vec![
            widget::text::body(description)
                .wrapping(iced::core::text::Wrapping::Word)
                .into(),
            widget::horizontal_space().into(),
            widget::row::with_capacity(2)
                .push(item)
                .push(widget::icon::from_name("go-next-symbolic").size(16).icon())
                .align_y(Alignment::Center)
                .spacing(cosmic::theme::spacing().space_s)
                .into(),
        ])
        .apply(widget::container)
        .class(cosmic::theme::Container::List)
        .apply(widget::button::custom)
        .padding(0)
        .class(cosmic::theme::Button::Transparent)
        .on_press_maybe(msg_opt.into())
        .into()
    }

    fn general_settings_ui(&self) -> Element<crate::app::Message> {
        let refresh_rate = self.config.refresh_rate as f64 / 1000.0;

        let refresh_row = settings::item(
            fl!("refresh-rate"),
            spin_button(
                format!("{:.2}", refresh_rate),
                refresh_rate,
                0.250,
                0.250,
                5.00,
                Message::RefreshRateChanged,
            ),
        );

        let label_row = settings::item(
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

        let mono_row = settings::item(
            fl!("settings-monospace_font"),
            row!(widget::checkbox("", self.config.monospace_labels)
                .on_toggle(Message::ToggleMonospaceLabels)),
        );

        let sysmon_row = settings::item(
            fl!("choose-sysmon"),
            row!(widget::dropdown(
                &SYSMON_NAMES,
                Some(self.config.sysmon),
                Message::SysmonSelect
            )
            .width(220)),
        );

        column!(refresh_row, label_row, mono_row, sysmon_row)
            .spacing(10)
            .into()
    }

    fn cpu_panel_ui(&self, horizontal: bool) -> Vec<Element<crate::app::Message>> {
        let mut elements: Vec<Element<Message>> = Vec::new();
        // If we are below 10% and horizontal layout we can show another decimal
        let formated_cpu = if self.cpu.latest_sample() < 10.0 && horizontal {
            format!("{:.2}%", self.cpu.latest_sample())
        } else {
            format!("{:.1}%", self.cpu.latest_sample())
        };

        if self.config.cpu.label {
            elements.push(self.figure_label(formated_cpu).into());
        }

        if self.config.cpu.chart {
            let content = self
                .core
                .applet
                .icon_button_from_handle(Minimon::make_icon_handle(&self.cpu));

            elements.push(content.into());
        }

        elements
    }

    fn memory_panel_ui(&self, _horizontal: bool) -> Vec<Element<crate::app::Message>> {
        let mut elements: Vec<Element<Message>> = Vec::new();

        let formated_mem = format!("{:.1} GB", self.memory.latest_sample());
        if self.config.memory.label {
            elements.push(self.figure_label(formated_mem).into());
        }

        if self.config.memory.chart {
            let content = self
                .core
                .applet
                .icon_button_from_handle(Minimon::make_icon_handle(&self.memory));

            elements.push(content.into());
        }

        elements
    }

    fn network_panel_ui(&self, horizontal: bool) -> Vec<Element<crate::app::Message>> {
        let nw_combined = self.config.network1.variant == NetworkVariant::Combined;
        let sample_rate_ms = self.config.refresh_rate;
        let mut elements: Vec<Element<Message>> = Vec::new();

        if self.config.network1.label {
            let mut network_labels: Vec<Element<Message>> = Vec::new();

            // DL
            let dl_label = match horizontal {
                true => self.figure_label(format!(
                    "↓ {}",
                    &self
                        .network1
                        .download_label(sample_rate_ms, network::UnitVariant::Long)
                )),
                false => self.figure_label(
                    self.network1
                        .download_label(sample_rate_ms, network::UnitVariant::Short),
                ),
            };
            if nw_combined {
                network_labels.push(widget::vertical_space().into());
            }
            network_labels.push(dl_label.into());

            if nw_combined {
                // UL
                let ul_label = match horizontal {
                    true => self.figure_label(format!(
                        "↑ {}",
                        &self
                            .network1
                            .upload_label(sample_rate_ms, network::UnitVariant::Long)
                    )),
                    false => self.figure_label(
                        self.network1
                            .upload_label(sample_rate_ms, network::UnitVariant::Short),
                    ),
                };
                network_labels.push(ul_label.into());
                network_labels.push(widget::vertical_space().into());
            }
            elements.push(Column::from_vec(network_labels).into());
        }

        if self.config.network1.chart {
            let svg = self.network1.graph();
            let handle = cosmic::widget::icon::from_svg_bytes(svg.into_bytes());
            let content = self.core.applet.icon_button_from_handle(handle);

            elements.push(content.into());
        }

        if self.config.network2.label && !nw_combined {
            let mut network_labels: Vec<Element<Message>> = Vec::new();

            let ul_label = match horizontal {
                true => self.figure_label(format!(
                    "↑ {}",
                    &self
                        .network2
                        .upload_label(sample_rate_ms, network::UnitVariant::Long)
                )),
                false => self.figure_label(
                    self.network2
                        .upload_label(sample_rate_ms, network::UnitVariant::Short),
                ),
            };
            network_labels.push(ul_label.into());

            elements.push(Column::from_vec(network_labels).into());
        }

        if self.config.network2.chart && !nw_combined {
            let svg = self.network2.graph();
            let handle = cosmic::widget::icon::from_svg_bytes(svg.into_bytes());
            let content = self.core.applet.icon_button_from_handle(handle);

            elements.push(content.into());
        }

        elements
    }

    fn disks_panel_ui(&self, horizontal: bool) -> Vec<Element<crate::app::Message>> {
        let disks_combined = self.config.disks1.variant == DisksVariant::Combined;
        let sample_rate_ms = self.config.refresh_rate;
        let mut elements: Vec<Element<Message>> = Vec::new();

        if self.config.disks1.label {
            let mut disks_labels: Vec<Element<Message>> = Vec::new();

            // Write
            let wr_label = match horizontal {
                true => self.figure_label(format!(
                    "W {}",
                    &self
                        .disks1
                        .write_label(sample_rate_ms, disks::UnitVariant::Long)
                )),
                false => self.figure_label(
                    self.disks1
                        .write_label(sample_rate_ms, disks::UnitVariant::Short),
                ),
            };
            if disks_combined {
                disks_labels.push(widget::vertical_space().into());
            }
            disks_labels.push(wr_label.into());

            if disks_combined {
                // Read
                let rd_label = match horizontal {
                    true => self.figure_label(format!(
                        "R {}",
                        &self
                            .disks1
                            .read_label(sample_rate_ms, disks::UnitVariant::Long)
                    )),
                    false => self.figure_label(
                        self.disks1
                            .read_label(sample_rate_ms, disks::UnitVariant::Short),
                    ),
                };
                disks_labels.push(rd_label.into());
                disks_labels.push(widget::vertical_space().into());
            }
            elements.push(Column::from_vec(disks_labels).into());
        }

        if self.config.disks1.chart {
            let svg = self.disks1.graph();
            let handle = cosmic::widget::icon::from_svg_bytes(svg.into_bytes());
            let content = self.core.applet.icon_button_from_handle(handle);

            elements.push(content.into());
        }

        if self.config.disks2.label && !disks_combined {
            let mut disks_labels: Vec<Element<Message>> = Vec::new();

            let rd_label = match horizontal {
                true => self.figure_label(format!(
                    "R {}",
                    &self
                        .disks2
                        .read_label(sample_rate_ms, disks::UnitVariant::Long)
                )),
                false => self.figure_label(
                    self.disks2
                        .read_label(sample_rate_ms, disks::UnitVariant::Short),
                ),
            };
            disks_labels.push(rd_label.into());

            elements.push(Column::from_vec(disks_labels).into());
        }

        if self.config.disks2.chart && !disks_combined {
            let svg = self.disks2.graph();
            let handle = cosmic::widget::icon::from_svg_bytes(svg.into_bytes());
            let content = self.core.applet.icon_button_from_handle(handle);

            elements.push(content.into());
        }

        elements
    }

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
                self.config.cpu.colors = colors;
                self.cpu.set_colors(colors);
            }
            DeviceKind::Memory(_) => {
                self.config.memory.colors = colors;
                self.memory.set_colors(colors);
            }
            DeviceKind::Network(variant) => {
                let (network, config) = network_select!(self, variant);
                config.colors = colors;
                network.set_colors(colors);
            }
            DeviceKind::Disks(variant) => {
                let (disks, config) = disks_select!(self, variant);
                config.colors = colors;
                disks.set_colors(colors);
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

    fn set_network_max_y(&mut self, variant: NetworkVariant) {
        let (network, config) = network_select!(self, variant);
        if config.adaptive {
            network.set_max_y(None);
        } else {
            let unit = config.unit.unwrap_or(1).min(4); // ensure safe index
            let multiplier = [1, 1_000, 1_000_000, 1_000_000_000, 1_000_000_000_000];
            let sec_per_tic = self.config.refresh_rate as f64 / 1000.0;
            let new_y = (config.bandwidth * multiplier[unit]) as f64 * sec_per_tic;
            network.set_max_y(Some(new_y.round() as u64));
        }
    }

    fn refresh_stats(&mut self) {
        self.cpu.update();
        self.memory.update();
        self.network1.update();
        self.network2.update();
        self.disks1.update();
        self.disks2.update();
    }

    fn spawn_sysmon_by_index(index: usize) {
        let list = &*SYSMON_LIST;
        let safe_index = if index < list.len() { index } else { 0 };
        let (command, _) = &list[safe_index];

        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .spawn();
    }

    fn get_sysmon_list() -> Vec<(String, String)> {
        let mut found: Vec<(String, String)> = Vec::new();

        // Native system monitors
        let native_monitors = vec![
            ("observatory", "COSMIC Observatory"),
            ("gnome-system-monitor", "GNOME System Monitor"),
            ("xfce4-taskmanager", "XFCE Task Manager"),
            ("plasma-systemmonitor", "Plasma System Monitor"),
            ("mate-system-monitor", "MATE System Monitor"),
            ("lxqt-taskmanager", "LXQt Task Manager"),
        ];

        for (command, name) in native_monitors {
            if let Ok(output) = std::process::Command::new("which").arg(command).output() {
                if output.status.success() && !output.stdout.is_empty() {
                    found.push((command.to_string(), name.to_string()));
                }
            }
        }

        // Flatpak-based system monitors
        let flatpak_monitors = vec![
            ("net.nokyan.Resources", "Resources"),
            ("com.github.gi_r.Usage", "Usage"),
            ("org.gnome.SystemMonitor", "GNOME System Monitor (Flatpak)"),
            ("io.missioncenter.MissionCenter", "Mission Center"),
        ];

        for (flatpak_id, name) in flatpak_monitors {
            if std::process::Command::new("flatpak")
                .arg("info")
                .arg(flatpak_id)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                found.push((format!("flatpak run {}", flatpak_id), name.to_string()));
            }
        }

        found
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

        if self.config.monospace_labels {
            widget::text(text).size(size).font(cosmic::font::mono())
        } else {
            widget::text(text).size(size)
        }
    }
}
