//! Tool-call identity and lifecycle.
//!
//! AK4H does **not** own or call the application's tool objects. It indexes each
//! tool call ([`ToolCallId`]), emits
//! [`SessionEvent::ToolCallDispatch`](crate::SessionEvent::ToolCallDispatch),
//! and receives lifecycle events back through
//! [`Session::push_tool_event`](crate::Session::push_tool_event). Persisting
//! tool-execution state is the application's responsibility.

use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::message::{ToolResult, ToolResultContent};

/// Maximum provider-safe tool-name length.
pub const MAX_TOOL_NAME_LENGTH: usize = 64;

/// The model-facing tool name.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ToolName(String);

impl ToolName {
    /// Sanitize `raw` into a provider-safe name (ASCII alphanumeric, `_`, `-`).
    pub fn new(raw: &str) -> Self {
        todo!()
    }

    /// The canonical string.
    pub fn as_str(&self) -> &str {
        todo!()
    }
}

impl fmt::Display for ToolName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl From<&str> for ToolName {
    fn from(s: &str) -> Self {
        todo!()
    }
}

impl From<String> for ToolName {
    fn from(s: String) -> Self {
        todo!()
    }
}

/// A tool's arguments: structured JSON or free-form text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ToolInput {
    Json(serde_json::Value),
    Text(String),
}

impl ToolInput {
    /// Borrow the JSON value, if this input is JSON.
    pub fn as_json(&self) -> Option<&serde_json::Value> {
        todo!()
    }

    /// Consume into a JSON value.
    pub fn into_json(self) -> Result<serde_json::Value, ToolInputError> {
        todo!()
    }

    /// Deserialize the input into `T`.
    pub fn parse<T: serde::de::DeserializeOwned>(&self) -> Result<T, ToolInputError> {
        todo!()
    }

    /// A JSON value suitable for display.
    pub fn to_display_json(&self) -> serde_json::Value {
        todo!()
    }
}

/// An error produced while interpreting a [`ToolInput`].
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ToolInputError {
    #[error("tool input is free-form text, not JSON")]
    NotJson,
    #[error("failed to interpret tool input: {0}")]
    Parse(String),
}

/// Durable identity of one tool call within a session.
///
/// Encoded as `"<ordinal>"` for kernel-originated calls and
/// `"<ordinal>:<llm-id>"` for calls named by the model. The ordinal is a
/// kernel-assigned, session-monotonic counter, so the identity stays unique
/// even when a provider reuses a tool-call id across request cycles.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ToolCallId {
    ordinal: u64,
    llm_id: Option<String>,
}

impl ToolCallId {
    /// A kernel-originated call.
    pub fn kernel(ordinal: u64) -> Self {
        todo!()
    }

    /// A model-originated call carrying the provider's tool-call id.
    pub fn llm(ordinal: u64, llm_id: impl Into<String>) -> Self {
        todo!()
    }

    /// The kernel-assigned ordinal.
    pub fn ordinal(&self) -> u64 {
        todo!()
    }

    /// The provider-issued id, if any.
    pub fn llm_id(&self) -> Option<&str> {
        todo!()
    }
}

impl fmt::Display for ToolCallId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

/// Error returned when a [`ToolCallId`] cannot be parsed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("invalid tool call id")]
pub struct ParseToolCallIdError;

impl FromStr for ToolCallId {
    type Err = ParseToolCallIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

/// A coarse, presentation-only classification of a tool.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Read,
    Edit,
    Move,
    Search,
    Execute,
    Fetch,
    Think,
    Other,
}

/// How the kernel supervises a tool call.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ToolCallMode {
    /// Fire and forget; many may run concurrently.
    Async,
    /// Supervised: if no [`ToolEvent::Message`] arrives within `idle_timeout`,
    /// the kernel drives the call to a terminal outcome.
    Sync { idle_timeout: Duration },
    /// Dispatched only if `assertion` holds at dispatch time. The kernel
    /// evaluates the assertion.
    Guarded { assertion: AssertionExpr },
}

