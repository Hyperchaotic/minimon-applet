use crate::config::GraphColors;

#[derive(Debug, Clone, Copy)]
pub struct GraphColorsIced {
    pub color1: cosmic::iced::Color,
    pub color2: cosmic::iced::Color,
    pub color3: cosmic::iced::Color,
    pub color4: cosmic::iced::Color,
}

impl From<GraphColors> for GraphColorsIced {
    fn from(colors: GraphColors) -> Self {
        fn to_iced_color(srgba: cosmic::cosmic_theme::palette::Srgba<u8>) -> cosmic::iced::Color {
            cosmic::iced::Color {
                r: srgba.color.red as f32 / 255.0,
                g: srgba.color.green as f32 / 255.0,
                b: srgba.color.blue as f32 / 255.0,
                a: srgba.alpha as f32 / 255.0,
            }
        }

        GraphColorsIced {
            color1: to_iced_color(colors.color1),
            color2: to_iced_color(colors.color2),
            color3: to_iced_color(colors.color3),
            color4: to_iced_color(colors.color4),
        }
    }
}

pub mod ring;
pub mod heat;