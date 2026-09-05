// BSLT checksum-checked migration and pool pattern, adapted to a fresh schema.
use std::time::Duration;
use deadpool_postgres::{Manager, ManagerConfig, Object, Pool, RecyclingMethod, Runtime};
use sha2::{Digest, Sha256};
use tokio_postgres::{NoTls, Row};
use crate::{app::config::Config, platform::http::Error};

const MIGRATION_LOCK: i64 = 0x52525354415254;
const MIGRATIONS: &[(&str, &str)] = &[("0001_starter", include_str!("../../../migrations/0001_starter.sql"))];
#[derive(Clone)]
pub struct Database { pool: Pool }
impl Database {
    pub fn new(config: &Config) -> Result<Self, Error> {
        let manager = Manager::from_config(config.database.clone(), NoTls, ManagerConfig { recycling_method: RecyclingMethod::Verified });
        let pool = Pool::builder(manager).max_size(8).runtime(Runtime::Tokio1)
            .wait_timeout(Some(Duration::from_secs(3))).create_timeout(Some(Duration::from_secs(3)))
            .recycle_timeout(Some(Duration::from_secs(3))).build().map_err(|_| Error::Internal)?;
        Ok(Self { pool })
    }
    pub async fn client(&self) -> Result<Object, Error> { self.pool.get().await.map_err(|_| Error::Unavailable) }
    pub async fn ready(&self) -> Result<(), Error> {
        let client = self.client().await?;
        let exists: bool = client.query_one("SELECT to_regclass('starter.schema_migrations') IS NOT NULL", &[]).await?.get(0);
        if !exists { return Err(Error::MigrationDrift); }
        let rows = client.query("SELECT version,name,checksum FROM starter.schema_migrations ORDER BY version", &[]).await?;
        validate(&rows)?;
        if rows.len() != MIGRATIONS.len() { return Err(Error::MigrationDrift); }
        Ok(())
    }
    pub async fn migrate(&self) -> Result<(), Error> {
        let mut client = self.client().await?;
        let tx = client.transaction().await?;
        tx.batch_execute("SET LOCAL lock_timeout='5s'; SET LOCAL statement_timeout='15s';").await?;
        tx.query_one("SELECT pg_advisory_xact_lock($1)", &[&MIGRATION_LOCK]).await?;
        let exists: bool = tx.query_one("SELECT to_regclass('starter.schema_migrations') IS NOT NULL", &[]).await?.get(0);
        let applied = if exists {
            let rows = tx.query("SELECT version,name,checksum FROM starter.schema_migrations ORDER BY version", &[]).await?;
            validate(&rows)?;
            rows.len()
        } else { 0 };
        for (index, (name, sql)) in MIGRATIONS.iter().enumerate().skip(applied) {
            tx.batch_execute(sql).await?;
            tx.execute("INSERT INTO starter.schema_migrations(version,name,checksum) VALUES($1,$2,$3)", &[&((index + 1) as i64), name, &checksum(sql)]).await?;
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn prune(&self) -> Result<(), Error> {
        let client = self.client().await?;
        client.batch_execute("DELETE FROM starter.sessions WHERE expires_at < now() OR last_seen_at < now() - idle_seconds * interval '1 second'; DELETE FROM starter.auth_tokens WHERE expires_at < now() OR used_at IS NOT NULL;").await?;
        Ok(())
    }
    pub fn close(&self) { self.pool.close(); }
}
fn checksum(sql: &str) -> String { format!("{:x}", Sha256::digest(sql.as_bytes())) }
fn validate(rows: &[Row]) -> Result<(), Error> {
    if rows.is_empty() || rows.len() > MIGRATIONS.len() { return Err(Error::MigrationDrift); }
    for (index, row) in rows.iter().enumerate() {
        let version: i64 = row.try_get("version").map_err(|_| Error::MigrationDrift)?;
        let name: String = row.try_get("name").map_err(|_| Error::MigrationDrift)?;
        let stored: String = row.try_get("checksum").map_err(|_| Error::MigrationDrift)?;
        if version != (index + 1) as i64 || name != MIGRATIONS[index].0 || stored != checksum(MIGRATIONS[index].1) {
            return Err(Error::MigrationDrift);
        }
    }
    Ok(())
}
