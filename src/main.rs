// SPDX-License-Identifier: GPL-3.0-only

use app::Minimon;

mod app;
mod colorpicker;
mod config;
mod core;
mod netmon;
mod svgstat;
mod svg_graph;

/// The `cosmic::app::run()` function is the starting point of your application.
/// It takes two arguments:
/// - `settings` is a structure that contains everything relevant with your app's configuration, such as antialiasing, themes, icons, etc...
/// - `()` is the flags that your app needs to use before it starts.
fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<Minimon>(())
}
