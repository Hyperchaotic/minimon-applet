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

use crate::config::{SvgColorVariant, SvgColors, SvgKind};
use crate::netmon::NetMon;
use crate::svgstat::SvgStat;

/// Data for managing the CircleColorPicker dialog
#[derive(Debug)]
pub struct ColorPicker {
    pub is_active: bool,
    /// Type of current displaying device CPU or Memory
    pub graph_kind: SvgKind,
    // Current field being adjusted background/text/etc.
    pub color_variant: SvgColorVariant,
    /// An example SVG to show the changes
    pub svg_ring: SvgStat,
    pub svg_line: NetMon,

    ///Current slider values
    pub slider_red_val: u8,
    pub slider_green_val: u8,
    pub slider_blue_val: u8,
}

impl ColorPicker {
    pub fn new(kind: SvgKind) -> Self {
        ColorPicker {
            is_active: false,
            graph_kind: kind,
            color_variant: SvgColorVariant::Color1,
            svg_ring: SvgStat::new(100),
            svg_line: NetMon::new(),
            slider_red_val: 0,
            slider_green_val: 0,
            slider_blue_val: 0,
        }
    }

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
                        gradient: Linear::new(Radians(0.0)).add_stops(color_stops.iter().copied()),
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
                        gradient: Linear::new(Radians(0.0)).add_stops(color_stops.iter().copied()),
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
                        gradient: Linear::new(Radians(0.0)).add_stops(color_stops.iter().copied()),
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

    pub fn sliders(&self) -> Srgb<u8> {
        Srgb::from_components((
            self.slider_red_val,
            self.slider_green_val,
            self.slider_blue_val,
        ))
    }

    pub fn demo_svg(&self) -> String {
        if self.graph_kind==SvgKind::Network {
            self.svg_line.svg_demo()
        } else {
            self.svg_ring.svg_demo()
        }
    }

    pub fn set_sliders(&mut self, color: Srgb<u8>) {
        self.slider_red_val = color.red;
        self.slider_green_val = color.green;
        self.slider_blue_val = color.blue;

        if self.graph_kind == SvgKind::Network {
            let mut col = self.svg_line.colors();
            col.set_color(self.sliders(), self.color_variant);
            self.svg_line.set_colors(col);
        } else {
            let mut col = self.svg_ring.colors();
            col.set_color(self.sliders(), self.color_variant);
            self.svg_ring.set_colors(col);
        }
    }

    pub fn set_colors(&mut self, colors: SvgColors) {
        if self.graph_kind==SvgKind::Network {
            self.svg_line.set_colors(colors);
            self.set_sliders(self.svg_line.colors().get_color(self.color_variant));
        } else {
            self.svg_ring.set_colors(colors);
            self.set_sliders(self.svg_ring.colors().get_color(self.color_variant));
        }
    }

    pub fn set_variant(&mut self, variant: SvgColorVariant) {
        self.color_variant = variant;

        let cols = if self.graph_kind == SvgKind::Network {
            self.svg_line.colors()
        } else {
            self.svg_ring.colors()
        };

        let col = cols.get_color(variant);

        self.slider_red_val = col.red;
        self.slider_green_val = col.green;
        self.slider_blue_val = col.blue;
    }

    pub fn colors(&self) -> SvgColors {
        if self.graph_kind == SvgKind::Network {
            self.svg_line.colors()
        } else {
            self.svg_ring.colors()
        }
    }
}
