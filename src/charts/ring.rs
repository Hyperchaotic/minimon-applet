use cosmic::iced::Point;
use cosmic::iced::Radians;
use cosmic::iced::Rectangle;
use cosmic::iced::mouse::Cursor;
use cosmic::Renderer;
use cosmic::theme;
use cosmic::widget::canvas;
use cosmic::widget::canvas::Geometry;

use cosmic::widget::canvas::Text;
use cosmic::widget::canvas::Path;
use cosmic::widget::canvas::path::Arc;

use std::f32::consts::PI;

use crate::config::GraphColors;
use crate::app::Message;

use super::GraphColorsIced;

#[derive(Debug)]
pub struct RingChart {
    // How much if the ring is filled. 0..100
    pub percent: f32,

    //Text to display inside, if any
    pub text: String,
    pub colors: GraphColorsIced,
}

impl RingChart {
    pub fn new(percent: f32, text: &str, colors: &GraphColors) -> Self {
        RingChart {
            percent: if percent <= 100.0 { percent } else { 100.0 },
            text: text.to_string(),
            colors: (*colors).into(),
        }
    }
}

impl canvas::Program<Message, theme::Theme> for RingChart {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &theme::Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry<Renderer>> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        /* 
        // Not sure if we need to fill the canvas with transparency?
        // don't for now
        frame.fill_rectangle(
            Point::ORIGIN,                         
            Size::new(bounds.width, bounds.height),
            cosmic::iced::Color::from_rgba(0.0, 0.0, 0.0, 0.0), 
        );
        */

        // The starting poing of the Ring graph, bottom/6pm
        let starting_point = PI / 2.0;

        // Max height/width of chart/widget. Side lebgth in a square
        let limit = bounds.width.min(bounds.height);

        // Width and radius of ring
        let stroke_width = 0.08*limit;
        let radius = (limit / 2.0) - stroke_width / 2.0;
        
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);

        // Fill background color inside ring
        let inner_circle = Path::circle(center, radius - stroke_width / 2.0 );
        frame.fill(&inner_circle, self.colors.color1);

        // Draw ring segment showing status/percentage
        let start_angle = starting_point;
        let end_angle = starting_point + PI * 2.0 * (self.percent / 100.0);

        let ring1 = Path::new(|p| {
            p.arc(Arc {
                center: center,
                radius: radius,
                start_angle: Radians::from(start_angle),
                end_angle: Radians::from(end_angle),
            });
        });

        frame.stroke(
            &ring1,
            canvas::Stroke {
                style: canvas::Style::Solid(self.colors.color4),
                width: stroke_width,
                ..Default::default()
            },
        );

        // Fill unused ring segment
        if end_angle < 2.0 * PI {
            let start_angle = end_angle;
            let end_angle = starting_point + PI * 2.0;
            let ring2 = Path::new(|p| {
                p.arc(Arc {
                    center: center,
                    radius: radius,
                    start_angle: Radians::from(start_angle),
                    end_angle: Radians::from(end_angle),
                });
            });
            frame.stroke(
                &ring2,
                canvas::Stroke {
                    style: canvas::Style::Solid(self.colors.color3),
                    width: stroke_width,
                    ..Default::default()
                },
            );
        }

        // Create text object
        let text = Text {
            content: self.text.clone(),
            position: center,
            color: self.colors.color2,
            size: cosmic::iced::Pixels(radius * 0.95),
            horizontal_alignment: cosmic::iced::alignment::Horizontal::Center,
            vertical_alignment: cosmic::iced::alignment::Vertical::Center,
            ..Default::default()
        };

        frame.fill_text(text);

        return vec![frame.into_geometry()];
    }
}
