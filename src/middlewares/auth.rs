use axum::{
    extract::{Request, State},
    http::HeaderValue,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tracing::{debug, error};

use crate::{app::AppState, database::AccessTokenRepository, jwt::validate_jwt};

use crate::responses::{server_error_response, unauthorized_response};

pub async fn handle(State(state): State<Arc<AppState>>, mut req: Request, next: Next) -> Response {
    let token = match req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
    {
        Some(t) if t.starts_with("Bearer ") => t.strip_prefix("Bearer ").unwrap_or(""),
        Some(t) => t,
        None => {
            debug!("No token in Authorization header");
            return unauthorized_response();
        }
    };

    let claims = match state.token_cache.get(&token) {
        Some(claims) => {
            debug!("Token cache hit (valid)");
            claims
        }
        None => {
            debug!("Token cache miss, validating JWT");

            let claims = match validate_jwt(&token, None) {
                Ok(c) => c,
                Err(e) => {
                    debug!("{}", e);
                    state.token_cache.put_invalid(&token);
                    return unauthorized_response();
                }
            };

            match state.access_tokens.is_token_revoked(&claims.jti).await {
                Ok(false) => {
                    state.token_cache.put_valid(&token, claims.clone());

                    claims
                }
                Ok(true) => {
                    state.token_cache.put_invalid(&token);

                    return unauthorized_response();
                }
                Err(e) => {
                    error!("Database error while checking token: {}", e);
                    return server_error_response();
                }
            }
        }
    };

    let headers = req.headers_mut();
    headers.remove("Authorization");

    headers.remove("X-Gateway-Secret");
    headers.insert(
        "X-Gateway-Secret",
        HeaderValue::from_str(&state.config.gateway_secret)
            .unwrap_or_else(|_| HeaderValue::from_static("")),
    );

    headers.remove("X-User-Id");
    headers.insert(
        "X-User-Id",
        HeaderValue::from_str(&claims.sub).unwrap_or_else(|_| HeaderValue::from_static("")),
    );

    headers.remove("X-Client-Id");
    if let Some(client_id) = &claims.aud {
        headers.insert(
            "X-Client-Id",
            HeaderValue::from_str(client_id).unwrap_or_else(|_| HeaderValue::from_static("")),
        );
    }

    headers.remove("X-Scope");
    headers.insert(
        "X-Scope",
        HeaderValue::from_str(
            &claims
                .scopes
                .as_ref()
                .map(|s| s.join(" "))
                .unwrap_or_default(),
        )
        .unwrap_or_else(|_| HeaderValue::from_static("")),
    );

    next.run(req).await
}
