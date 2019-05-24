use crate::{
    CommitTransaction, DefaultQueries, MigrateMultiple, MigrateSingle, MigrationError,
    MigrationVersion, Query, Transaction, WrapTransactionError, ExecuteMultiple
};
use chrono::{DateTime, Local};
use mysql::{
    error::Error, params::Params, Conn, IsolationLevel, PooledConn, Transaction as MTransaction,
};

fn query_migration_version(transaction: &mut MTransaction, query: &str) -> Result<Option<MigrationVersion>, Error> {
    let rows = transaction.query(query)?;
    match rows.into_iter().next() {
        None => Ok(None),
        Some(Ok(row)) => {
            let version: i64 = row.get(0).unwrap();
            let _installed_on: String = row.get(2).unwrap();
            let installed_on = DateTime::parse_from_rfc3339(&_installed_on)
                .unwrap()
                .with_timezone(&Local);

            Ok(Some(MigrationVersion {
                version: version as usize,
                name: row.get(1).unwrap(),
                installed_on,
                checksum: row.get(3).unwrap(),
            }))
        }
        Some(Err(err)) => Err(err),
    }
}

impl<'a> Transaction for MTransaction<'a> {
    type Error = Error;

    fn execute(&mut self, query: &str) -> Result<usize, Self::Error> {
        let count = self.first_exec(query, Params::Empty)?;
        Ok(count.unwrap_or(0) as usize)
    }
}

impl<'a> CommitTransaction for MTransaction<'a> {
    fn commit(self) -> Result<(), Self::Error> {
        MTransaction::commit(self)
    }
}

impl<'a> Query<MigrationVersion> for MTransaction<'a> {
    fn query(&mut self, query: &str) -> Result<Option<MigrationVersion>, Self::Error> {
        query_migration_version(self, query)
    }
}

impl<'a> DefaultQueries for MTransaction<'a> {}

impl<'a> MigrateSingle<'a> for Conn {
    type Transaction = MTransaction<'a>;

    fn transaction(&'a mut self) -> Result<MTransaction<'a>, MigrationError> {
        self.start_transaction(true, Some(IsolationLevel::RepeatableRead), None)
            .transaction_err("error starting transaction")
    }
}


impl<'a> MigrateSingle<'a> for PooledConn {
    type Transaction = MTransaction<'a>;

    fn transaction(&'a mut self) -> Result<MTransaction<'a>, MigrationError> {
        self.start_transaction(true, Some(IsolationLevel::RepeatableRead), None)
            .transaction_err("error starting transaction")
    }
}

impl Transaction for Conn {
    type Error = Error;

    fn execute(&mut self, query: &str) -> Result<usize, Self::Error> {
        let mut transaction = self.start_transaction(true, Some(IsolationLevel::RepeatableRead), None)?;
        let count = transaction.first_exec(query, Params::Empty)?;
        transaction.commit()?;
        Ok(count.unwrap_or(0) as usize)
    }
}

impl Transaction for PooledConn {
    type Error = Error;

    fn execute(&mut self, query: &str) -> Result<usize, Self::Error> {
        let mut transaction = self.start_transaction(true, Some(IsolationLevel::RepeatableRead), None)?;
        let count = transaction.first_exec(query, Params::Empty)?;
        transaction.commit()?;
        Ok(count.unwrap_or(0) as usize)
    }
}

impl ExecuteMultiple for Conn {
    fn execute_multiple(&mut self, queries: &[&str]) -> Result<usize, Self::Error> {
        let mut transaction = self.start_transaction(true, Some(IsolationLevel::RepeatableRead), None)?;
        let mut count = 0;
        for query in queries.iter() {
            count += transaction.first_exec(query, Params::Empty)?.unwrap_or(0);
        }
        transaction.commit()?;
        Ok(count as usize)
    }
}

impl ExecuteMultiple for PooledConn {
    fn execute_multiple(&mut self, queries: &[&str]) -> Result<usize, Self::Error> {
        let mut transaction = self.start_transaction(true, Some(IsolationLevel::RepeatableRead), None)?;
        let mut count = 0;
        for query in queries.iter() {
            count += transaction.first_exec(query, Params::Empty)?.unwrap_or(0);
        }
        transaction.commit()?;
        Ok(count as usize)
    }
}

impl Query<MigrationVersion> for Conn {
    fn query(&mut self, query: &str) -> Result<Option<MigrationVersion>, Self::Error> {
        let mut transaction = self.start_transaction(true, Some(IsolationLevel::RepeatableRead), None)?;
        let version = query_migration_version(&mut transaction, query)?;
        transaction.commit()?;
        Ok(version)
    }
}

impl Query<MigrationVersion> for PooledConn {
    fn query(&mut self, query: &str) -> Result<Option<MigrationVersion>, Self::Error> {
        let mut transaction = self.start_transaction(true, Some(IsolationLevel::RepeatableRead), None)?;
        let version = query_migration_version(&mut transaction, query)?;
        transaction.commit()?;
        Ok(version)
    }
}

impl DefaultQueries for Conn {}

impl DefaultQueries for PooledConn {}

impl MigrateMultiple for Conn {}

impl MigrateMultiple for PooledConn {}
