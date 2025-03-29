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
    id: i32,
    name: String,
    version: String,
    path: String,
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
    id: i32,
    path: String,
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

    pub fn get_all() -> anyhow::Result<Vec<DownloadedBottle>> {
        use downloaded_bottles::dsl;

        let mut db = connection()?.lock().unwrap();

        let results = dsl::downloaded_bottles
            .order((dsl::name, dsl::version))
            .select(DownloadedBottle::as_select())
            .load(&mut *db)?;

        Ok(results)
    }

    pub fn delete(&self) -> anyhow::Result<()> {
        use downloaded_bottles::dsl;

        let mut db = connection()?.lock().unwrap();

        diesel::delete(downloaded_bottles::table)
            .filter(dsl::id.eq(self.id))
            .execute(&mut *db)?;

        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn path(&self) -> &Path {
        self.path.as_ref()
    }

    pub fn linked_files(&self) -> anyhow::Result<Vec<LinkedFile>> {
        use linked_files::dsl;

        let mut db = connection()?.lock().unwrap();

        let results = dsl::linked_files
            .filter(dsl::bottle_id.eq(self.id))
            .select(LinkedFile::as_select())
            .load(&mut *db)?;

        Ok(results)
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

    pub fn delete(&self) -> anyhow::Result<()> {
        use linked_files::dsl;

        let mut db = connection()?.lock().unwrap();

        diesel::delete(linked_files::table)
            .filter(dsl::id.eq(self.id))
            .execute(&mut *db)?;

        Ok(())
    }

    pub fn path(&self) -> &Path {
        self.path.as_ref()
    }
}
