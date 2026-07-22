use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use tracing::{debug, error};

use crate::app::AppState;
use crate::database::AccessTokenRepository;
use crate::jwt::{JWTClaims, validate_jwt};

pub async fn handler(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    match headers
        .get("X-Gateway-Secret")
        .and_then(|v| v.to_str().ok())
    {
        Some(gateway_secret) => {
            if gateway_secret != state.config.gateway_secret {
                return StatusCode::FORBIDDEN.into_response();
            }
        }
        None => {
            return StatusCode::FORBIDDEN.into_response();
        }
    };

    let token = match headers.get("X-Token").and_then(|v| v.to_str().ok()) {
        Some(t) if t.starts_with("Bearer ") => t.strip_prefix("Bearer ").unwrap_or(""),
        Some(t) => t,
        None => {
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    if token.is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let client_id = headers
        .get("X-Client-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if let Some(claims) = state.token_cache.get(&token) {
        debug!("Token cache hit (valid)");

        return build_response(claims);
    }

    if let Some(claims) = state.token_cache.lock(&token).await {
        return build_response(claims);
    }

    debug!("Token cache miss, validating JWT");

    let claims = match validate_jwt(&token, client_id) {
        Ok(c) => c,
        Err(e) => {
            debug!("{}", e);
            state.token_cache.put_invalid(&token);
            state.token_cache.unlock(&token);

            return StatusCode::UNAUTHORIZED.into_response();
        }
    };

    match state.access_tokens.is_token_revoked(&claims.jti).await {
        Ok(false) => {
            state.token_cache.put_valid(&token, claims.clone());
        }
        Ok(true) => {
            state.token_cache.put_invalid(&token);
            state.token_cache.unlock(&token);

            return StatusCode::UNAUTHORIZED.into_response();
        }
        Err(e) => {
            error!("Database error while checking token: {}", e);
            state.token_cache.unlock(&token);

            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    state.token_cache.unlock(&token);

    build_response(claims)
}

fn build_response(claims: JWTClaims) -> Response {
    let mut response = StatusCode::OK.into_response();

    response
        .headers_mut()
        .insert("X-Sub", to_header_value(&claims.sub));
    if let Some(aud) = &claims.aud {
        response.headers_mut().insert("X-Aud", to_header_value(aud));
    }
    if let Some(scope) = claims.scopes.as_ref().map(|s| s.join(" ")) {
        response
            .headers_mut()
            .insert("X-Scope", to_header_value(&scope));
    }

    response
}

fn to_header_value(s: &str) -> HeaderValue {
    HeaderValue::from_str(s).unwrap_or_else(|_| HeaderValue::from_static(""))
}
