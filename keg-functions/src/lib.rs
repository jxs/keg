mod error;
mod traits;

use regex::Regex;
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use chrono::{DateTime, Local};

pub use error::{MigrationError, WrapTransactionError};
pub use traits::{Transaction, DefaultQueries, CommitTransaction, ExecuteMultiple, Query, MigrateSingle, MigrateMultiple};

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

#[derive(Clone, Debug)]
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
        self.name.hash(&mut hasher);
        self.version.hash(&mut hasher);
        self.sql.hash(&mut hasher);
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
pub struct MigrationVersion {
    name: String,
    version: usize,
    installed_on: DateTime<Local>,
    checksum: String,
}

pub struct Runner {
    multiple: bool,
    migrations: Vec<Migration>
}

impl Runner {
    pub fn new(migrations: &[Migration]) -> Runner {
        Runner {
            multiple: true,
            migrations: migrations.to_vec()
        }
    }
    pub fn set_multiple(&mut self, multiple: bool) {
        self.multiple = multiple;
    }

    pub fn run<'a, C>(&self, conn: &'a mut C) -> Result<(), MigrationError> where C: MigrateSingle<'a> + MigrateMultiple {
        if self.multiple {
            MigrateMultiple::migrate(conn, &self.migrations)?;
        } else {
            MigrateSingle::migrate(conn, &self.migrations)?;
        }
        Ok(())
    }
}
