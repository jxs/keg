use std::fmt;

#[derive(Debug)]
pub enum Error {
    InvalidName,
    InvalidVersion,
    TransactionError(String, Box<dyn std::error::Error + Sync + Send>),
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidName => write!(
                fmt,
                "migration name must be in the format V{{number}}__{{name}}"
            )?,
            Error::InvalidVersion => {
                write!(fmt, "migration version must be a valid integer")?
            }
            Error::TransactionError(msg, cause) => {
                write!(fmt, "error applying migration {}, {}", msg, cause)?
            }
        }
        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::TransactionError(_migration, cause) => Some(&**cause),
            _ => None,
        }
    }
}

pub trait WrapMigrationError<T, E> {
    fn migration_err(self, msg: &str) -> Result<T, Error>;
}

impl<T, E> WrapMigrationError<T, E> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn migration_err(self, msg: &str) -> Result<T, Error> {
        self.map_err(|err| Error::TransactionError(msg.into(), Box::new(err)))
    }
}
