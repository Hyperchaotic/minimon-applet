use cosmic::applet::cosmic_panel_config::PanelSize;
use cosmic::applet::{PanelType, Size};
use cosmic::cosmic_config::CosmicConfigEntry;
use cosmic::cosmic_theme::palette::bool_mask::BoolMask;
use cosmic::cosmic_theme::palette::{FromColor, WithAlpha};
use std::collections::BTreeMap;
use std::fmt::Write;
use std::{fs, time};

use cosmic::app::{Core, Task};
use cosmic::iced::Limits;
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::window::Id;
use cosmic::iced::{self, Subscription};
use cosmic::widget::{container, list, settings, spin_button, text};
use cosmic::{Apply, Element};
use cosmic::{widget, widget::autosize};

use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::atomic::{self, AtomicI64};

use cosmic::{
    applet::cosmic_panel_config::PanelAnchor,
    iced::{Alignment, widget::column, widget::row},
    iced_widget::{Column, Row},
};

use zbus::blocking::Connection;
use zvariant::OwnedObjectPath;

use log::{debug, error, info};

use crate::colorpicker::{ColorPicker, DemoGraph};
use crate::config::{
    ColorVariant, DeviceKind, DisksVariant, GpuConfig, GraphColors, GraphKind, NetworkVariant,
};
use crate::sensors::cpu::Cpu;
use crate::sensors::cputemp::CpuTemp;
use crate::sensors::disks::{self, Disks};
use crate::sensors::gpus::{Gpu, list_gpus};
use crate::sensors::memory::Memory;
use crate::sensors::network::{self, Network};
use crate::sensors::{Sensor, TempUnit};
use crate::{config::MinimonConfig, fl};
use cosmic::widget::Id as WId;

static AUTOSIZE_MAIN_ID: LazyLock<WId> = std::sync::LazyLock::new(|| WId::new("autosize-main"));

const TICK: i64 = 250;

const ICON: &str = "io.github.cosmic-utils.cosmic-applet-minimon";
const TEMP_ICON: &str = "io.github.cosmic-utils.cosmic-applet-minimon-temperature";
const RAM_ICON: &str = "io.github.cosmic-utils.cosmic-applet-minimon-ram";
const GPU_ICON: &str = "io.github.cosmic-utils.cosmic-applet-minimon-gpu";
const NETWORK_ICON: &str = "io.github.cosmic-utils.cosmic-applet-minimon-network";
const DISK_ICON: &str = "io.github.cosmic-utils.cosmic-applet-minimon-harddisk";

pub static SETTINGS_CPU_CHOICE: LazyLock<&'static str> =
    LazyLock::new(|| fl!("settings-cpu").leak());
pub static SETTINGS_CPU_TEMP_CHOICE: LazyLock<&'static str> =
    LazyLock::new(|| fl!("settings-cpu-temperature").leak());
pub static SETTINGS_MEMORY_CHOICE: LazyLock<&'static str> =
    LazyLock::new(|| fl!("settings-memory").leak());
pub static SETTINGS_NETWORK_CHOICE: LazyLock<&'static str> =
    LazyLock::new(|| fl!("settings-network").leak());
pub static SETTINGS_DISKS_CHOICE: LazyLock<&'static str> =
    LazyLock::new(|| fl!("settings-disks").leak());
pub static SETTINGS_GPU_CHOICE: LazyLock<&'static str> =
    LazyLock::new(|| fl!("settings-gpu").leak());

pub static SETTINGS_GENERAL_HEADING: LazyLock<&'static str> =
    LazyLock::new(|| fl!("settings-subpage-general").leak());
pub static SETTINGS_BACK: LazyLock<&'static str> =
    LazyLock::new(|| fl!("settings-subpage-back").leak());
pub static SETTINGS_CPU_HEADING: LazyLock<&'static str> = LazyLock::new(|| fl!("cpu-title").leak());
pub static SETTINGS_CPU_TEMP_HEADING: LazyLock<&'static str> =
    LazyLock::new(|| fl!("cpu-temperature-title").leak());
pub static SETTINGS_MEMORY_HEADING: LazyLock<&'static str> =
    LazyLock::new(|| fl!("memory-title").leak());
pub static SETTINGS_NETWORK_HEADING: LazyLock<&'static str> =
    LazyLock::new(|| fl!("net-title").leak());
pub static SETTINGS_DISKS_HEADING: LazyLock<&'static str> =
    LazyLock::new(|| fl!("disks-title").leak());
pub static SETTINGS_GPU_HEADING: LazyLock<&'static str> =
    LazyLock::new(|| fl!("gpu-title").leak());

// The UI requires static lifetime of dropdown items
pub static SYSMON_LIST: LazyLock<Vec<(String, String)>> =
    LazyLock::new(|| Minimon::get_sysmon_list());

pub static SYSMON_NAMES: LazyLock<Vec<&'static str>> =
    LazyLock::new(|| SYSMON_LIST.iter().map(|(_, name)| name.as_str()).collect());

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

macro_rules! settings_sub_page_heading {
    ($heading:ident) => {
        Minimon::sub_page_header(Some(&$heading), &SETTINGS_BACK, Message::Settings(None))
    };
}

#[derive(Debug, Clone, Copy)]
pub enum SettingsVariant {
    General,
    Cpu,
    CpuTemp,
    Memory,
    Network,
    Disks,
    Gpu,
}

pub struct Minimon {
    /// Application state which is managed by the COSMIC runtime.
    core: Core,
    /// The svg image to draw for the CPU load
    cpu: Cpu,
    /// The svg image to draw for the CPU load
    cputemp: CpuTemp,
    /// The svg image to draw for the Memory load
    memory: Memory,

