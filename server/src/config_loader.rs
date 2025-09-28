use fedi_wplace_application::error::{AppError, AppResult};
use fedi_wplace_application::infrastructure_config::Config;
use figment::{
    Figment,
    providers::{Env, Format, Json, Serialized, Toml},
};
use std::fs;
use std::path::Path;
use tracing::info;

pub fn load_config() -> AppResult<Config> {
    generate_env_template_if_missing()?;

    let default_config = Config::default();
    let mut figment = Figment::from(Serialized::defaults(default_config));

    if Path::new("config.toml").exists() {
        figment = figment.merge(Toml::file("config.toml"));
    }

    if Path::new("config.json").exists() {
        figment = figment.merge(Json::file("config.json"));
    }

    let config: Config = figment
        .merge(Env::prefixed("FEDIPLACE_").split("__"))
        .extract()
        .map_err(|e| AppError::ConfigError {
            message: format!("Failed to load configuration: {e}"),
        })?;

    config.validate()?;
    Ok(config)
}

fn generate_env_template_if_missing() -> AppResult<()> {
    let env_file = ".env";
    let template_file = ".env.example";

    if Path::new(env_file).exists() {
        return Ok(());
    }

    if !Path::new(template_file).exists() {
        return Ok(());
    }

    fs::copy(template_file, env_file).map_err(|e| AppError::ConfigError {
        message: format!("Failed to generate .env file from template: {e}"),
    })?;

    info!("Generated .env from template. Please configure your secrets!");
    info!("IMPORTANT: .env contains sensitive data and is gitignored.");

    Ok(())
}
