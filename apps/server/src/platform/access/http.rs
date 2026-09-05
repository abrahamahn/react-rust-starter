use super::{Service, domain::*, session_policy};
use crate::platform::http::Error;
use axum::{
    Json, Router,
    extract::{State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
};

pub fn router(service: Service) -> Router {
    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/session", get(session))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/logout-all", post(logout_all))
        .route("/api/auth/request-verification", post(request_verification))
        .route("/api/auth/verify", post(confirm))
        .route("/api/auth/forgot-password", post(forgot))
        .route("/api/auth/reset-password", post(reset))
        .route("/api/auth/change-password", post(change))
        .route("/api/account", patch(profile))
        .with_state(service)
}
fn body<T>(value: Result<Json<T>, JsonRejection>) -> Result<T, Error> {
    value.map(|Json(v)| v).map_err(|_| Error::InvalidInput)
}
fn cookie(service: &Service, token: &str, seconds: i64) -> String {
    format!(
        "{}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}{}",
        service.config.cookie_name(),
        token,
        seconds,
        if service.config.production {
            "; Secure"
        } else {
            ""
        }
    )
}
fn signed_in(
    service: &Service,
    value: (SessionView, String, bool),
    status: StatusCode,
) -> Response {
    let (view, token, remember) = value;
    let mut response = (status, Json(view)).into_response();
    response.headers_mut().insert(
        "set-cookie",
        cookie(service, &token, session_policy::span_seconds(remember))
            .parse()
            .unwrap(),
    );
    response
}
fn signed_out(service: &Service) -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    response
        .headers_mut()
        .insert("set-cookie", cookie(service, "", 0).parse().unwrap());
    response
}
async fn register(
    State(s): State<Service>,
    headers: HeaderMap,
    input: Result<Json<Credentials>, JsonRejection>,
) -> Result<Response, Error> {
    s.check_origin(&headers)?;
    Ok(signed_in(
        &s,
        s.register(body(input)?).await?,
        StatusCode::CREATED,
    ))
}
async fn login(
    State(s): State<Service>,
    headers: HeaderMap,
    input: Result<Json<Credentials>, JsonRejection>,
) -> Result<Response, Error> {
    s.check_origin(&headers)?;
    Ok(signed_in(
        &s,
        s.login(body(input)?, s.session_token(&headers)).await?,
        StatusCode::OK,
    ))
}
async fn session(State(s): State<Service>, headers: HeaderMap) -> Result<Json<SessionView>, Error> {
    Ok(Json(s.store.current(&s.session_digest(&headers)?).await?))
}
async fn logout(State(s): State<Service>, headers: HeaderMap) -> Result<Response, Error> {
    s.check_origin(&headers)?;
    if let Ok(digest) = s.session_digest(&headers) {
        s.store.logout(&digest, false).await?;
    }
    Ok(signed_out(&s))
}
async fn logout_all(State(s): State<Service>, headers: HeaderMap) -> Result<Response, Error> {
    s.check_origin(&headers)?;
    s.store.logout(&s.session_digest(&headers)?, true).await?;
    Ok(signed_out(&s))
}
async fn request_verification(
    State(s): State<Service>,
    headers: HeaderMap,
    input: Result<Json<EmailRequest>, JsonRejection>,
) -> Result<StatusCode, Error> {
    s.check_origin(&headers)?;
    s.request_link(body(input)?, "verify").await?;
    Ok(StatusCode::ACCEPTED)
}
async fn forgot(
    State(s): State<Service>,
    headers: HeaderMap,
    input: Result<Json<EmailRequest>, JsonRejection>,
) -> Result<StatusCode, Error> {
    s.check_origin(&headers)?;
    s.request_link(body(input)?, "reset").await?;
    Ok(StatusCode::ACCEPTED)
}
async fn confirm(
    State(s): State<Service>,
    headers: HeaderMap,
    input: Result<Json<TokenRequest>, JsonRejection>,
) -> Result<StatusCode, Error> {
    s.check_origin(&headers)?;
    s.confirm(body(input)?).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn reset(
    State(s): State<Service>,
    headers: HeaderMap,
    input: Result<Json<ResetRequest>, JsonRejection>,
) -> Result<Response, Error> {
    s.check_origin(&headers)?;
    s.reset(body(input)?).await?;
    Ok(signed_out(&s))
}
async fn change(
    State(s): State<Service>,
    headers: HeaderMap,
    input: Result<Json<ChangePassword>, JsonRejection>,
) -> Result<Response, Error> {
    s.check_origin(&headers)?;
    s.change(&s.session_digest(&headers)?, body(input)?).await?;
    Ok(signed_out(&s))
}
async fn profile(
    State(s): State<Service>,
    headers: HeaderMap,
    input: Result<Json<ProfileRequest>, JsonRejection>,
) -> Result<Json<UserView>, Error> {
    s.check_origin(&headers)?;
    let input = body(input)?.validate()?;
    Ok(Json(
        s.store
            .profile(&s.session_digest(&headers)?, &input)
            .await?,
    ))
}
