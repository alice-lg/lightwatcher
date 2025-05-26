use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// Error Response
#[derive(Serialize, Clone, Debug)]
struct ErrorResponse {
    code: u16,
    error: String,
}

/// Wrapped Anyhow Error
pub struct Error(anyhow::Error);

/// Implement error conversion
impl<E> From<E> for Error
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

/// Implement IntoResponse for Error
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let code = StatusCode::INTERNAL_SERVER_ERROR;
        let err = ErrorResponse {
            code: code.as_u16(),
            error: format!("{}", self.0),
        };
        let body = serde_json::to_string(&err).unwrap();
        (
            code,
            [(header::CONTENT_TYPE, "application/json")],
            body,
        ).into_response()
    }
}
