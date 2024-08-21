use cosmic::cosmic_theme::palette::Srgb;
use std::ops::RangeInclusive;
use std::rc::Rc;

use cosmic::{
    iced::{
        gradient::{ColorStop, Linear},
        Color, Length, Radians,
    },
    theme,
    widget::{
        self,
        slider::{self, HandleShape, RailBackground},
    },
};

use crate::config::{GraphColorVariant, GraphColors, GraphKind};
use crate::svgstat::SvgStat;

/// Data for managing the colorpicker dialog
#[derive(Debug)]
pub struct ColorPicker {
    /// If dialog is active this is not None
    active: bool,
    /// Type of current displaying device CPU or Memory
    pub graph_kind: GraphKind,
    // Current field being adjusted background/text/etc.
    pub color_variant: GraphColorVariant,
    /// An example SVG to show the changes
    pub example_svg: SvgStat,
    ///Current slider values
    pub slider_red_val: u8,
    pub slider_green_val: u8,
    pub slider_blue_val: u8,
}

impl ColorPicker {

    // Thanks to PixelDoted/cosmic-color-picker for his cool slider
    pub fn color_slider<'a, Message>(
        range: RangeInclusive<u8>,
        value: u8,
        on_change: impl Fn(u8) -> Message + 'a,
        color_stops: &'static [ColorStop],
    ) -> cosmic::Element<'a, Message>
    where
        Message: Clone + 'a,
    {
        widget::slider(range, value, on_change)
            .width(Length::Fixed(220.0))
            .step(1)
            .style(theme::iced::Slider::Custom {
                active: Rc::new(|t| {
                    let cosmic = t.cosmic();
                    let mut a = slider::StyleSheet::active(t, &theme::iced::Slider::default());
                    a.rail.colors = RailBackground::Gradient {
                        gradient: Linear::new(Radians(0.0)).add_stops(color_stops.iter().cloned()),
                        auto_angle: true,
                    };
                    a.rail.width = 8.0;
                    a.handle.color = Color::TRANSPARENT;
                    a.handle.shape = HandleShape::Circle { radius: 8.0 };
                    a.handle.border_color = cosmic.palette.neutral_10.into();
                    a.handle.border_width = 4.0;
                    a
                }),
                hovered: Rc::new(|t| {
                    let cosmic = t.cosmic();
                    let mut a = slider::StyleSheet::active(t, &theme::iced::Slider::default());
                    a.rail.colors = RailBackground::Gradient {
                        gradient: Linear::new(Radians(0.0)).add_stops(color_stops.iter().cloned()),
                        auto_angle: true,
                    };
                    a.rail.width = 8.0;
                    a.handle.color = Color::TRANSPARENT;
                    a.handle.shape = HandleShape::Circle { radius: 8.0 };
                    a.handle.border_color = cosmic.palette.neutral_10.into();
                    a.handle.border_width = 4.0;
                    a
                }),
                dragging: Rc::new(|t| {
                    let cosmic = t.cosmic();
                    let mut a = slider::StyleSheet::active(t, &theme::iced::Slider::default());
                    a.rail.colors = RailBackground::Gradient {
                        gradient: Linear::new(Radians(0.0)).add_stops(color_stops.iter().cloned()),
                        auto_angle: true,
                    };
                    a.rail.width = 8.0;
                    a.handle.color = Color::TRANSPARENT;
                    a.handle.shape = HandleShape::Circle { radius: 8.0 };
                    a.handle.border_color = cosmic.palette.neutral_10.into();
                    a.handle.border_width = 4.0;
                    a
                }),
            })
            .into()
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn set_active(&mut self, activation: bool) {
        self.active = activation;
    }

    pub fn sliders(&self) -> Srgb<u8> {
        Srgb::from_components((
            self.slider_red_val,
            self.slider_green_val,
            self.slider_blue_val,
        ))
    }

    pub fn set_sliders(&mut self, color: Srgb<u8>) {
        self.slider_red_val = color.red;
        self.slider_green_val = color.green;
        self.slider_blue_val = color.blue;

        let mut col = self.example_svg.colors();
        col.set_color(self.sliders(), self.color_variant);
        self.example_svg.set_colors(col);
    }

    pub fn set_colors(&mut self, colors: GraphColors) {
        self.example_svg.set_colors(colors);
    }

    pub fn set_variant(&mut self, variant: GraphColorVariant) {
        self.color_variant = variant;

        let col = self.example_svg.colors().to_srgb(variant);

        self.slider_red_val = col.red;
        self.slider_green_val = col.green;
        self.slider_blue_val = col.blue;
    }

    pub fn colors(&self) -> GraphColors {
        self.example_svg.colors()
    }
}

impl Default for ColorPicker {
    fn default() -> Self {
        let mut dev = SvgStat::new(100);
        dev.set_variable(50.0);
        Self {
            active: false,
            graph_kind: GraphKind::Cpu,
            color_variant: GraphColorVariant::RingFront,
            example_svg: dev,
            slider_red_val: 0,
            slider_green_val: 0,
            slider_blue_val: 0,
        }
    }
}
