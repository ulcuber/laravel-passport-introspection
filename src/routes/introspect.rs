use std::sync::Arc;

use axum::{
    extract::{Form, Json, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

use crate::app::AppState;
use crate::database::AccessTokenRepository;
use crate::jwt::validate_jwt;
use crate::validation::{Validatable, ValidationException};

#[derive(Debug, Deserialize)]
pub struct IntrospectRequest {
    pub token: Option<String>,
    #[serde(default)]
    pub token_type_hint: Option<String>,
}

impl Validatable for IntrospectRequest {
    fn validate(&self) -> Result<(), ValidationException> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| ValidationException::new().add("token", "The token field is required"))?
            .trim();

        if token.is_empty() {
            return Err(ValidationException::new().add("token", "The token field is required"));
        }

        if token.len() < 10 {
            return Err(
                ValidationException::new().add("token", "The token must be at least 10 characters")
            );
        }

        if let Some(hint) = &self.token_type_hint {
            if hint != "access_token" && hint != "refresh_token" {
                return Err(ValidationException::new().add(
                    "token_type_hint",
                    "Must be 'access_token' or 'refresh_token'",
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize)]
struct IntrospectResponse {
    pub active: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>, // Convert from scopes array (join with space)

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<String>>, // Original Laravel format

    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub iat: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub nbf: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub aud: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<i64>,
}

fn invalid_token() -> Response {
    (
        StatusCode::OK,
        Json(IntrospectResponse {
            active: false,
            scope: None,
            scopes: None,
            client_id: None,
            username: None,
            token_type: None,
            exp: None,
            iat: None,
            nbf: None,
            sub: None,
            aud: None,
            iss: None,
            jti: None,
            user_id: None,
        }),
    )
        .into_response()
}

pub async fn form_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(payload): Form<IntrospectRequest>,
) -> Response {
    token_handler(state, headers, payload).await
}

pub async fn json_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<IntrospectRequest>,
) -> Response {
    token_handler(state, headers, payload).await
}

pub async fn token_handler(
    state: Arc<AppState>,
    headers: HeaderMap,
    request: IntrospectRequest,
) -> Response {
    if let Err(validation) = request.validate() {
        return validation.into_response();
    }

    let token = match request.token {
        Some(token) => token,
        None => {
            return ValidationException::new()
                .add("token", "The token field is required")
                .into_response();
        }
    };

    let client_id = headers
        .get("X-Client-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let claims = match state.token_cache.get(&token) {
        Some(claims) => {
            debug!("Token cache hit (valid)");
            claims
        }
        None => {
            debug!("Token cache miss, validating JWT");

            let claims = match validate_jwt(&token, client_id) {
                Ok(c) => c,
                Err(e) => {
                    debug!("{}", e);
                    state.token_cache.put_invalid(&token);
                    return invalid_token();
                }
            };

            match state.access_tokens.is_token_revoked(&claims.jti).await {
                Ok(false) => {
                    state.token_cache.put_valid(&token, claims.clone());

                    claims
                }
                Ok(true) => {
                    state.token_cache.put_invalid(&token);

                    return invalid_token();
                }
                Err(e) => {
                    error!("Database error while checking token: {}", e);
                    return ValidationException::new()
                        .with_status(StatusCode::INTERNAL_SERVER_ERROR)
                        .with_message("Something went wrong.")
                        .into_response();
                }
            }
        }
    };

    (
        StatusCode::OK,
        Json(IntrospectResponse {
            active: true,
            scope: claims.scopes.as_ref().map(|s| s.join(" ")),
            scopes: claims.scopes,
            client_id: claims.client_id,
            username: claims.username,
            token_type: Some("Bearer".to_string()),
            exp: Some(claims.exp),
            iat: Some(claims.iat),
            nbf: Some(claims.nbf),
            sub: Some(claims.sub),
            aud: claims.aud,
            iss: claims.iss,
            jti: Some(claims.jti),
            user_id: claims.user_id,
        }),
    )
        .into_response()
}
