use super::{Connection, MigrationError, MigrationErrorKind, MigrationMeta, Transaction};
use postgres::{
    transaction::Transaction as PgTransaction, Connection as PgConnection, Error as PgError,
};
use chrono::{DateTime, Local};

impl<'a> Transaction for PgTransaction<'a> {
    type Error = PgError;

    fn execute(&mut self, query: &str) -> Result<usize, Self::Error> {
        let result = PgTransaction::execute(self, query, &[])?;
        Ok(result as usize)
    }

    fn get_migration_meta(&mut self, query: &str) -> Result<Option<MigrationMeta>, Self::Error> {
        let rows = self.query(query, &[])?;
        match rows.into_iter().next() {
            None => Ok(None),
            Some(row) => {
                let version: i64 = row.get(0);
                let _installed_on: String = row.get(2);
                let installed_on = DateTime::parse_from_rfc3339(&_installed_on).unwrap().with_timezone(&Local);

                Ok(Some(MigrationMeta {
                    version: version as usize,
                    name: row.get(1),
                    installed_on
                }))
            }
        }
    }

    fn commit(self) -> Result<(), Self::Error> {
        PgTransaction::commit(self)
    }
}

impl<'a> Connection<'a, PgTransaction<'a>> for PgConnection {
    fn transaction(&'a mut self) -> Result<PgTransaction<'a>, MigrationError> {
        PgConnection::transaction(self).map_err(|err| MigrationError {
            msg: "error starting transaction".into(),
            kind: MigrationErrorKind::SqlError,
            cause: Some(Box::new(err)),
        })
    }
}
