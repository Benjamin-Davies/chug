use std::sync::Mutex;

use anyhow::{Context, anyhow};
use diesel::{prelude::*, sql_query};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

use crate::dirs::db_file;

pub mod models;
pub mod schema;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

fn connection() -> anyhow::Result<&'static Mutex<SqliteConnection>> {
    cache!(Mutex<SqliteConnection>).get_or_init(|| {
        let path = db_file()?.to_str().context("DB file path is non-utf8")?;
        let mut db = SqliteConnection::establish(path)?;

        sql_query("PRAGMA foreign_keys = ON;").execute(&mut db)?;

        db.run_pending_migrations(MIGRATIONS)
            .map_err(|e| anyhow!(e))?;

        Ok(Mutex::new(db))
    })
}
