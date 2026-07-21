#![cfg(feature = "proxy")]

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::{StatusCode, Uri},
    response::{Response, IntoResponse},
};
use tracing::debug;

use super::AppState;

pub async fn handler(
    State(state): State<Arc<AppState>>,
    mut req: Request<Body>,
) -> Response {
    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or_default();

    let target = state.routes.route_request(&path);
    let full_target = format!("{}{}", target, query);

    *req.uri_mut() = Uri::try_from(full_target).expect("Invalid target URI");

    match state.client.request(req).await {
        Ok(response) => {
            let (parts, body) = response.into_parts();
            let axum_body = Body::new(body);

            Response::from_parts(parts, axum_body)
        },
        Err(e) => {
            debug!("{}", e);
            (StatusCode::BAD_GATEWAY, "Service Unavailable").into_response()
        }
    }
}
