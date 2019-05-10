use super::{Connection, MigrationError, MigrationErrorKind, MigrationMeta, Transaction};
use rusqlite::{
    Connection as RqlConnection, Error as RqlError, OptionalExtension,
    Transaction as RqlTransaction, NO_PARAMS,
};

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
