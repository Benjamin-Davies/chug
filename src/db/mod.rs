use std::sync::Mutex;

use anyhow::{Context, anyhow};
use diesel::prelude::*;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

use crate::dirs::db_file;

pub mod models;
pub mod schema;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn connection() -> anyhow::Result<&'static Mutex<SqliteConnection>> {
    cache!(Mutex<SqliteConnection>).get_or_init(|| {
        let path = db_file()?.to_str().context("DB file path is non-utf8")?;
        let mut db = SqliteConnection::establish(path)?;

        db.run_pending_migrations(MIGRATIONS)
            .map_err(|e| anyhow!(e))?;

        Ok(Mutex::new(db))
    })
}
