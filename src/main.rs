// SPDX-License-Identifier: GPL-3.0-only

use app::Minimon;

mod app;
mod colorpicker;
mod config;
mod i18n;
mod sensors;
#[cfg(feature = "caffeine")]
mod sleepinhibitor;
mod svg_graph;
mod charts;

use log::info;

use chrono::Local;
use std::io;

/// Controls whether logging goes to stdout or a file.
const LOG_TO_FILE: bool = false;

fn setup_logger() -> Result<(), Box<dyn std::error::Error>> {
    let base_config = fern::Dispatch::new()
        //.level(log::LevelFilter::Debug)
        .level(log::LevelFilter::Off)
        .level_for("libcosmic", log::LevelFilter::Debug)
        .level_for("cosmic_applet_minimon", log::LevelFilter::Debug)
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}] {}",
                Local::now().format("%H:%M:%S"),
                record.level(),
                message
            ));
        });

    if LOG_TO_FILE {
        base_config
            .chain(fern::log_file("/tmp/minimon.log")?)
            .apply()?;
    } else {
        base_config.chain(io::stdout()).apply()?;
    }

    Ok(())
}

fn main() -> cosmic::iced::Result {
    setup_logger().expect("Failed to initialize logger");

    info!("Application started");

    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
    i18n::init(&requested_languages);
    cosmic::applet::run::<Minimon>(())
}
