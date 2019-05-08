use regex::Regex;
use std::cmp::Ordering;
use std::error::Error;

#[cfg(feature = "rusqlite")]
pub mod sqlite;

pub fn file_match_re() -> Regex {
    Regex::new(r"([V])([\d|\.]+)__(\w+)").unwrap()
}

lazy_static::lazy_static! {
    static ref RE: regex::Regex = file_match_re();
}

#[derive(Clone, Debug)]
pub struct Migration {
    name: String,
    version: usize,
    sql: String,
}

#[derive(Debug)]
pub struct MigrationError {
    msg: String,
    kind: MigrationErrorKind,
    cause: Option<Box<dyn Error + Sync + Send>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationErrorKind {
    InvalidName,
    InvalidVersion,
    SqlError,
}

impl Migration {
    pub fn new(name: &str, sql: &str) -> Result<Migration, MigrationError> {
        let captures = RE
            .captures(name)
            .filter(|caps| caps.len() == 4)
            .ok_or(MigrationError {
                msg: format!(
                    "{}: migration name must be in the format V{{number}}__{{name}}",
                    name
                ),
                kind: MigrationErrorKind::InvalidName,
                cause: None,
            })?;
        let version = captures[2].parse().map_err(|_| MigrationError {
            msg: format!("{:?}: migration number must be a valid integer", captures),
            kind: MigrationErrorKind::InvalidVersion,
            cause: None,
        })?;

        let name = (&captures[3]).into();
        Ok(Migration {
            name,
            version,
            sql: sql.into(),
        })
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

#[allow(dead_code)]
#[derive(Debug)]
pub struct MigrationMeta {
    name: String,
    version: usize,
}

pub trait Transaction {
    type Error;

    fn execute(&mut self, query: &str) -> Result<usize, Self::Error>;

    fn get_migration_meta(&mut self, query: &str) -> Result<Option<MigrationMeta>, Self::Error>;

    fn commit(self) -> Result<(), Self::Error>;
}

pub trait Connection<'a, T>
where
    T: Transaction,
    T::Error: Error + Send + Sync + 'static,
{
    // type Error: From<T::Error>;

    fn assert_migrations_table(transaction: &mut T) -> Result<(), MigrationError> {
        transaction
            .execute(
                "CREATE TABLE IF NOT EXISTS keg_schema_history( \
                 version INTEGER PRIMARY KEY,\
                 name VARCHAR(255),\
                 installed_on DATETIME);",
            )
            .map_err(|err| MigrationError {
                msg: "could not create schema history table".into(),
                kind: MigrationErrorKind::SqlError,
                cause: Some(Box::new(err)),
            })?;
        Ok(())
    }

    fn get_current_version(transaction: &mut T) -> Result<Option<MigrationMeta>, MigrationError> {
        transaction.get_migration_meta("SELECT version, name FROM keg_schema_history where version=(SELECT MAX(version) from keg_schema_history)").map_err(|err| {
            MigrationError {
                msg: "error getting current schema history version".into(),
                kind: MigrationErrorKind::SqlError,
                cause: Some(Box::new(err)),
            }
        })
    }

    fn migrate(&'a mut self, migrations: &[Migration]) -> Result<(), MigrationError> {
        let mut transaction = self.transaction()?;
        Self::assert_migrations_table(&mut transaction)?;
        let current = Self::get_current_version(&mut transaction)?.unwrap_or(MigrationMeta {
            name: "".into(),
            version: 0,
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
                .map_err(|err| MigrationError {
                    msg: format!(
                        "error applying migration V{}__{}",
                        migration.version, migration.name
                    ),
                    kind: MigrationErrorKind::SqlError,
                    cause: Some(Box::new(err)),
                })?;

            transaction
                .execute(&format!(
                    "INSERT INTO keg_schema_history (version, name) VALUES ({}, '{}')",
                    migration.version, migration.name
                ))
                .map_err(|err| MigrationError {
                    msg: format!(
                        "error updating schema history to version: {}",
                        migration.version
                    ),
                    kind: MigrationErrorKind::SqlError,
                    cause: Some(Box::new(err)),
                })?;
        }

        transaction.commit().map_err(|err| MigrationError {
            msg: "error commiting transaction".into(),
            kind: MigrationErrorKind::SqlError,
            cause: Some(Box::new(err)),
        })?;

        Ok(())
    }

    fn transaction(&'a mut self) -> Result<T, MigrationError>;
}
