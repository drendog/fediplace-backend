use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
};
use axum_login::AuthSession;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl, basic::BasicClient,
};
use reqwest;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use tracing::{error, info};
use url;
#[cfg(feature = "docs")]
use utoipa::ToSchema;

use crate::incoming::http_axum::auth::backend::{AuthBackend, User};
use crate::incoming::http_axum::error_mapper::HttpError;
use crate::shared::app_state::AppState;
use fedi_wplace_application::{error::AppError, infrastructure_config::AuthConfig};

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://www.googleapis.com/oauth2/v4/token";
const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v3/userinfo";
const OAUTH_STATE_KEY: &str = "oauth_state";

#[derive(Debug, Serialize, Deserialize)]
struct GoogleUserInfo {
    sub: String,
    email: String,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OAuthState {
    csrf_state: CsrfToken,
    pkce_verifier: PkceCodeVerifier,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "docs", derive(ToSchema))]
pub struct AuthRequest {
    code: String,
    state: String,
}

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/auth/google/start",
    tag = "auth",
    summary = "Start Google OAuth authentication",
    description = "Initiates the Google OAuth 2.0 authentication flow by redirecting to Google's authorization server with PKCE and CSRF protection.",
    responses(
        (status = 302, description = "Redirect to Google OAuth authorization server"),
        (status = 500, description = "Internal server error")
    )
))]
pub async fn google_auth_start(
    State(state): State<AppState>,
    session: Session,
) -> Result<Response, HttpError> {
    let client = BasicClient::new(ClientId::new(
        state
            .config
            .auth
            .google_client_id
            .clone()
            .unwrap_or_default(),
    ))
    .set_client_secret(ClientSecret::new(
        state
            .config
            .auth
            .google_client_secret
            .clone()
            .unwrap_or_default(),
    ))
    .set_auth_uri(
        AuthUrl::new(GOOGLE_AUTH_URL.to_string())
            .map_err(|_| HttpError(AppError::InternalServerError))?,
    )
    .set_token_uri(
        TokenUrl::new(GOOGLE_TOKEN_URL.to_string())
            .map_err(|_| HttpError(AppError::InternalServerError))?,
    )
    .set_redirect_uri(
        RedirectUrl::new(
            state
                .config
                .auth
                .google_redirect_url
                .clone()
                .unwrap_or_default(),
        )
        .map_err(|_| HttpError(AppError::InternalServerError))?,
    );

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let csrf_token = CsrfToken::new_random();

    let oauth_state = OAuthState {
        csrf_state: csrf_token.clone(),
        pkce_verifier,
    };

    session
        .insert(OAUTH_STATE_KEY, &oauth_state)
        .await
        .map_err(|e| {
            error!("Failed to store OAuth state in session: {}", e);
            HttpError(AppError::InternalServerError)
        })?;

    let (auth_url, _) = client
        .authorize_url(|| csrf_token.clone())
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    info!("Redirecting to Google OAuth: {}", auth_url);

    Ok(Redirect::to(auth_url.as_ref()).into_response())
}

fn redirect_to_error(config: &AuthConfig, error_type: &str) -> Response {
    let Ok(mut url) = url::Url::parse(&config.frontend_error_url) else {
        error!("Invalid frontend error URL configuration");
        let fallback_url = format!("{}/login", config.public_base_url);
        let Ok(mut fallback) = url::Url::parse(&fallback_url) else {
            error!("Invalid public base URL configuration, using minimal fallback");
            return Redirect::to("/login?error=config_error").into_response();
        };
        fallback
            .query_pairs_mut()
            .append_pair("error", "config_error");
        return Redirect::to(fallback.as_str()).into_response();
    };
    url.query_pairs_mut().append_pair("error", error_type);
    let error_url = url.as_str();
    error!("Auth error, redirecting to: {}", error_url);
    Redirect::to(error_url).into_response()
}

async fn validate_oauth_state(
    session: &Session,
    state_param: &str,
    config: &AuthConfig,
) -> Result<OAuthState, Response> {
    let oauth_state: OAuthState = match session.get(OAUTH_STATE_KEY).await {
        Ok(Some(state)) => state,
        Ok(None) => {
            error!("OAuth state not found in session");
            return Err(redirect_to_error(config, "invalid_state"));
        }
        Err(e) => {
            error!("Failed to retrieve OAuth state from session: {}", e);
            return Err(redirect_to_error(config, "session_error"));
        }
    };

    if oauth_state.csrf_state.secret() != state_param {
        error!("CSRF state validation failed");
        return Err(redirect_to_error(config, "invalid_state"));
    }

    if let Err(e) = session.remove::<OAuthState>(OAUTH_STATE_KEY).await {
        error!("Failed to remove OAuth state from session: {}", e);
        return Err(redirect_to_error(config, "session_error"));
    }

    Ok(oauth_state)
}

