use super::{Connection, MigrationError, MigrationErrorKind, MigrationMeta, Transaction};
use mysql::{error::Error, params::Params, Conn, IsolationLevel, Transaction as MTransaction};

impl<'a> Transaction for MTransaction<'a> {
    type Error = Error;

    fn execute(&mut self, query: &str) -> Result<usize, Self::Error> {
        let result = self.first_exec(query, Params::Empty)?;
        Ok(result.unwrap_or(0) as usize)
    }

    fn get_migration_meta(&mut self, query: &str) -> Result<Option<MigrationMeta>, Self::Error> {
        let rows = self.query(query)?;
        match rows.into_iter().next() {
            None => Ok(None),
            Some(Ok(row)) => {
                let version: i64 = row.get(0).unwrap();
                Ok(Some(MigrationMeta {
                    version: version as usize,
                    name: row.get(1).unwrap(),
                }))
            }
            Some(Err(err)) => Err(err),
        }
    }

    fn commit(self) -> Result<(), Self::Error> {
        MTransaction::commit(self)
    }
}

impl<'a> Connection<'a, MTransaction<'a>> for Conn {
    fn transaction(&'a mut self) -> Result<MTransaction<'a>, MigrationError> {
        self.start_transaction(true, Some(IsolationLevel::RepeatableRead), Some(false))
            .map_err(|err| MigrationError {
                msg: "error starting transaction".into(),
                kind: MigrationErrorKind::SqlError,
                cause: Some(Box::new(err)),
            })
    }
}
