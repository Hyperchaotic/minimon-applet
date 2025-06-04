use cosmic::iced::mouse::Cursor;
use cosmic::iced::{Color, Point, Renderer};
use cosmic::iced::{Rectangle, Size};
use cosmic::iced_widget::canvas::Geometry;
use cosmic::theme;
use cosmic::widget::canvas::{self, Path, Stroke, Style, path};
use std::collections::VecDeque;

use crate::app::Message;
use crate::config::GraphColors;

use super::GraphColorsIced;

// Trait for numeric sample types
pub trait SampleValue: Copy + PartialOrd {
    fn to_f64(self) -> f64;
}

impl SampleValue for u64 {
    fn to_f64(self) -> f64 {
        self as f64
    }
}

impl SampleValue for f64 {
    fn to_f64(self) -> f64 {
        self
    }
}

// Generic LineChart widget
// * Draws a graph of the last 'steps' samples.
// * Can take u64 or f64.
// * Can be adaptive or take a fixed max_y.
// * If samples2.len() is 0, only draws samples1 graph.
#[derive(Debug)]
pub struct LineChart<T: SampleValue> {
    pub steps: usize,
    pub samples1: VecDeque<T>,
    pub samples2: VecDeque<T>,
    pub max_y: Option<T>,
    pub colors: GraphColorsIced,
}

// The new function clones the sampels and creates a new object.
// Alternatively the sensor could have a LineChart member and access
// the samples directly on update. But performance wise
// it is a neglible impact to clone into a new object instance every second.
impl<T: SampleValue> LineChart<T> {
    pub fn new(
        steps: usize,
        samples: &VecDeque<T>,
        samples2: &VecDeque<T>,
        max: Option<T>,
        colors: &GraphColors,
    ) -> Self {
        Self {
            steps,
            samples1: samples.clone(),
            samples2: samples2.clone(),
            max_y: max,
            colors: (*colors).into(),
        }
    }
}

impl<T: SampleValue + 'static> canvas::Program<Message, theme::Theme> for LineChart<T> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &theme::Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry<Renderer>> {
        fn build_path_from_samples<T: SampleValue>(
            builder: &mut path::Builder,
            shade_builder: &mut path::Builder,
            samples: &VecDeque<T>,
            steps: usize,
            max_value: f64,
            top_left: Point,
            bottom_right: Point,
            scale_y: f32,
            scaling: f64,
            step_length: f32,
        ) {
            let total = samples.len();
            let start_index = total.saturating_sub(steps);

            let mut previous_point = None;

            for (i, sample) in samples.iter().skip(start_index).take(steps).enumerate() {
                let value = sample.to_f64().min(max_value);
                let x = (top_left.x + step_length * i as f32).round();
                let y = 0.5 + scale_y - (value * scaling) as f32;
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

            if previous_point.is_some() {
                shade_builder.line_to(bottom_right);
            }
        }

        fn draw_graph(frame: &mut canvas::Frame, path: Path, shade: Path, mut color: Color) {
            frame.stroke(
                &path,
                canvas::Stroke {
                    style: canvas::Style::Solid(color),
                    width: 1.0,
                    line_join: canvas::LineJoin::Round,
                    ..Default::default()
                },
            );

            color.a = 0.3;

            frame.fill(
                &shade,
                canvas::Fill {
                    style: canvas::Style::Solid(color),
                    ..Default::default()
                },
            );
        }

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

        let min = scale.y as f64;
        let max_value = self.max_y.map(|v| v.to_f64()).unwrap_or_else(|| {
            let max1 = self.samples1.iter().map(|s| s.to_f64()).fold(0.0, f64::max);
            let max2 = self.samples2.iter().map(|s| s.to_f64()).fold(0.0, f64::max);
            max1.max(max2).max(min)
        });

        let step_length = scale.x / self.steps as f32;
        let scaling = (scale.y - 0.5) as f64 / max_value;

        let mut graph_path1 = path::Builder::new();
        let mut graph_fill1 = path::Builder::new();

        build_path_from_samples(
            &mut graph_path1,
            &mut graph_fill1,
            &self.samples1,
            self.steps,
            max_value,
            top_left,
            bottom_right,
            scale.y,
            scaling,
            step_length,
        );
        draw_graph(
            &mut frame,
            graph_path1.build(),
            graph_fill1.build(),
            self.colors.color2,
        );

        if !self.samples2.is_empty() {
            let mut graph_fill2 = path::Builder::new();
            let mut graph_path2 = path::Builder::new();
            build_path_from_samples(
                &mut graph_path2,
                &mut graph_fill2,
                &self.samples2,
                self.steps,
                max_value,
                top_left,
                bottom_right,
                scale.y,
                scaling,
                step_length,
            );
            draw_graph(
                &mut frame,
                graph_path2.build(),
                graph_fill2.build(),
                self.colors.color3,
            );
        }

        // Erase corners and draw frame
        let frame_size = Size {
            width: frame.size().width - 1.0,
            height: frame.size().height - 1.0,
        };
        let corner_radius = frame.size().width.min(frame.size().height) / 7.0;

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

        let mut square = path::Builder::new();
        square.rounded_rectangle(Point { x: 0.5, y: 0.5 }, frame_size, corner_radius.into());
        frame.stroke(
            &square.build(),
            Stroke {
                style: Style::Solid(self.colors.color4),
                width: 1.0,
                ..Default::default()
            },
        );

        vec![frame.into_geometry()]
    }
}