    /// The network monitor, if combined we only use the first one
    network1: Network,
    network2: Network,

    /// The network monitor
    disks1: Disks,
    disks2: Disks,

    //GPUs
    gpus: BTreeMap<String, Gpu>,

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

    // On AC or battery?
    is_laptop: bool,
    on_ac: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,

    ColorPickerOpen(DeviceKind, GraphKind, Option<String>),
    ColorPickerClose(bool, Option<String>),
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

    SelectGraphType(DeviceKind, GraphKind),
    Tick,
    PopupClosed(Id),

    ToggleCpuChart(bool),
    ToggleCpuLabel(bool),
    ToggleCpuTempChart(bool),
    ToggleCpuTempLabel(bool),
    ToggleMemoryChart(bool),
    ToggleMemoryLabel(bool),
    ToggleMemoryPercentage(bool),
    ConfigChanged(Box<MinimonConfig>),
    LaunchSystemMonitor(),
    RefreshRateChanged(f64),
    LabelSizeChanged(u16),
    ToggleMonospaceLabels(bool),
    ToggleTightSpacing(bool),
    SelectCpuTempUnit(TempUnit),

    Settings(Option<SettingsVariant>),

    GpuToggleChart(String, DeviceKind, bool),
    GpuToggleLabel(String, DeviceKind, bool),
    GpuToggleStackLabels(String, bool),
    GpuSelectGraphType(String, DeviceKind, GraphKind),
    ToggleDisableOnBattery(String, bool),
    ToggleSymbols(bool),
    SysmonSelect(usize),
}

const APP_ID_DOCK: &str = "io.github.cosmic-utils.cosmic-applet-minimon-dock";
const APP_ID_PANEL: &str = "io.github.cosmic-utils.cosmic-applet-minimon-panel";
const APP_ID_OTHER: &str = "io.github.cosmic-utils.cosmic-applet-minimon-other";

impl cosmic::Application for Minimon {
    type Executor = cosmic::executor::Default;

    type Flags = ();

    type Message = Message;

    const APP_ID: &'static str = "io.github.cosmic-utils.cosmic-applet-minimon";

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        // Find GPUs
        let gpus: BTreeMap<String, Gpu> = list_gpus()
            .into_iter()
            .map(|gpu| {
                info!("Found GPU. Name: {}. UUID: {}", gpu.name(), gpu.id());
                (gpu.id().to_string(), gpu)
            })
            .collect();

