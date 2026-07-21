use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde_json::{Map, Value};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct ValidationException {
    status: StatusCode,
    message: String,
    errors: Map<String, Value>,
}

#[derive(Debug, Serialize)]
struct LaravelResponse {
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<Map<String, Value>>,
}

impl ValidationException {
    pub fn new() -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            message: "The given data was invalid.".to_string(),
            errors: Map::new(),
        }
    }

    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    pub fn add(mut self, field: impl Into<String>, message: impl Into<String>) -> Self {
        let field = field.into();
        let message = message.into();

        if let Some(existing) = self.errors.get_mut(&field) {
            // If field already has errors, append to array
            if let Some(array) = existing.as_array_mut() {
                array.push(Value::String(message));
            } else {
                // Convert single string to array
                let messages = vec![existing.clone(), Value::String(message)];
                self.errors.insert(field, Value::Array(messages));
            }
        } else {
            // First error for this field
            self.errors.insert(field, Value::String(message));
        }

        self
    }
}

impl Default for ValidationException {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoResponse for ValidationException {
    fn into_response(self) -> Response {
        let body = LaravelResponse {
            message: self.message,
            errors: (!self.errors.is_empty()).then_some(self.errors)
        };

        (self.status, Json(body)).into_response()
    }
}

pub trait Validatable {
    fn validate(&self) -> Result<(), ValidationException>;
}
