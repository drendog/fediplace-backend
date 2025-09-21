use axum::http::HeaderValue;
use axum::{extract::Request, middleware::Next, response::Response};
use uuid::Uuid;

const REQUEST_ID_HEADER: &str = "X-Request-Id";

pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|header| header.to_str().ok())
        .map_or_else(|| Uuid::new_v4().to_string(), ToString::to_string);

    let request_path = request.uri().path().to_string();
    let request_method = request.method().to_string();
    let is_auth_endpoint = request_path.starts_with("/auth/");

    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        request
            .headers_mut()
            .insert(REQUEST_ID_HEADER, header_value);
    }

    if is_auth_endpoint {
        tracing::info!(
            request_id = %request_id,
            method = %request_method,
            path = %request_path,
            "Processing auth request"
        );
    }

    let mut response = next.run(request).await;

    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .insert(REQUEST_ID_HEADER, header_value);
    }

    if is_auth_endpoint {
        tracing::info!(
            request_id = %request_id,
            status = %response.status(),
            method = %request_method,
            path = %request_path,
            "Auth request completed"
        );
    }

    response
}