        let is_laptop = Minimon::is_laptop();
        if is_laptop {
            info!("Is laptop");
        }
        let app = Minimon {
            core,
            cpu: Cpu::new(GraphKind::Ring),
            cputemp: CpuTemp::new(GraphKind::Ring),
            memory: Memory::new(GraphKind::Line),
            network1: Network::new(NetworkVariant::Combined, GraphKind::Line),
            network2: Network::new(NetworkVariant::Upload, GraphKind::Line),
            disks1: Disks::new(DisksVariant::Combined, GraphKind::Line),
            disks2: Disks::new(DisksVariant::Read, GraphKind::Line),
            gpus,
            popup: None,
            settings_page: None,
            colorpicker: ColorPicker::new(),
            config: MinimonConfig::default(),
            tick_timer: TICK,
            tick: Arc::new(AtomicI64::new(TICK)),
            is_laptop,
            on_ac: true,
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
        fn time_subscription(tick: &std::sync::Arc<AtomicI64>) -> Subscription<time::Instant> {
            let atomic = tick.clone();
            let val = atomic.load(atomic::Ordering::Relaxed);
            iced::time::every(time::Duration::from_millis(val as u64))
        }

        Subscription::batch(vec![
            time_subscription(&self.tick).map(|_| Message::Tick),
            self.core
                .watch_config(match self.core.applet.panel_type {
                    PanelType::Panel => APP_ID_PANEL,
                    PanelType::Dock => APP_ID_DOCK,
                    PanelType::Other(_) => APP_ID_OTHER,
                })
                .map(|u| Message::ConfigChanged(Box::new(u.config))),
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
            if b.width > 0.0 {
                limits = limits.max_width(b.width);
            }
            if b.height > 0.0 {
                limits = limits.max_height(b.height);
            }
        }

        let mut gpu_visible = false;
        for gpu in self.gpus.values() {
            if let Some(g) = self.config.gpus.get(&gpu.id()) {
                if g.gpu_label || g.gpu_chart || g.vram_chart || g.vram_label {
                    gpu_visible = true;
                    break;
                }
            }
        }

        // If nothing is showing, use symbolic icon
        if !gpu_visible
            && !self.config.cpu.is_visible()
            && !self.config.cputemp.is_visible()
            && !self.config.memory.is_visible()
            && !self.config.network1.is_visible()
            && !self.config.network2.is_visible()
            && !self.config.disks1.is_visible()
            && !self.config.disks2.is_visible()
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
        elements.extend(self.cpu_temp_panel_ui(horizontal));
        elements.extend(self.memory_panel_ui(horizontal));
        elements.extend(self.network_panel_ui(horizontal));
        elements.extend(self.disks_panel_ui(horizontal));
        for gpu in self.gpus.values() {
            elements.extend(self.gpu_panel_ui(gpu, horizontal));
        } 

        let spacing = if self.config.tight_spacing {
            0
        } else {
            cosmic.space_xxs()
        };

        // Layout horizontally or vertically
        let wrapper: Element<Message> = if horizontal {
            Row::from_vec(elements)
                .align_y(Alignment::Center)
                .spacing(spacing)
                .into()
        } else {
            Column::from_vec(elements)
                .align_x(Alignment::Center)
                .spacing(spacing)
                .into()
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
                        content = content.push(settings_sub_page_heading!(SETTINGS_CPU_HEADING));
                        content = content.push(self.cpu.settings_ui(&self.config));
                    }
                    SettingsVariant::CpuTemp => {
                        content =
                            content.push(settings_sub_page_heading!(SETTINGS_CPU_TEMP_HEADING));
                        content = content.push(self.cputemp.settings_ui(&self.config));
                    }
                    SettingsVariant::Memory => {
                        content = content.push(Minimon::sub_page_header(
                            Some(&SETTINGS_MEMORY_HEADING),
                            &SETTINGS_BACK,
                            Message::Settings(None),
                        ));
                        content = content.push(self.memory.settings_ui(&self.config));
                    }
                    SettingsVariant::Network => {
                        content =
                            content.push(settings_sub_page_heading!(SETTINGS_NETWORK_HEADING));
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
                        content = content.push(settings_sub_page_heading!(SETTINGS_DISKS_HEADING));
                        content = content.push(settings::item(
                            fl!("enable-disks-combined"),
                            widget::toggler(self.config.disks1.variant == DisksVariant::Combined)
                                .on_toggle(Message::ToggleDisksCombined),
                        ));
                        content = content.push(self.disks1.settings_ui(&self.config));
                        if self.config.disks1.variant == DisksVariant::Write {
                            content = content.push(self.disks2.settings_ui(&self.config));
                        }
                    }
                    SettingsVariant::Gpu => {
                        content = content.push(settings_sub_page_heading!(SETTINGS_GPU_HEADING));
                        for (id, gpu) in &self.gpus {
                            if let Some(config) = self.config.gpus.get(id) {
                                content = content.push(
                                    widget::row::with_capacity(2)
                                        .push(text::heading(gpu.name()))
                                        .spacing(cosmic::theme::spacing().space_m),
                                );
                                if self.is_laptop {
                                    let disable_row = settings::item(
                                        fl!("settings-disable-on-battery"),
                                        row!(widget::toggler(config.pause_on_battery).on_toggle(
                                            move |value| {
                                                Message::ToggleDisableOnBattery(id.clone(), value)
                                            }
                                        )),
                                    ).width(350);
                                    content = content.push(disable_row);
                                }
                                content = content.push(gpu.settings_ui(config));
                            } else {
                                error!(
                                    "SettingsVariant::Gpu: no config for selected GPU {}",
                                    gpu.id()
                                );
                            }
                        }
                    }
                    SettingsVariant::General => {
                        content =
                            content.push(settings_sub_page_heading!(SETTINGS_GENERAL_HEADING));
                        content = content.push(self.general_settings_ui());
                    }
                }
            } else {
                if !SYSMON_LIST.is_empty() {
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
                }

                let cpu = widget::text::body(self.cpu.to_string());
                let cputemp = widget::text::body(self.cputemp.to_string());
                let memory = widget::text::body(self.memory.to_string(false));

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

                let mut sensor_settings = list::ListColumn::new()
                    .add(Minimon::go_next_with_item(
                        &SETTINGS_GENERAL_HEADING,
                        text::body(""),
                        Message::Settings(Some(SettingsVariant::General)),
                    ))
                    .add(Minimon::go_next_with_item(
                        &SETTINGS_CPU_CHOICE,
                        cpu,
                        Message::Settings(Some(SettingsVariant::Cpu)),
                    ));

                if self.cputemp.is_found() {
                    sensor_settings = sensor_settings.add(Minimon::go_next_with_item(
                        &SETTINGS_CPU_TEMP_CHOICE,
                        cputemp,
                        Message::Settings(Some(SettingsVariant::CpuTemp)),
                    ));
                }

                sensor_settings = sensor_settings
                    .add(Minimon::go_next_with_item(
                        &SETTINGS_MEMORY_CHOICE,
                        memory,
                        Message::Settings(Some(SettingsVariant::Memory)),
                    ))
                    .add(Minimon::go_next_with_item(
                        &SETTINGS_NETWORK_CHOICE,
                        network,
                        Message::Settings(Some(SettingsVariant::Network)),
                    ))
                    .add(Minimon::go_next_with_item(
                        &SETTINGS_DISKS_CHOICE,
                        disks,
                        Message::Settings(Some(SettingsVariant::Disks)),
                    ))
                    .padding(0);

                if self.has_gpus() {
                    sensor_settings = sensor_settings.add(Minimon::go_next_with_item(
                        &SETTINGS_GPU_CHOICE,
                        "",
                        Message::Settings(Some(SettingsVariant::Gpu)),
                    ));
                }

                content = content.push(sensor_settings);
            }

            content = content.padding(padding).spacing(padding);

            //let content = column!(sensor_settings);
            let limits = Limits::NONE
                .max_width(420.0)
                .min_width(360.0)
                .min_height(200.0)
                .max_height(550.0);