async fn fetch_user_info(
    access_token: &oauth2::AccessToken,
    config: &AuthConfig,
) -> Result<GoogleUserInfo, Response> {
    let http_client = reqwest::Client::new();
    let response = match http_client
        .get(GOOGLE_USERINFO_URL)
        .bearer_auth(access_token.secret())
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            error!("Failed to fetch user info: {}", e);
            return Err(redirect_to_error(config, "userinfo_fetch_failed"));
        }
    };

    if !response.status().is_success() {
        error!("Google API returned error: {}", response.status());
        return Err(redirect_to_error(config, "userinfo_api_error"));
    }

    match response.json().await {
        Ok(info) => Ok(info),
        Err(e) => {
            error!("Failed to parse user info: {}", e);
            Err(redirect_to_error(config, "userinfo_parse_failed"))
        }
    }
}

#[cfg_attr(feature = "docs", utoipa::path(
    get,
    path = "/auth/google/callback",
    tag = "auth",
    summary = "Handle Google OAuth callback",
    description = "Processes the OAuth callback from Google, exchanges the authorization code for tokens, fetches user information, and creates or updates the user account. On success, redirects to frontend success URL. On error, redirects to frontend error URL with error parameter.",
    params(
        ("code" = String, Query, description = "Authorization code from Google OAuth"),
        ("state" = String, Query, description = "CSRF state token for validation")
    ),
    responses(
        (status = 302, description = "Redirect to frontend success URL on successful authentication, or error URL on failure")
    )
))]
pub async fn google_auth_callback(
    State(state): State<AppState>,
    session: Session,
    mut auth_session: AuthSession<AuthBackend>,
    Query(params): Query<AuthRequest>,
) -> Response {
    info!("OAuth callback received with code: {}", &params.code[..8]);

    let oauth_state = match validate_oauth_state(&session, &params.state, &state.config.auth).await
    {
        Ok(state) => state,
        Err(response) => return response,
    };

    let auth_url = match AuthUrl::new(GOOGLE_AUTH_URL.to_string()) {
        Ok(url) => url,
        Err(e) => {
            error!("Failed to create auth URL: {}", e);
            return redirect_to_error(&state.config.auth, "config_error");
        }
    };

    let token_url = match TokenUrl::new(GOOGLE_TOKEN_URL.to_string()) {
        Ok(url) => url,
        Err(e) => {
            error!("Failed to create token URL: {}", e);
            return redirect_to_error(&state.config.auth, "config_error");
        }
    };

    let redirect_url = match RedirectUrl::new(
        state
            .config
            .auth
            .google_redirect_url
            .clone()
            .unwrap_or_default(),
    ) {
        Ok(url) => url,
        Err(e) => {
            error!("Failed to create redirect URL: {}", e);
            return redirect_to_error(&state.config.auth, "config_error");
        }
    };

    let client = BasicClient::new(ClientId::new(
        state
            .config
            .auth
            .google_client_id
            .clone()
            .unwrap_or_default(),
    ))
    .set_client_secret(ClientSecret::new(
        state
            .config
            .auth
            .google_client_secret
            .clone()
            .unwrap_or_default(),
    ))
    .set_auth_uri(auth_url)
    .set_token_uri(token_url)
    .set_redirect_uri(redirect_url);

    let token_result = match client
        .exchange_code(AuthorizationCode::new(params.code))
        .set_pkce_verifier(oauth_state.pkce_verifier)
        .request_async(&reqwest::Client::new())
        .await
    {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to exchange authorization code: {}", e);
            return redirect_to_error(&state.config.auth, "token_exchange_failed");
        }
    };

    let access_token = token_result.access_token();

    let user_info = match fetch_user_info(access_token, &state.config.auth).await {
        Ok(info) => info,
        Err(response) => return response,
    };

    info!("Google user info: {:?}", user_info);

    let user_public = match state
        .auth_use_case
        .upsert_social_identity(
            "google".to_string(),
            user_info.sub,
            Some(user_info.email),
            Some(user_info.name),
        )
        .await
    {
        Ok(user) => user,
        Err(e) => {
            error!("Failed to upsert social identity: {}", e);
            return redirect_to_error(&state.config.auth, "auth_failed");
        }
    };

    let user: User = user_public.into();

    if let Err(e) = auth_session.login(&user).await {
        error!("Failed to log user into session: {}", e);
        return redirect_to_error(&state.config.auth, "login_failed");
    }

    info!("User authentication and login successful: {}", user.email);

    Redirect::to(&state.config.auth.frontend_success_url).into_response()
}
