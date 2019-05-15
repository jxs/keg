mod error;

use chrono::{DateTime, Local};
use regex::Regex;
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::fmt;
use std::hash::{Hash, Hasher};

pub use error::{MigrationError, WrapTransactionError};

#[cfg(feature = "rusqlite")]
pub mod rusqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "mysql")]
pub mod mysql;

pub fn file_match_re() -> Regex {
    Regex::new(r"([V])([\d|\.]+)__(\w+)").unwrap()
}

lazy_static::lazy_static! {
    static ref RE: regex::Regex = file_match_re();
}

#[derive(Clone, Debug, Hash)]
pub struct Migration {
    name: String,
    version: usize,
    sql: String,
}

impl Migration {
    pub fn new(name: &str, sql: &str) -> Result<Migration, MigrationError> {
        let captures = RE
            .captures(name)
            .filter(|caps| caps.len() == 4)
            .ok_or(MigrationError::InvalidName)?;
        let version = captures[2]
            .parse()
            .map_err(|_| MigrationError::InvalidVersion)?;

        let name = (&captures[3]).into();
        Ok(Migration {
            name,
            version,
            sql: sql.into(),
        })
    }

    pub fn checksum(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl fmt::Display for Migration {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "V{}__{}", self.version, self.name)
    }
}

impl Eq for Migration {}

impl Ord for Migration {
    fn cmp(&self, other: &Migration) -> Ordering {
        self.version.cmp(&other.version)
    }
}

impl PartialEq for Migration {
    fn eq(&self, other: &Migration) -> bool {
        self.version == other.version
    }
}

impl PartialOrd for Migration {
    fn partial_cmp(&self, other: &Migration) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
pub struct MigrationMeta {
    name: String,
    version: usize,
    installed_on: DateTime<Local>,
    checksum: String,
}

pub trait Transaction {
    type Error: Error + Send + Sync + 'static;

    fn execute(&mut self, query: &str) -> Result<usize, Self::Error>;

    fn get_migration_meta(&mut self, query: &str) -> Result<Option<MigrationMeta>, Self::Error>;

    fn commit(self) -> Result<(), Self::Error>;
}

pub trait Connection<'a, T>
where
    T: Transaction,
{
    fn assert_migrations_table(transaction: &mut T) -> Result<(), MigrationError> {
        transaction
            .execute(
                "CREATE TABLE IF NOT EXISTS keg_schema_history( \
                 version INTEGER PRIMARY KEY,\
                 name VARCHAR(255),\
                 installed_on VARCHAR(255),
                 checksum VARCHAR(255));",
            )
            .transaction_err("error creating schema history table")?;
        Ok(())
    }

    fn get_current_version(transaction: &mut T) -> Result<Option<MigrationMeta>, MigrationError> {
        transaction
            .get_migration_meta(
                "SELECT version, name, installed_on, checksum FROM keg_schema_history where version=(SELECT MAX(version) from keg_schema_history)",
            )
            .transaction_err("error getting current schema history version")
    }

    fn migrate(&'a mut self, migrations: &[Migration]) -> Result<(), MigrationError> {
        let mut transaction = self.transaction()?;
        Self::assert_migrations_table(&mut transaction)?;
        let current = Self::get_current_version(&mut transaction)?.unwrap_or(MigrationMeta {
            name: "".into(),
            version: 0,
            installed_on: Local::now(),
            checksum: "".into(),
        });
        log::debug!("current migration: {}", current.version);
        let mut migrations = migrations
            .iter()
            .filter(|m| m.version > current.version)
            .collect::<Vec<_>>();
        migrations.sort();

        if migrations.is_empty() {
            log::debug!("no migrations to apply");
        }

        for migration in migrations.iter() {
            log::debug!("applying migration: {}", migration.name);
            transaction
                .execute(&migration.sql)
                .transaction_err(&format!("error applying migration {}", migration))?;

            transaction
                .execute(&format!(
                    "INSERT INTO keg_schema_history (version, name, installed_on, checksum) VALUES ({}, '{}', '{}', '{}')",
                    migration.version, migration.name, Local::now().to_rfc3339(), migration.checksum().to_string()
                ))
                .transaction_err(&format!("error updating schema history to migration: {}", migration))?;
        }

        transaction
            .commit()
            .transaction_err("error committing transaction")?;

        Ok(())
    }

    fn transaction(&'a mut self) -> Result<T, MigrationError>;
}
