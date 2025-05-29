use cosmic::iced::mouse::Cursor;
use cosmic::iced::{Point, Renderer};
use cosmic::iced::{Rectangle, Size};
use cosmic::iced_widget::canvas::Geometry;
use cosmic::theme;
use cosmic::widget::canvas::{self, Stroke, Style, path};
use std::collections::VecDeque;

use cosmic::iced::Color;

use crate::app::Message;
use crate::config::GraphColors;

use super::GraphColorsIced;

#[derive(Debug)]
pub struct HeatChart {
    pub steps: usize,
    pub samples: VecDeque<f64>,
    pub max_y: Option<f64>,
    pub colors: GraphColorsIced,
}

impl HeatChart {
    pub fn new(
        steps: usize,
        samples: &VecDeque<f64>,
        max: Option<f64>,
        colors: &GraphColors,
    ) -> Self {
        HeatChart {
            steps,
            samples: samples.clone(),
            max_y: max,
            colors: (*colors).into(),
        }
    }
}

impl canvas::Program<Message, theme::Theme> for HeatChart {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &theme::Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry<Renderer>> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        let top_left = Point::new(
            frame.center().x - frame.size().width / 2. + 1.,
            frame.center().y - frame.size().height / 2. + 1.,
        );
        let bottom_right = Point::new(
            frame.center().x + frame.size().width / 2. - 1.,
            frame.center().y + frame.size().height / 2. - 1.,
        );
        let scale = bottom_right - top_left;

        // Find max value, if not provided will scale to largest value
        let max_value: f64 = if let Some(m) = self.max_y {
            m
        } else {
            let max_point = self.samples.iter().cloned().fold(0.0, f64::max);
            if max_point > 0.0 { max_point } else { 100.0 }
        };

        let corner_radius = frame.size().width.min(frame.size().height) / 7.0;

        let frame_color = self.colors.color2;
        let bg_color = self.colors.color1;

        frame.fill_rectangle(Point { x: 0.5, y: 0.5 }, Size {
            width: frame.size().width - 1.0,
            height: frame.size().height - 1.0,
        }, bg_color);

        let step_length = scale.x / self.steps as f32;
        let scaling = (scale.y - 0.5) as f64 / max_value;

        let mut builder = path::Builder::new();
        let mut shade_builder = path::Builder::new();

        let mut previous_point = None;

        for i in 0..self.samples.len() {
            let sample = self.samples[i].min(max_value);
            let y = 0.5 + scale.y - (sample * scaling) as f32;
            let x = (top_left.x + step_length * i as f32).round();
            let p = Point::new(x, y.round());

            if i == 0 {
                builder.move_to(p);
                shade_builder.move_to(Point::new(top_left.x, bottom_right.y));
                shade_builder.line_to(p);
            } else {
                builder.line_to(p);
                shade_builder.line_to(p);
            }

            previous_point = Some(p);
        }

        /* Bezier version, much more expensive, only looks better on big charts

                // Draw graph
                let step_length = scale.x / self.steps as f32;
                let scaling = (scale.y - 0.5) as f64 / max_value;

                let mut builder = path::Builder::new();
                let mut shade_builder = path::Builder::new();

                let mut previous_point: Option<Point> = None;

                for i in 0..self.samples.len() {
                    let sample = self.samples[i].min(max_value);
                    let y = 0.5 + scale.y - (sample * scaling) as f32;
                    let x = (top_left.x + step_length * i as f32).round();
                    let p = Point::new(x, y.round());

                    if i == 0 {
                        shade_builder.move_to(Point::new(top_left.x, bottom_right.y));
                        shade_builder.line_to(p);
                    } else if let Some(prev) = previous_point {
                        let control_prev = Point::new(prev.x + step_length * 0.5, prev.y);
                        let control_curr = Point::new(p.x - step_length * 0.5, p.y);

                        builder.move_to(prev);
                        builder.bezier_curve_to(control_prev, control_curr, p);
                        shade_builder.bezier_curve_to(control_prev, control_curr, p);
                    }

                    previous_point = Some(p);
                }
        */

        if let Some(_) = previous_point {
            shade_builder.line_to(bottom_right);
        }

        // Draw the chart, with a gradient

        let linear = cosmic::widget::canvas::gradient::Linear::new(
            Point::new(0.0, frame.size().height),
            Point::new(0.0, 0.0),
        )
        .add_stop(0.0, Color::from_rgba(1.0, 0.65, 0.0, 1.0))
        .add_stop(1.0, Color::from_rgba(1.0, 0.0, 0.0, 1.0));

        frame.fill(
            &shade_builder.build(),
            canvas::Fill {
                style: canvas::Style::Gradient(canvas::Gradient::Linear(linear)),
                ..Default::default()
            },
        );

        let frame_size: Size = Size {
            width: frame.size().width - 1.0,
            height: frame.size().height - 1.0,
        };

        // Erase corners, with transparent pixels
        for i in 0..=corner_radius.trunc() as i32 {
            let mut square = path::Builder::new();
            square.rounded_rectangle(Point { x: 0.5, y: 0.5 }, frame_size, i.into());
            frame.stroke(
                &square.build(),
                Stroke {
                    style: Style::Solid(theme.cosmic().bg_color().into()),
                    width: 1.0,
                    ..Default::default()
                },
            );
        }

        // Draw background square
        let mut square = path::Builder::new();
        square.rounded_rectangle(Point { x: 0.5, y: 0.5 }, frame_size, corner_radius.into());
        frame.stroke(
            &square.build(),
            Stroke {
                style: Style::Solid(frame_color),
                width: 1.0,
                ..Default::default()
            },
        );

        vec![frame.into_geometry()]
    }
}