            self.core
                .applet
                .popup_container(content.apply(cosmic::widget::scrollable))
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
            Message::ColorPickerOpen(device, kind, id) => {
                info!("Message::ColorPickerOpen({kind:?}, {id:?})");
                match device {
                    DeviceKind::Cpu => {
                        debug!("Cpu");
                        self.colorpicker.activate(
                            device,
                            kind,
                            self.cpu.demo_graph(self.config.cpu.colors),
                        );
                    }
                    DeviceKind::CpuTemp => {
                        debug!("Temp");
                        self.colorpicker.activate(
                            device,
                            kind,
                            self.cputemp.demo_graph(self.config.cputemp.colors),
                        );
                    }
                    DeviceKind::Memory => {
                        self.colorpicker.activate(
                            device,
                            kind,
                            self.memory.demo_graph(self.config.memory.colors),
                        );
                    }
                    DeviceKind::Network(variant) => {
                        let (network, config) = network_select!(self, variant);
                        self.colorpicker
                            .activate(device, kind, network.demo_graph(config.colors));
                    }
                    DeviceKind::Disks(variant) => {
                        let (disks, _) = disks_select!(self, variant);
                        self.colorpicker
                            .activate(device, kind, disks.demo_graph(disks.colors()));
                    }
                    DeviceKind::Gpu => {
                        if let Some(id) = id {
                            if let (Some(config), Some(gpu)) =
                                (self.config.gpus.get(&id), self.gpus.get(&id))
                            {
                                self.colorpicker.activate(
                                    device,
                                    kind,
                                    gpu.demo_gpu_graph(config.gpu_colors),
                                );
                            } else {
                                error!("no config for selected GPU {id}");
                            }
                        } else {
                            error!("Id is None");
                        }
                    }
                    DeviceKind::Vram => {
                        debug!("Vram 1");
                        if let Some(id) = id {
                            if let (Some(config), Some(gpu)) =
                                (self.config.gpus.get(&id), self.gpus.get(&id))
                            {
                                debug!("Vram 2");
                                self.colorpicker.activate(
                                    device,
                                    kind,
                                    gpu.demo_vram_graph(config.vram_colors),
                                );
                            } else {
                                error!("no config for selected GPU {id}");
                            }
                        } else {
                            error!("Id is None");
                        }
                    }
                }
                self.colorpicker.set_variant(ColorVariant::Color1);
                let col = self
                    .colorpicker
                    .colors()
                    .get_color(self.colorpicker.variant());
                self.colorpicker.set_sliders(col);
            }

            Message::ColorPickerClose(save, id) => {
                info!("Message::ColorPickerClose({save})");
                if save {
                    self.set_colors(self.colorpicker.colors(), self.colorpicker.kind().0, id);
                    self.save_config();
                }
                self.colorpicker.deactivate();
            }

            Message::ColorPickerDefaults => {
                info!("Message::ColorPickerDefaults()");
                self.colorpicker.default_colors();
            }

            Message::ColorPickerAccent => {
                info!("Message::ColorPickerAccent()");
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
                info!("Message::ToggleNetCombined({toggle})");
                if toggle.is_true() {
                    self.network1.variant = NetworkVariant::Combined;
                    self.config.network1.variant = NetworkVariant::Combined;
                } else {
                    self.network1.variant = NetworkVariant::Download;
                    self.config.network1.variant = NetworkVariant::Download;
                }
                self.network2.variant = NetworkVariant::Upload;
                self.config.network2.variant = NetworkVariant::Upload;
                self.save_config();
            }

            Message::ToggleDisksCombined(toggle) => {
                info!("Message::ToggleDisksCombined({toggle})");
                if toggle.is_true() {
                    self.disks1.variant = DisksVariant::Combined;
                    self.config.disks1.variant = DisksVariant::Combined;
                } else {
                    self.disks1.variant = DisksVariant::Write;
                    self.config.disks1.variant = DisksVariant::Write;
                }
                self.disks2.variant = DisksVariant::Read;
                self.config.disks2.variant = DisksVariant::Read;
                self.save_config();
            }

            Message::ToggleDisksChart(variant, toggled) => {
                info!("Message::ToggleDiskChart({variant:?})");
                let (_, config) = disks_select!(self, variant);
                config.chart = toggled;
                self.save_config();
            }

            Message::ToggleDisksLabel(variant, toggled) => {
                info!("Message::ToggleDiskLabel({variant:?})");
                let (_, config) = disks_select!(self, variant);
                config.label = toggled;
                self.save_config();
            }

