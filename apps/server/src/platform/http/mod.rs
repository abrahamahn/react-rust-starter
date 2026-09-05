use axum::{Json, http::StatusCode, response::{IntoResponse, Response}};
use serde_json::json;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    InvalidInput,
    InvalidCredentials,
    Unauthenticated,
    Forbidden,
    Unverified,
    AccountUnavailable,
    InvalidToken,
    NotFound,
    Conflict,
    RateLimited,
    Unavailable,
    Internal,
    Config(&'static str),
    MigrationDrift,
}

impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidInput => "invalid_input",
            Self::InvalidCredentials => "invalid_credentials",
            Self::Unauthenticated => "unauthenticated",
            Self::Forbidden => "origin_rejected",
            Self::Unverified => "email_unverified",
            Self::AccountUnavailable => "account_unavailable",
            Self::InvalidToken => "invalid_or_expired_token",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::RateLimited => "rate_limited",
            Self::Unavailable => "service_unavailable",
            Self::Internal => "internal_error",
            Self::Config(_) => "configuration_error",
            Self::MigrationDrift => "migration_drift",
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(message) => write!(f, "configuration_error: {message}"),
            _ => f.write_str(self.code()),
        }
    }
}
impl std::error::Error for Error {}
impl From<tokio_postgres::Error> for Error {
    fn from(_: tokio_postgres::Error) -> Self { Self::Unavailable }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match self {
            Self::InvalidInput => StatusCode::UNPROCESSABLE_ENTITY,
            Self::InvalidCredentials | Self::Unauthenticated => StatusCode::UNAUTHORIZED,
            Self::Forbidden | Self::Unverified => StatusCode::FORBIDDEN,
            Self::AccountUnavailable | Self::Conflict => StatusCode::CONFLICT,
            Self::InvalidToken => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let mut response = (status, Json(json!({"error": {"code": self.code()}}))).into_response();
        response.headers_mut().insert("cache-control", "no-store".parse().unwrap());
        if status == StatusCode::TOO_MANY_REQUESTS {
            response.headers_mut().insert("retry-after", "300".parse().unwrap());
        }
        response
    }
}
