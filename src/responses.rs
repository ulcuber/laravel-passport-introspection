use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::validation::ValidationException;

pub fn unauthorized_response() -> Response {
    ValidationException::new()
        .with_status(StatusCode::UNAUTHORIZED)
        .with_message("Unauthorized.")
        .into_response()
}

pub fn server_error_response() -> Response {
    ValidationException::new()
        .with_status(StatusCode::INTERNAL_SERVER_ERROR)
        .with_message("Something went wrong.")
        .into_response()
}
