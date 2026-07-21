use std::sync::Arc;

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
    body::Body,
};

use crate::responses::unauthorized_response;

pub struct GatewayMiddlewareState {
    pub gateway_secret: Arc<String>,
}

pub async fn handle(
    State(state): State<Arc<GatewayMiddlewareState>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let header_secret = req
        .headers()
        .get("X-Gateway-Secret")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if header_secret.is_none() {
        return unauthorized_response();
    }

    let header_secret = header_secret.unwrap();

    if header_secret != *state.gateway_secret {
        return unauthorized_response();
    }

    next.run(req).await
}
