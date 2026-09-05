use super::domain::{Note, WriteNote};
use crate::platform::http::Error;
use tokio_postgres::{Row, Transaction};
fn note(row: Row) -> Note {
    Note {
        id: row.get("id"),
        title: row.get("title"),
        body: row.get("body"),
        revision: row.get("revision"),
        updated_at: row.get("updated_at"),
    }
}
pub(super) async fn list(tx: &Transaction<'_>, owner: &str) -> Result<Vec<Note>, Error> {
    let rows=tx.query("SELECT id,title,body,revision,extract(epoch FROM updated_at)::bigint AS updated_at FROM starter.notes WHERE user_id=$1 ORDER BY created_at DESC,id DESC LIMIT 100",&[&owner]).await?;
    Ok(rows.into_iter().map(note).collect())
}
pub(super) async fn create(
    tx: &Transaction<'_>,
    owner: &str,
    input: &WriteNote,
) -> Result<Note, Error> {
    // authorize holds the owner row lock, serializing the count and insertion.
    let count: i64 = tx
        .query_one(
            "SELECT count(*) FROM starter.notes WHERE user_id=$1",
            &[&owner],
        )
        .await?
        .get(0);
    if count >= 100 {
        return Err(Error::Conflict);
    }
    let id = uuid::Uuid::new_v4().to_string();
    Ok(note(tx.query_one("INSERT INTO starter.notes(id,user_id,title,body) VALUES($1,$2,$3,$4) RETURNING id,title,body,revision,extract(epoch FROM updated_at)::bigint AS updated_at",&[&id,&owner,&input.title,&input.body]).await?))
}
pub(super) async fn update(
    tx: &Transaction<'_>,
    owner: &str,
    id: &str,
    input: &WriteNote,
) -> Result<Note, Error> {
    let existing = tx
        .query_opt(
            "SELECT revision FROM starter.notes WHERE id=$1 AND user_id=$2",
            &[&id, &owner],
        )
        .await?
        .ok_or(Error::NotFound)?;
    let revision: i32 = existing.get(0);
    if Some(revision) != input.revision {
        return Err(Error::Conflict);
    }
    Ok(note(tx.query_one("UPDATE starter.notes SET title=$1,body=$2,revision=revision+1,updated_at=now() WHERE id=$3 AND user_id=$4 AND revision=$5 RETURNING id,title,body,revision,extract(epoch FROM updated_at)::bigint AS updated_at",&[&input.title,&input.body,&id,&owner,&revision]).await?))
}
pub(super) async fn delete(
    tx: &Transaction<'_>,
    owner: &str,
    id: &str,
    revision: i32,
) -> Result<(), Error> {
    let existing = tx
        .query_opt(
            "SELECT revision FROM starter.notes WHERE id=$1 AND user_id=$2",
            &[&id, &owner],
        )
        .await?
        .ok_or(Error::NotFound)?;
    if existing.get::<_, i32>(0) != revision {
        return Err(Error::Conflict);
    }
    tx.execute(
        "DELETE FROM starter.notes WHERE id=$1 AND user_id=$2 AND revision=$3",
        &[&id, &owner, &revision],
    )
    .await?;
    Ok(())
}
