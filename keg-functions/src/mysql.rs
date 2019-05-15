use super::{Connection, MigrationError, MigrationMeta, Transaction, WrapTransactionError};
use chrono::{DateTime, Local};
use mysql::{
    error::Error, params::Params, Conn, IsolationLevel, PooledConn, Transaction as MTransaction,
};

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
                let _installed_on: String = row.get(2).unwrap();
                let installed_on = DateTime::parse_from_rfc3339(&_installed_on)
                    .unwrap()
                    .with_timezone(&Local);

                Ok(Some(MigrationMeta {
                    version: version as usize,
                    name: row.get(1).unwrap(),
                    installed_on,
                    checksum: row.get(3).unwrap(),
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
        self.start_transaction(true, Some(IsolationLevel::RepeatableRead), None)
            .transaction_err("error starting transaction")
    }
}

impl<'a> Connection<'a, MTransaction<'a>> for PooledConn {
    fn transaction(&'a mut self) -> Result<MTransaction<'a>, MigrationError> {
        self.start_transaction(true, Some(IsolationLevel::RepeatableRead), None)
            .transaction_err("error starting transaction")
    }
}