            Message::ToggleAdaptiveNet(variant, toggle) => {
                info!("Message::ToggleAdaptiveNet({variant:?}, {toggle:?})");
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

            Message::SelectGraphType(dev, kind) => {
                info!("Message::SelectGraphType({dev:?})");
                match dev {
                    DeviceKind::Cpu => {
                        self.cpu.set_graph_kind(kind);
                        self.config.cpu.kind = kind;
                    }
                    DeviceKind::CpuTemp => {
                        self.cputemp.set_graph_kind(kind);
                        self.config.cputemp.kind = kind;
                    }
                    DeviceKind::Memory => {
                        self.memory.set_graph_kind(kind);
                        self.config.memory.kind = kind;
                    }
                    _ => error!("Message::SelectGraphType unsupported kind/device combination."), // Disks and Network don't have graph selection
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
                }

                if self.is_laptop {
                    let current_on_ac = self.is_on_ac().unwrap_or(true);
                    if self.on_ac != current_on_ac {
                        self.on_ac = current_on_ac;

                        for (id, gpu) in &mut self.gpus {
                            if let Some(c) = self.config.gpus.get(id) {
                                if c.pause_on_battery {
                                    if current_on_ac {
                                        info!("Changed to AC, restart polling");
                                        gpu.restart(); // on AC, start polling
                                    } else {
                                        info!("Changed to DC, stop polling");
                                        gpu.stop(); // on battery, stop polling
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Message::ToggleCpuChart(toggled) => {
                info!("Message::ToggleCpuChart({toggled:?})");
                self.config.cpu.chart = toggled;
                self.save_config();
            }

            Message::ToggleCpuTempChart(toggled) => {
                info!("Message::ToggleCpuTempChart({toggled:?})");
                self.config.cputemp.chart = toggled;
                self.save_config();
            }

            Message::SelectCpuTempUnit(unit) => {
                info!("Message::SelectCpuTempUnit({unit:?})");
                self.config.cputemp.unit = unit;
                self.save_config();
            }
            Message::ToggleMemoryChart(toggled) => {
                info!("Message::ToggleMemoryChart({toggled:?})");
                self.config.memory.chart = toggled;
                self.save_config();
            }

            Message::ToggleNetChart(variant, toggled) => {
                info!("Message::ToggleNetChart({toggled:?})");
                let (_, config) = network_select!(self, variant);
                config.chart = toggled;
                self.save_config();
            }

            Message::ToggleCpuLabel(toggled) => {
                info!("Message::ToggleCpuLabel({toggled:?})");
                self.config.cpu.label = toggled;
                self.save_config();
            }

            Message::ToggleCpuTempLabel(toggled) => {
                info!("Message::ToggleCpuTempLabel({toggled:?})");
                self.config.cputemp.label = toggled;
                self.save_config();
            }

            Message::ToggleMemoryLabel(toggled) => {
                info!("Message::ToggleMemoryLabel({toggled:?})");
                self.config.memory.label = toggled;
                self.save_config();
            }

            Message::ToggleMemoryPercentage(toggled) => {
                info!("Message::ToggleMemoryPercentage({toggled:?})");
                self.config.memory.percentage = toggled;
                self.memory.set_percentage(toggled);
                self.save_config();
            }

            Message::ToggleNetLabel(variant, toggled) => {
                info!("Message::ToggleNetLabel({toggled:?})");
                let (_, config) = network_select!(self, variant);
                config.label = toggled;
                self.save_config();
            }

            Message::ConfigChanged(config) => {
                self.config = *config;
                self.sync_gpu_configs();
                self.tick_timer = self.config.refresh_rate as i64;
                self.cpu.set_colors(self.config.cpu.colors);
                self.cpu.set_graph_kind(self.config.cpu.kind);
                self.cputemp.set_colors(self.config.cputemp.colors);
                self.cputemp.set_graph_kind(self.config.cputemp.kind);
                self.cputemp.set_unit(self.config.cputemp.unit);
                self.memory.set_colors(self.config.memory.colors);
                self.memory.set_graph_kind(self.config.memory.kind);
                self.memory.set_percentage(self.config.memory.percentage);
                self.network1.set_colors(self.config.network1.colors);
                self.network2.set_colors(self.config.network2.colors);
                self.network1.variant = self.config.network1.variant;
                self.network2.variant = NetworkVariant::Upload;
                self.disks1.variant = self.config.disks1.variant;
                self.disks2.variant = DisksVariant::Read;
                self.set_network_max_y(NetworkVariant::Download);
                self.set_network_max_y(NetworkVariant::Upload);
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
                info!("Message::LaunchSystemMonitor()");
                Minimon::spawn_sysmon_by_index(self.config.sysmon);
            }

            Message::RefreshRateChanged(rate) => {
                info!("Message::RefreshRateChanged({:?})", rate);
                self.config.refresh_rate = (rate * 1000.0) as u32;
                self.set_tick();
                self.save_config();
            }

            Message::LabelSizeChanged(size) => {
                info!("Message::LabelSizeChanged({size:?})");
                self.config.label_size_default = size;
                self.save_config();
            }

            Message::ToggleMonospaceLabels(toggle) => {
                info!("Message::Monospacelabels({toggle:?})");
                self.config.monospace_labels = toggle;
                self.save_config();
            }

            Message::ToggleTightSpacing(toggle) => {
                info!("Message::ToggleTightSpacing({toggle:?})");
                self.config.tight_spacing = toggle;
                self.save_config();
            }

            Message::ToggleSymbols(toggle) => {
                info!("Message::ToggleSymbols({toggle:?})");
                self.config.symbols = toggle;
                self.save_config();
            }

            Message::Settings(setting) => {
                info!("Message::Settings({setting:?})");
                self.settings_page = setting;
            }
            Message::SysmonSelect(sysmon) => {
                info!("Message::SysmonSelect({sysmon:?})");
                self.config.sysmon = sysmon;
                self.save_config();
            }
            Message::GpuToggleChart(id, device, toggled) => {
                self.update_gpu_config(
                    id,
                    "GpuToggleChart",
                    device,
                    |config, device| match device {
                        DeviceKind::Gpu => config.gpu_chart = toggled,
                        DeviceKind::Vram => config.vram_chart = toggled,
                        _ => error!("GpuToggleChart: wrong kind {device:?}"),
                    },
                );
            }

            Message::GpuToggleLabel(id, device, toggled) => {
                self.update_gpu_config(
                    id,
                    "GpuToggleLabel",
                    device,
                    |config, device| match device {
                        DeviceKind::Gpu => config.gpu_label = toggled,
                        DeviceKind::Vram => config.vram_label = toggled,
                        _ => error!("GpuToggleLabel: wrong kind {device:?}"),
                    },
                );
            }

            Message::GpuToggleStackLabels(id, toggled) => {
                info!("Message::GpuToggleStackLabels({id:?}, {toggled:?})");
                if let Some(c) = self.config.gpus.get_mut(&id) {
                    c.stack_labels = toggled;
                } else {
                    error!("GpuToggleStackLabels: wrong id {id:?}");
                }
            }

            Message::GpuSelectGraphType(id, device, kind) => {
                info!("Message::GpuSelectGraphType({id:?}, {device:?})");
                self.update_gpu_config(
                    id.clone(),
                    "GpuSelectGraphType",
                    device,
                    |config, device| match device {
                        DeviceKind::Gpu => config.gpu_kind = kind,
                        DeviceKind::Vram => config.vram_kind = kind,
                        _ => error!("GpuSelectGraphType: wrong kind {device:?}"),
                    },
                );
                if let Some(gpu) = self.gpus.get_mut(&id) {
                    match device {
                        DeviceKind::Gpu => gpu.gpu.set_graph_kind(kind),
                        DeviceKind::Vram => gpu.vram.set_graph_kind(kind),
                        _ => error!("GpuSelectGraphType: wrong kind {device:?}"),
                    }
                }
            }
            Message::ToggleDisableOnBattery(id, toggled) => {
                info!("Message::ToggleDisableOnBattery({id:?}, {toggled:?})");
                if let Some(c) = self.config.gpus.get_mut(&id) {
                    c.pause_on_battery = toggled;
                } else {
                    error!("ToggleDisableOnBattery: wrong id {id:?}");
                }
            }
        }
        Task::none()
    }
}

impl Minimon {
    pub fn sub_page_header<'a, Message: 'static + Clone>(
        sub_page: Option<&'a str>,
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

        if let Some(p) = sub_page {
            let sub_page_header = widget::row::with_capacity(2).push(text::title3(p));

            widget::column::with_capacity(2)
                .push(previous_button)
                .push(sub_page_header)
                .spacing(6)
                .width(iced::Length::Shrink)
                .into()
        } else {
            widget::column::with_capacity(2)
                .push(previous_button)
                .spacing(6)
                .width(iced::Length::Shrink)
                .into()
        }
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

        // Create settings rows
        let refresh_row = settings::item(
            fl!("refresh-rate"),
            spin_button(
                format!("{refresh_rate:.2}"),
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
            row!(
                widget::checkbox("", self.config.monospace_labels)
                    .on_toggle(Message::ToggleMonospaceLabels)
            ),
        );

        let symbol_row = settings::item(
            fl!("enable-symbols"),
            widget::toggler(self.config.symbols).on_toggle(Message::ToggleSymbols),
        );

        let spacing_row = settings::item(
            fl!("settings-tight-spacing"),
            widget::toggler(self.config.tight_spacing).on_toggle(Message::ToggleTightSpacing),
        );

        let sysmon_row = settings::item(
            fl!("choose-sysmon"),
            row!(
                widget::dropdown(
                    &SYSMON_NAMES,
                    Some(self.config.sysmon),
                    Message::SysmonSelect
                )
                .width(220)
            ),
        );

        // Combine rows into a column with spacing
        column!(
            refresh_row,
            label_row,
            mono_row,
            symbol_row,
            spacing_row,
            sysmon_row
        )
        .spacing(10)
        .into()
    }

    fn cpu_panel_ui(&self, horizontal: bool) -> Vec<Element<crate::app::Message>> {
        let mut elements: Vec<Element<Message>> = Vec::new();

        // Handle the symbols button if needed
        if self.config.symbols && (self.config.cpu.label || self.config.cpu.chart) {
            let btn = self.core.applet.icon_button(ICON);
            elements.push(btn.into());
        }

        // Format CPU usage based on horizontal layout and sample value
        let formatted_cpu = if self.cpu.latest_sample() < 10.0 && horizontal {
            format!("{:.2}%", self.cpu.latest_sample())
        } else {
            format!("{:.1}%", self.cpu.latest_sample())
        };

        // Add the CPU label if needed
        if self.config.cpu.label {
            elements.push(self.figure_label(formatted_cpu).into());
        }

        // Add the CPU chart if needed
        if self.config.cpu.chart {
            let content = self
                .core
                .applet
                .icon_button_from_handle(Minimon::make_icon_handle(&self.cpu));
            elements.push(content.into());
        }

        elements
    }

    fn cpu_temp_panel_ui(&self, _horizontal: bool) -> Vec<Element<crate::app::Message>> {
        let mut elements: Vec<Element<Message>> = Vec::new();

        if self.cputemp.is_found() {
            // Handle the symbols button if needed
            if self.config.symbols && (self.config.cputemp.label || self.config.cputemp.chart) {
                let btn = self.core.applet.icon_button(TEMP_ICON);
                elements.push(btn.into());
            }

            // Add the CPU label if needed
            if self.config.cputemp.label {
                elements.push(self.figure_label(self.cputemp.to_string()).into());
            }

            // Add the CPU chart if needed
            if self.config.cputemp.chart {
                let content = self
                    .core
                    .applet
                    .icon_button_from_handle(Minimon::make_icon_handle(&self.cputemp));
                elements.push(content.into());
            }
        }

        elements
    }

    fn memory_panel_ui(&self, horizontal: bool) -> Vec<Element<crate::app::Message>> {
        let mut elements: Vec<Element<Message>> = Vec::new();

        // Handle the symbols button if needed
        if self.config.symbols && (self.config.memory.label || self.config.memory.chart) {
            let btn = self.core.applet.icon_button(RAM_ICON);
            elements.push(btn.into());
        }

        // Label section
        if self.config.memory.label {
            let formatted_mem = self.memory.to_string(!horizontal);
            elements.push(self.figure_label(formatted_mem).into());
        }

        // Chart section
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

        let format_label = |text: String| self.figure_label(text);

        if self.config.network1.label {
            let mut network_labels = Vec::new();

            // Download label
            let dl_text = if horizontal {
                format!(
                    "↓ {}",
                    self.network1
                        .download_label(sample_rate_ms, network::UnitVariant::Long)
                )
            } else {
                self.network1
                    .download_label(sample_rate_ms, network::UnitVariant::Short)
            };
            if nw_combined {
                network_labels.push(widget::vertical_space().into());
            }
            network_labels.push(format_label(dl_text).into());

            if nw_combined {
                // Upload label
                let ul_text = if horizontal {
                    format!(
                        "↑ {}",
                        self.network1
                            .upload_label(sample_rate_ms, network::UnitVariant::Long)
                    )
                } else {
                    self.network1
                        .upload_label(sample_rate_ms, network::UnitVariant::Short)
                };
                network_labels.push(format_label(ul_text).into());
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
            let mut network_labels = Vec::new();

            let ul_text = if horizontal {
                format!(
                    "↑ {}",
                    self.network2
                        .upload_label(sample_rate_ms, network::UnitVariant::Long)
                )
            } else {
                self.network2
                    .upload_label(sample_rate_ms, network::UnitVariant::Short)
            };
            network_labels.push(format_label(ul_text).into());

            elements.push(Column::from_vec(network_labels).into());
        }

        if self.config.network2.chart && !nw_combined {
            let svg = self.network2.graph();
            let handle = cosmic::widget::icon::from_svg_bytes(svg.into_bytes());
            let content = self.core.applet.icon_button_from_handle(handle);

            elements.push(content.into());
        }

        if self.config.symbols && !elements.is_empty() {
            let btn = self.core.applet.icon_button(NETWORK_ICON);
            elements.insert(0, btn.into());
        }

        elements
    }

    fn disks_panel_ui(&self, horizontal: bool) -> Vec<Element<crate::app::Message>> {
        let disks_combined = self.config.disks1.variant == DisksVariant::Combined;
        let sample_rate_ms = self.config.refresh_rate;
        let mut elements: Vec<Element<Message>> = Vec::new();

        let format_label = |text: String| self.figure_label(text);

        if self.config.disks1.label {
            let mut disks_labels = Vec::new();

            // Write label
            let write_text = if horizontal {
                format!(
                    "W {}",
                    self.disks1
                        .write_label(sample_rate_ms, disks::UnitVariant::Long)
                )
            } else {
                self.disks1
                    .write_label(sample_rate_ms, disks::UnitVariant::Short)
            };
            if disks_combined {
                disks_labels.push(widget::vertical_space().into());
            }
            disks_labels.push(format_label(write_text).into());

            if disks_combined {
                // Read label
                let read_text = if horizontal {
                    format!(
                        "R {}",
                        self.disks1
                            .read_label(sample_rate_ms, disks::UnitVariant::Long)
                    )
                } else {
                    self.disks1
                        .read_label(sample_rate_ms, disks::UnitVariant::Short)
                };
                disks_labels.push(format_label(read_text).into());
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
            let mut disks_labels = Vec::new();

            let read_text = if horizontal {
                format!(
                    "R {}",
                    self.disks2
                        .read_label(sample_rate_ms, disks::UnitVariant::Long)
                )
            } else {
                self.disks2
                    .read_label(sample_rate_ms, disks::UnitVariant::Short)
            };
            disks_labels.push(format_label(read_text).into());

            elements.push(Column::from_vec(disks_labels).into());
        }

        if self.config.disks2.chart && !disks_combined {
            let svg = self.disks2.graph();
            let handle = cosmic::widget::icon::from_svg_bytes(svg.into_bytes());
            let content = self.core.applet.icon_button_from_handle(handle);

            elements.push(content.into());
        }

        if self.config.symbols && !elements.is_empty() {
            let btn = self.core.applet.icon_button(DISK_ICON);
            elements.insert(0, btn.into());
        }

        elements
    }

    fn gpu_panel_ui(&self, gpu: &Gpu, horizontal: bool) -> Vec<Element<crate::app::Message>> {
        let mut elements: Vec<Element<Message>> = Vec::new();

        if let Some(config) = self.config.gpus.get(&gpu.id()) {
            let mut formatted_gpu = String::with_capacity(10);
            let mut formatted_vram = String::with_capacity(10);
            let stacked_labels = config.stack_labels && config.gpu_label && config.vram_label;

            if config.gpu_label {
                if gpu.is_active() {
                    let value = gpu.gpu.latest_sample();
                    if value < 10.0 && horizontal {
                        write!(&mut formatted_gpu, "{value:.2}%").ok();
                    } else {
                        write!(&mut formatted_gpu, "{value:.1}%").ok();
                    }
                } else {
                    formatted_gpu.push_str("----%");
                }
            }

            if config.vram_label {
                if gpu.is_active() {
                    formatted_vram = gpu.vram.string(!horizontal);
                } else {
                    let placeholder = if horizontal { "---- GB" } else { "----GB" };
                    formatted_vram.push_str(placeholder);
                }
            }

            if stacked_labels {
                let gpu_labels = vec![
                    widget::vertical_space().into(),
                    self.figure_label(formatted_gpu).into(),
                    self.figure_label(formatted_vram.clone()).into(),
                    widget::vertical_space().into(),
                ];
                elements.push(Column::from_vec(gpu_labels).into());
            } else if config.gpu_label {
                elements.push(self.figure_label(formatted_gpu).into());
            }

            if config.gpu_chart {
                let g = cosmic::widget::icon::from_svg_bytes(gpu.gpu.graph().into_bytes());
                let content = self.core.applet.icon_button_from_handle(g);
                elements.push(content.into());
            }

            if config.vram_label && !stacked_labels {
                elements.push(self.figure_label(formatted_vram).into());
            }

            if config.vram_chart {
                let g = cosmic::widget::icon::from_svg_bytes(gpu.vram.graph().into_bytes());
                let content = self.core.applet.icon_button_from_handle(g);
                elements.push(content.into());
            }
        }

        if self.config.symbols && !elements.is_empty() {
            let btn = self.core.applet.icon_button(GPU_ICON);
            elements.insert(0, btn.into());
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
                info!("Error writing config {err}");
            }
        }
    }

    fn set_colors(&mut self, colors: GraphColors, kind: DeviceKind, id: Option<String>) {
        match kind {
            DeviceKind::Cpu => {
                self.config.cpu.colors = colors;
                self.cpu.set_colors(colors);
            }
            DeviceKind::CpuTemp => {
                self.config.cputemp.colors = colors;
                self.cputemp.set_colors(colors);
            }
            DeviceKind::Memory => {
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
            DeviceKind::Gpu => {
                if let Some(id) = id {
                    if let (Some(config), Some(gpu)) =
                        (self.config.gpus.get_mut(&id), self.gpus.get_mut(&id))
                    {
                        config.gpu_colors = colors;
                        gpu.gpu.set_colors(colors);
                    } else {
                        error!("No config for selected GPU {id}");
                    }
                }
            }
            DeviceKind::Vram => {
                debug!("Vram set colors a {colors:?}");
                if let Some(id) = id {
                    if let (Some(config), Some(gpu)) =
                        (self.config.gpus.get_mut(&id), self.gpus.get_mut(&id))
                    {
                        debug!("Vram set colors b");
                        config.vram_colors = colors;
                        gpu.vram.set_colors(colors);
                    } else {
                        error!("No config for selected GPU {id}");
                    }
                }
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
        // Update everything if popup open
        let all = self.popup.is_some();

        self.cpu.update();

        if all || self.config.cputemp.is_visible() {
            self.cputemp.update();
        }

        if all || self.config.memory.is_visible() {
            self.memory.update();
        }

        let combined_network = self.config.network1.variant == NetworkVariant::Combined;
        if all
            || (combined_network && self.config.network1.is_visible())
            || (!combined_network
                && (self.config.network1.is_visible() || self.config.network1.is_visible()))
        {
            self.network1.update();
            self.network2.update();
        }

        let combined_disks = self.config.disks1.variant == DisksVariant::Combined;

        if all
            || (combined_disks && self.config.disks1.is_visible())
            || (!combined_disks
                && (self.config.disks1.is_visible() || self.config.disks2.is_visible()))
        {
            self.disks1.update();
            self.disks2.update();
        }

        for gpu in &mut self.gpus.values_mut() {
            if let Some(g) = self.config.gpus.get(&gpu.id()) {
                if all || g.is_visible() {
                    gpu.update();
                }
            }
        }
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
                found.push((format!("flatpak run {flatpak_id}"), name.to_string()));
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

    // make sure our liust of detected GPUs and the config list are equal
    fn sync_gpu_configs(&mut self) {
        let config_gpus = &mut self.config.gpus;

        // Remove entries not present in detected GPUs
        config_gpus.retain(|id, _| self.gpus.contains_key(id));

        // Add missing GPU configs
        for id in self.gpus.keys() {
            config_gpus.entry(id.clone()).or_default();
        }

        for (id, gpu) in &mut self.gpus {
            if let Some(config) = config_gpus.get(id) {
                gpu.gpu.set_graph_kind(config.gpu_kind);
                gpu.vram.set_graph_kind(config.vram_kind);
                gpu.gpu.set_colors(config.gpu_colors);
                gpu.vram.set_colors(config.vram_colors);
            }
        }
    }

    fn update_gpu_config<F>(&mut self, id: String, action: &str, device: DeviceKind, update_fn: F)
    where
        F: FnOnce(&mut GpuConfig, DeviceKind),
    {
        info!("{action}({:?})", (id.to_string(), &device));
        if let Some(config) = self.config.gpus.get_mut(&id) {
            update_fn(config, device);
            self.save_config();
        } else {
            error!("{action}: no config for selected GPU {id}");
        }
    }

    fn has_gpus(&self) -> bool {
        !self.gpus.is_empty()
    }

    fn is_on_ac(&self) -> Result<bool, Box<dyn std::error::Error>> {
        if self.is_laptop {
            // Connect to the system bus
            let connection = Connection::system()?;

            // Create a proxy to UPower service
            let proxy = zbus::blocking::Proxy::new(
                &connection,
                "org.freedesktop.UPower",
                "/org/freedesktop/UPower",
                "org.freedesktop.UPower",
            )?;

            // Get the list of power-related devices
            let devices: Vec<OwnedObjectPath> = proxy.call("EnumerateDevices", &())?;

            for device_path in devices {
                let device_proxy = zbus::blocking::Proxy::new(
                    &connection,
                    "org.freedesktop.UPower",
                    device_path.as_str(),
                    "org.freedesktop.UPower.Device",
                )?;

                // Get the Type property (1 = line power / AC)
                let kind: u32 = device_proxy.get_property("Type")?;
                if kind == 1 {
                    // Get the Online property
                    let online: bool = device_proxy.get_property("Online")?;
                    return Ok(online);
                }
            }
        }

        Ok(true)
    }

    fn is_laptop() -> bool {
        let power_supply_path = "/sys/class/power_supply";
        match fs::read_dir(power_supply_path) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .any(|entry| entry.file_name().to_string_lossy().starts_with("BAT")),
            Err(e) => {
                info!("Could not read power supply info: {e}");
                false
            }
        }
    }
}
