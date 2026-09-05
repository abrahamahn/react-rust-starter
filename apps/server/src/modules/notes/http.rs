use axum::{Json,Router,extract::{State,Path,rejection::JsonRejection},http::{HeaderMap,StatusCode},routing::{get,put}};
use crate::platform::{access::{Service,authorize},db::Database,http::Error};
use super::{domain::*,store};
#[derive(Clone)]
struct StateData { database:Database,access:Service }
pub fn router(database:Database,access:Service)->Router {
    Router::new().route("/api/notes",get(list).post(create)).route("/api/notes/{id}",put(update).delete(delete)).with_state(StateData{database,access})
}
async fn list(State(s):State<StateData>,headers:HeaderMap)->Result<Json<Vec<Note>>,Error> {
    let digest=s.access.session_digest(&headers)?;
    let mut client=s.database.client().await?; let tx=client.transaction().await?;
    let session=authorize(&tx,&digest,true).await?;
    let result=store::list(&tx,&session.user.id).await?; tx.commit().await?; Ok(Json(result))
}
async fn create(State(s):State<StateData>,headers:HeaderMap,input:Result<Json<WriteNote>,JsonRejection>)->Result<(StatusCode,Json<Note>),Error> {
    s.access.check_origin(&headers)?;
    let input=input.map_err(|_|Error::InvalidInput)?.0.validate(false)?;
    let digest=s.access.session_digest(&headers)?;
    let mut client=s.database.client().await?; let tx=client.transaction().await?;
    let session=authorize(&tx,&digest,true).await?;
    let result=store::create(&tx,&session.user.id,&input).await?; tx.commit().await?; Ok((StatusCode::CREATED,Json(result)))
}
async fn update(State(s):State<StateData>,Path(id):Path<String>,headers:HeaderMap,input:Result<Json<WriteNote>,JsonRejection>)->Result<Json<Note>,Error> {
    s.access.check_origin(&headers)?;
    uuid::Uuid::parse_str(&id).map_err(|_|Error::NotFound)?;
    let input=input.map_err(|_|Error::InvalidInput)?.0.validate(true)?;
    let digest=s.access.session_digest(&headers)?;
    let mut client=s.database.client().await?; let tx=client.transaction().await?;
    let session=authorize(&tx,&digest,true).await?;
    let result=store::update(&tx,&session.user.id,&id,&input).await?; tx.commit().await?; Ok(Json(result))
}
async fn delete(State(s):State<StateData>,Path(id):Path<String>,headers:HeaderMap,input:Result<Json<DeleteNote>,JsonRejection>)->Result<StatusCode,Error> {
    s.access.check_origin(&headers)?;
    uuid::Uuid::parse_str(&id).map_err(|_|Error::NotFound)?;
    let input=input.map_err(|_|Error::InvalidInput)?.0;
    if input.revision<1 {return Err(Error::InvalidInput);}
    let digest=s.access.session_digest(&headers)?;
    let mut client=s.database.client().await?; let tx=client.transaction().await?;
    let session=authorize(&tx,&digest,true).await?;
    store::delete(&tx,&session.user.id,&id,input.revision).await?; tx.commit().await?; Ok(StatusCode::NO_CONTENT)
}
