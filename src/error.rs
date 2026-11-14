//! Harness errors. These describe problems with the benchmark itself, never
//! the behaviour of an adapter under test.

use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    /// A filesystem operation the harness itself needed failed.
    Io { context: String, source: io::Error },
    /// A scenario failed static confinement validation and was not run.
    InvalidScenario { id: String, reason: String },
    /// The harness could not build a fixture it had validated.
    Fixture { id: String, reason: String },
    /// The harness observed something it must never see. Fatal.
    Integrity(String),
    /// A bad command-line argument.
    Usage(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { context, source } => write!(f, "{context}: {source}"),
            Error::InvalidScenario { id, reason } => {
                write!(f, "scenario `{id}` failed confinement validation: {reason}")
            }
            Error::Fixture { id, reason } => {
                write!(f, "could not build the fixture for `{id}`: {reason}")
            }
            Error::Integrity(msg) => write!(f, "harness integrity violation: {msg}"),
            Error::Usage(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Attach a short description of what the harness was doing to an I/O error.
pub(crate) trait Context<T> {
    fn context(self, what: &str) -> Result<T, Error>;
}

impl<T> Context<T> for io::Result<T> {
    fn context(self, what: &str) -> Result<T, Error> {
        self.map_err(|source| Error::Io {
            context: what.to_string(),
            source,
        })
    }
}