/// A pure, serializable predicate over kernel-owned session state.
///
/// The assertion is part of the persisted session, so it is data rather than a
/// closure the kernel could evaluate.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AssertionExpr {
    Equals {
        lhs: AssertionPath,
        rhs: serde_json::Value,
    },
    Compare {
        lhs: AssertionPath,
        op: CompareOp,
        rhs: serde_json::Value,
    },
    Exists {
        path: AssertionPath,
    },
    /// True iff no committed tool call is still in flight.
    NoInFlightToolCalls,
    All(Vec<AssertionExpr>),
    Any(Vec<AssertionExpr>),
    Not(Box<AssertionExpr>),
}

/// A path into kernel-owned session state.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssertionPath(pub String);

/// A comparison operator for [`AssertionExpr::Compare`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompareOp {
    Lt,
    Le,
    Gt,
    Ge,
}

/// A lifecycle event pushed by the application.
///
/// `Start` is sent for a call dispatched in the current process. After a
/// restart, the application reconciles calls it can still observe with
/// [`Session::pending_tool_calls`](crate::Session::pending_tool_calls) and
/// pushes the remaining [`ToolEvent::Message`] / [`ToolEvent::Finish`] events
/// for an already-committed id.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum ToolEvent {
    Start(ToolCallStart),
    Message(ToolCallMessage),
    Finish(ToolCallFinish),
}

/// The application has begun executing a dispatched call.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolCallStart {
    pub id: ToolCallId,
    pub name: ToolName,
    pub input: ToolInput,
    pub mode: ToolCallMode,
    pub kind: ToolKind,
    /// Presentation-only title.
    pub title: Option<String>,
}

/// An incremental update for a running call.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolCallMessage {
    pub id: ToolCallId,
    /// Kernel-assigned, session-monotonic ordering token.
    pub seq: u64,
    /// Whether this chunk is a fragment rather than a terminal result.
    pub partial: bool,
    pub payload: ToolMessagePayload,
}

/// The payload of a [`ToolCallMessage`].
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum ToolMessagePayload {
    /// A streamed tool-argument fragment.
    InputDelta(ToolInput),
    /// A progress / output chunk.
    Output(ToolResultContent),
    /// A presentation-only field update.
    Fields {
        title: Option<String>,
        kind: Option<ToolKind>,
    },
}

/// The single terminal event for a tool call.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolCallFinish {
    pub id: ToolCallId,
    /// Kernel-assigned, session-monotonic ordering token.
    pub seq: u64,
    pub outcome: ToolOutcome,
}

/// The terminal disposition of a tool call.
///
/// Every variant yields exactly one model-visible [`ToolResult`] via
/// [`ToolOutcome::into_result`].
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum ToolOutcome {
    /// Normal completion. `ToolResult::is_error` may still be set: a
    /// tool-level failure is a model-readable result, not a kernel error.
    Success(ToolResult),
    /// A kernel / session-level failure with no structured tool payload.
    Error {
        message: String,
        output: Option<serde_json::Value>,
    },
    /// The call was refused at dispatch (e.g. a guarded assertion failed).
    Denied { message: String },
    /// The call was cancelled by the application in this process.
    Cancelled { reason: Option<String> },
    /// Shutdown / restart severed the call.
    Interrupted { reason: InterruptionReason },
}

impl ToolOutcome {
    /// The single model-visible result of this outcome.
    pub fn into_result(self) -> ToolResult {
        todo!()
    }
}

/// Why an [`ToolOutcome::Interrupted`] occurred.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterruptionReason {
    Shutdown,
    Restart,
    Superseded,
}

/// A read-only view of a tool call recorded in the history.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolCallView {
    pub id: ToolCallId,
    pub name: ToolName,
    pub input: ToolInput,
    /// Whether the call already has a committed terminal result.
    pub settled: bool,
}
