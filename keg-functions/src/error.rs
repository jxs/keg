use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum MigrationError {
    InvalidName,
    InvalidVersion,
    TransactionError(String, Box<dyn Error + Sync + Send>),
}

impl fmt::Display for MigrationError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MigrationError::InvalidName => write!(
                fmt,
                "migration name must be in the format V{{number}}__{{name}}"
            )?,
            MigrationError::InvalidVersion => {
                write!(fmt, "migration version must be a valid integer")?
            }
            MigrationError::TransactionError(msg, cause) => {
                write!(fmt, "error applying migration {}, {}", msg, cause)?
            }
        }
        Ok(())
    }
}

impl Error for MigrationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            MigrationError::TransactionError(_migration, cause) => Some(&**cause),
            _ => None,
        }
    }
}

pub trait WrapTransactionError<T, E> {
    fn transaction_err(self, msg: &str) -> Result<T, MigrationError>;
}

impl<T, E> WrapTransactionError<T, E> for Result<T, E>
where
    E: Error + Send + Sync + 'static,
{
    fn transaction_err(self, msg: &str) -> Result<T, MigrationError> {
        self.map_err(|err| MigrationError::TransactionError(msg.into(), Box::new(err)))
    }
}
