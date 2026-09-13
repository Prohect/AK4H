//! Outbound events emitted by the kernel.

use crate::request::SessionRequest;
use crate::stream::StopReason;
use crate::tool::{ToolCallId, ToolCallMode, ToolInput, ToolName};

/// An event emitted by the kernel to the application.
///
/// Actionable events ([`PostRequest`](SessionEvent::PostRequest),
/// [`ToolCallDispatch`](SessionEvent::ToolCallDispatch),
/// [`ToolCallCancel`](SessionEvent::ToolCallCancel)) require the application to
/// act; the rest are observations.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum SessionEvent {
    /// Perform the session's single in-flight LLM request.
    PostRequest { request: SessionRequest },
    /// Abort the in-flight request.
    CancelPost,
    /// Execute a tool call.
    ToolCallDispatch {
        id: ToolCallId,
        name: ToolName,
        input: ToolInput,
        mode: ToolCallMode,
    },
    /// Stop executing a dispatched tool call.
    ToolCallCancel { id: ToolCallId },
    /// The turn has ended.
    Stop { reason: StopReason },
    /// A diagnostic notice.
    Notice { notice: Notice },
}

/// A kernel-assigned notice identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NoticeId(pub u64);

/// An out-of-band diagnostic, shown to the application but never part of the
/// session history.
#[derive(Clone, Debug, PartialEq)]
pub struct Notice {
    pub id: NoticeId,
    pub kind: NoticeKind,
    pub severity: NoticeSeverity,
    pub title: String,
    pub description: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
}

/// The severity of a [`Notice`].
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum NoticeSeverity {
    Info,
    Warning,
    Error,
    Other(String),
}

/// The malformed-SDAVE recovery lifecycle, reported as [`Notice`]s.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum NoticeKind {
    /// An incoming envelope failed validation; recovery was entered.
    MalformedSdave { rejection: SdaveRejection },
    /// The last known-good history was snapshotted.
    SnapshotTaken { at_message: usize },
    /// Still awaiting a valid SDAVE; counts toward the escape limit.
    AwaitingValidSdave { attempt: u32, max_attempts: u32 },
    /// The escape limit was exceeded; the kernel resumed.
    RecoveryLimitExceeded { attempts: u32 },
    /// A valid SDAVE arrived and normal processing resumed.
    Recovered { attempts: u32 },
    /// A non-recovery informational diagnostic.
    Info,
}

/// Why an SDAVE envelope was rejected.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum SdaveRejection {
    NotAnEnvelope,
    /// The application mutated a SDAVE envelope.
    EnvelopeMutated,
    UnknownVariant { tag: String },
    SchemaViolation { detail: String },
}

/// One structured diagnostic attached to a [`Notice`].
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    /// A dotted path into the offending envelope, if known.
    pub path: Option<String>,
    pub message: String,
    /// A truncated raw excerpt, if outbound display is meaningful.
    pub raw_excerpt: Option<String>,
}
