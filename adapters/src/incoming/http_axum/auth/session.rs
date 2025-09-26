use tower_sessions::{SessionManagerLayer, cookie::SameSite};
use tower_sessions_redis_store::{RedisStore, fred::prelude::*};

use fedi_wplace_application::error::AppError;

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub cookie_name: String,
    pub secure: bool,
    pub same_site: String,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            cookie_name: "fediplace_session".to_string(),
            secure: false,
            same_site: "Lax".to_string(),
        }
    }
}

pub async fn create_session_layer(
    redis_url: &str,
    session_config: &SessionConfig,
) -> Result<SessionManagerLayer<RedisStore<Client>>, AppError> {
    let redis_config = Config::from_url(redis_url).map_err(|_| AppError::InternalServerError)?;

    let redis_client = Client::new(redis_config, None, None, None);
    redis_client.connect();
    redis_client
        .wait_for_connect()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    let session_store = RedisStore::new(redis_client);

    let same_site = match session_config.same_site.to_lowercase().as_str() {
        "strict" => SameSite::Strict,
        "none" => SameSite::None,
        _ => SameSite::Lax,
    };

    let session_layer = SessionManagerLayer::new(session_store)
        .with_name(session_config.cookie_name.clone())
        .with_same_site(same_site)
        .with_secure(session_config.secure)
        .with_http_only(true);

    Ok(session_layer)
}
