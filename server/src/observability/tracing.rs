use std::{error::Error, io::stdout};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use fedi_wplace_application::infrastructure_config::{Config, LogFormat};

pub fn setup_logging(config: &Config) -> Result<(), Box<dyn Error>> {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.logging.level));

    match config.logging.format {
        LogFormat::Json => {
            let formatting_layer =
                BunyanFormattingLayer::new("fedi-wplace-backend".to_string(), stdout);
            let json_layer = JsonStorageLayer;

            tracing_subscriber::registry()
                .with(env_filter)
                .with(json_layer)
                .with(formatting_layer)
                .init();
        }
        LogFormat::Pretty => {
            let format = fmt::format()
                .with_target(true)
                .with_thread_ids(true)
                .compact();

            let mut subscriber = tracing_subscriber::fmt()
                .event_format(format)
                .with_env_filter(env_filter);

            if config.logging.include_location {
                subscriber = subscriber.with_file(true).with_line_number(true);
            }

            subscriber.init();
        }
    }

    Ok(())
}
