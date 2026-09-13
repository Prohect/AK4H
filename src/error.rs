//! Kernel errors.

use crate::event::Diagnostic;
use crate::session::SessionId;
use crate::tool::{ToolCallId, ToolInputError};

/// The kernel result type.
pub type Result<T> = std::result::Result<T, Error>;

/// A kernel error.
///
/// Fatal variants end the session; recoverable variants describe a contract
/// violation by the application and leave the session usable.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    // ---- fatal ----
    #[error("session {session:?} exhausted its SDAVE recovery escape limit after {attempts} attempts")]
    RecoveryLimitExceeded {
        session: SessionId,
        attempts: u32,
        diagnostics: Vec<Diagnostic>,
    },
    #[error("session {session:?} is poisoned: {reason}")]
    SessionPoisoned { session: SessionId, reason: String },
    #[error("unknown session {0:?}")]
    UnknownSession(SessionId),

    // ---- recoverable misuse ----
    #[error("a POST is already in flight for session {session:?}")]
    PostAlreadyInFlight { session: SessionId },
    #[error("no POST is in flight for session {session:?}")]
    NoPostInFlight { session: SessionId },
    #[error("tool call {id:?} is not known")]
    UnknownToolCall { id: ToolCallId },
    #[error("tool call {id:?} has already settled")]
    ToolCallAlreadySettled { id: ToolCallId },
    #[error("the embedding application mutated a SDAVE envelope")]
    EnvelopeMutation {
        diagnostics: Vec<Diagnostic>,
    },
    #[error("invalid session history: {0}")]
    InvalidHistory(String),

    #[error(transparent)]
    ToolInput(#[from] ToolInputError),

    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Whether this error is fatal to the session.
    pub fn is_fatal(&self) -> bool {
        todo!()
    }
}
