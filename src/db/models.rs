use std::path::Path;

use anyhow::Context;
use diesel::{prelude::*, sqlite::Sqlite};

use crate::db::{
    connection,
    schema::{downloaded_bottles, linked_files},
};

#[derive(Queryable, Selectable)]
#[diesel(table_name = downloaded_bottles)]
#[diesel(check_for_backend(Sqlite))]
pub struct DownloadedBottle {
    pub id: i32,
    pub name: String,
    pub version: String,
    pub path: String,
}

#[derive(Insertable)]
#[diesel(table_name = downloaded_bottles)]
#[diesel(check_for_backend(Sqlite))]
struct NewDownloadedBottle<'a> {
    name: &'a str,
    version: &'a str,
    path: &'a str,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = linked_files)]
#[diesel(check_for_backend(Sqlite))]
pub struct LinkedFile {
    pub id: i32,
    pub path: String,
    pub bottle_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = linked_files)]
#[diesel(check_for_backend(Sqlite))]
struct NewLinkedFile<'a> {
    path: &'a str,
    bottle_id: i32,
}

impl DownloadedBottle {
    pub fn create(name: &str, version: &str, path: &Path) -> anyhow::Result<DownloadedBottle> {
        let mut db = connection()?.lock().unwrap();

        let result = diesel::insert_into(downloaded_bottles::table)
            .values(NewDownloadedBottle {
                name,
                version,
                path: path.to_str().context("Installed bottle path is non-utf8")?,
            })
            .returning(DownloadedBottle::as_returning())
            .get_result(&mut *db)?;

        Ok(result)
    }

    pub fn get(name: &str, version: &str) -> anyhow::Result<Option<DownloadedBottle>> {
        use downloaded_bottles::dsl;

        let mut db = connection()?.lock().unwrap();
        let name_value = name;
        let version_value = version;

        let mut results = dsl::downloaded_bottles
            .filter(dsl::name.eq(name_value))
            .filter(dsl::version.eq(version_value))
            .select(DownloadedBottle::as_select())
            .load_iter(&mut *db)?;

        if let Some(bottle) = results.next() {
            Ok(Some(bottle?))
        } else {
            Ok(None)
        }
    }
}

impl LinkedFile {
    pub fn create(path: &Path, bottle: &DownloadedBottle) -> anyhow::Result<()> {
        let mut db = connection()?.lock().unwrap();

        diesel::insert_into(linked_files::table)
            .values(NewLinkedFile {
                path: path.to_str().context("Linked file path is non-utf8")?,
                bottle_id: bottle.id,
            })
            .on_conflict(linked_files::dsl::path)
            .do_nothing()
            .execute(&mut *db)?;

        Ok(())
    }
}
