use axum::{Json,Router,body::Body,extract::{DefaultBodyLimit,Request,State},http::{StatusCode,HeaderValue},middleware::{self,Next},response::Response,routing::get};
use serde_json::json;
use crate::{app::config::Config,modules::notes,platform::{access,db::Database,http::Error,mail::Mailer}};

pub fn router(config:Config,database:Database,mailer:Mailer)->Router {
    let access=access::Service::new(database.clone(),config,mailer);
    Router::new()
        .route("/health/live",get(||async{Json(json!({"status":"ok"}))}))
        .route("/health/ready",get(ready).with_state(database.clone()))
        .merge(access::router(access.clone()))
        .merge(notes::router(database,access))
        .fallback(||async{Error::NotFound})
        .layer(DefaultBodyLimit::max(65_536))
        .layer(tower_http::timeout::TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT,std::time::Duration::from_secs(20)))
        .layer(middleware::from_fn(boundaries))
}
async fn ready(State(db):State<Database>)->Result<Json<serde_json::Value>,Error> {
    db.ready().await?; Ok(Json(json!({"status":"ready"})))
}
async fn boundaries(request:Request<Body>,next:Next)->Response {
    let id=uuid::Uuid::new_v4().to_string();
    let mut response=next.run(request).await;
    let headers=response.headers_mut();
    headers.insert("x-request-id",HeaderValue::from_str(&id).unwrap());
    headers.insert("cache-control",HeaderValue::from_static("no-store"));
    headers.insert("x-content-type-options",HeaderValue::from_static("nosniff"));
    headers.insert("referrer-policy",HeaderValue::from_static("no-referrer"));
    headers.insert("x-frame-options",HeaderValue::from_static("DENY"));
    response
}
