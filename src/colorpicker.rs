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

use crate::config::{SvgColorVariant, SvgColors};

const ERROR: &str = "<svg width=\"800px\" height=\"800px\" viewBox=\"0 0 25 25\" fill=\"none\" xmlns=\"http://www.w3.org/2000/svg\">
<path d=\"M12.5 16V14.5M12.5 9V13M20.5 12.5C20.5 16.9183 16.9183 20.5 12.5 20.5C8.08172 20.5 4.5 16.9183 4.5 12.5C4.5 8.08172 8.08172 4.5 12.5 4.5C16.9183 4.5 20.5 8.08172 20.5 12.5Z\" stroke=\"red\" stroke-width=\"1.2\"/>
</svg>";

pub trait DemoSvg {
    fn svg_demo(&self) -> String;
    fn svg_colors(&self) -> SvgColors;
    fn svg_set_colors(&mut self, colors: SvgColors);
}

/// Data for managing the `ColorPicker` dialog
pub struct ColorPicker {
    demo_svg: Option<Box<dyn DemoSvg>>,
    // Current field being adjusted background/text/etc.
    color_variant: SvgColorVariant,
    ///Current slider values
    slider_red_val: u8,
    slider_green_val: u8,
    slider_blue_val: u8,
}

impl ColorPicker {
    pub fn new() -> Self {
        ColorPicker {
            demo_svg: None,
            color_variant: SvgColorVariant::Color1,
            slider_red_val: 0,
            slider_green_val: 0,
            slider_blue_val: 0,
        }
    }

    pub fn active(&self) -> bool {
        self.demo_svg.is_some()
    }

    pub fn activate(&mut self, demo_svg: Box<dyn DemoSvg>) {
        self.demo_svg = Some(demo_svg);
    }

    pub fn deactivate(&mut self) {
        self.demo_svg = None;
    }

    pub fn color_slider<'b, Message>(
        range: RangeInclusive<u8>,
        value: u8,
        on_change: impl Fn(u8) -> Message + 'b,
        color_stops: &'static [ColorStop],
    ) -> cosmic::Element<'b, Message>
    where
        Message: Clone + 'b,
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
        if let Some(d) = self.demo_svg.as_ref() {
            return d.svg_demo();
        } 
        ERROR.to_string()
    }

    pub fn set_sliders(&mut self, color: Srgb<u8>) {
        self.slider_red_val = color.red;
        self.slider_green_val = color.green;
        self.slider_blue_val = color.blue;

        if let Some(d) = self.demo_svg.as_mut() {
            let mut col = d.svg_colors();
            col.set_color(color, self.color_variant);
            d.svg_set_colors(col);
        }
    }

    pub fn set_colors(&mut self, colors: SvgColors) {
        if let Some(d) = self.demo_svg.as_mut() {
            d.svg_set_colors(colors);
            self.set_sliders(colors.get_color(self.color_variant));
        }
    }

    pub fn variant(&self) -> SvgColorVariant {
        self.color_variant
    }

    pub fn set_variant(&mut self, variant: SvgColorVariant) {

        if let Some(d) = self.demo_svg.as_mut() {
            
            self.color_variant = variant;
            let color = d.svg_colors().get_color(variant);

            self.slider_red_val = color.red;
            self.slider_green_val = color.green;
            self.slider_blue_val = color.blue;
            }
    }

    pub fn colors(&self) -> SvgColors {
        if let Some(d) = self.demo_svg.as_ref() {
            return d.svg_colors()
        }
        SvgColors::default()
    }
}
