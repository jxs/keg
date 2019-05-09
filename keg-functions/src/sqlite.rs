use super::{Connection, MigrationError, MigrationErrorKind, MigrationMeta, Transaction};
use rusqlite::{
    Connection as RqlConnection, Error as RqlError, OptionalExtension,
    Transaction as RqlTransaction, NO_PARAMS,
};
// use core::ops::Deref;

impl<'a> Transaction for RqlTransaction<'a> {
    type Error = RqlError;

    fn execute(&mut self, query: &str) -> Result<usize, Self::Error> {
        RqlConnection::execute(self, query, NO_PARAMS)
    }

    fn get_migration_meta(&mut self, query: &str) -> Result<Option<MigrationMeta>, Self::Error> {
        self.query_row(query, NO_PARAMS, |row| {
            //FromSql not implemented for usize
            let version: isize = row.get(0)?;
            Ok(MigrationMeta {
                version: version as usize,
                name: row.get(1)?,
            })
        })
        .optional()
    }

    fn commit(self) -> Result<(), Self::Error> {
        RqlTransaction::commit(self)
    }
}

impl<'a> Connection<'a, RqlTransaction<'a>> for RqlConnection {
    fn transaction(&'a mut self) -> Result<RqlTransaction<'a>, MigrationError> {
        self.transaction().map_err(|err| MigrationError {
            msg: "error starting transaction".into(),
            kind: MigrationErrorKind::SqlError,
            cause: Some(Box::new(err)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Connection as _;
    use crate::Migration;
    use rusqlite::{Connection, NO_PARAMS};

    #[test]
    fn creates_migration_table() {
        let mut conn = Connection::open_in_memory().unwrap();
        let result = conn.migrate(&Vec::new());
        assert!(result.is_ok());
        let table_name: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='keg_schema_history'",
                NO_PARAMS,
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!("keg_schema_history", table_name);
    }

    #[test]
    fn applies_migration() {
        let mut conn = Connection::open_in_memory().unwrap();
        let sql = concat! {
            "CREATE TABLE artists(",
            "id int,",
            "name varchar(255),",
            "age int)"
        };
        let m = Migration::new("V1__initial", sql).unwrap();
        let migrations = [m];
        conn.migrate(&migrations).unwrap();

        conn.execute(
            "INSERT INTO artists (name, age) VALUES (?, ?)",
            &[&"John Legend", &"36"],
        )
        .unwrap();
        let (name, age): (String, u32) = conn
            .query_row("SELECT name, age FROM artists", NO_PARAMS, |row| {
                Ok((row.get(0).unwrap(), row.get(1).unwrap()))
            })
            .unwrap();
        assert_eq!("John Legend", name);
        assert_eq!(36, age);
    }

    #[test]
    fn updates_schema_history() {
        let mut conn = Connection::open_in_memory().unwrap();
        let sql = concat! {
            "CREATE TABLE artists(",
            "id int,",
            "name varchar(255),",
            "age int)"
        };
        let m = Migration::new("V1__initial", sql).unwrap();
        let migrations = [m];
        conn.migrate(&migrations).unwrap();

        conn.execute(
            "INSERT INTO artists (name, age) VALUES (?, ?)",
            &[&"John Legend", &"36"],
        )
        .unwrap();

        let current: u32 = conn
            .query_row("SELECT version FROM keg_schema_history", NO_PARAMS, |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(1, current);
    }

    #[test]
    fn applies_migrations_sorted() {
        let mut conn = Connection::open_in_memory().unwrap();
        let sql = concat! {
            "CREATE TABLE artists(",
            "id int,",
            "name varchar(255),",
            "age int)"
        };
        let m = Migration::new("V1__initial", sql).unwrap();

        let sql2 = "ALTER TABLE artists ADD country varchar (255)";
        let m2 = Migration::new("V2__add_country_field_to_artists", sql2).unwrap();
        let migrations = [m, m2];
        conn.migrate(&migrations).unwrap();

        conn.execute(
            "INSERT INTO artists (name, age, country) VALUES (?, ?, ?)",
            &[&"John Legend", &"36", "United States"],
        )
        .unwrap();

        let country: String = conn
            .query_row(
                "SELECT country FROM artists where name = 'John Legend'",
                NO_PARAMS,
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!("United States", country);
    }
}
