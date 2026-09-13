//! Streamed model output, pushed into the kernel by the application.
//!
//! The application performs the LLM API request and feeds the raw response back
//! as a sequence of [`ChannelDelta`]s through
//! [`Session::push_delta`](crate::Session::push_delta). Output text is fed
//! through **byte-verbatim**: SDAVE envelopes live in the output channel and the
//! kernel, not the application, is responsible for decoding them.

use serde::{Deserialize, Serialize};

/// Which producer channel a delta belongs to.
///
/// A switch between [`Channel::Output`] and [`Channel::Thinking`] is a session
/// write boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    /// Model output text. SDAVE envelopes are embedded here.
    Output,
    /// Model reasoning text.
    Thinking,
    /// Legacy tool-call channel.
    ToolCall,
}

/// One delta of a model response.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ChannelDelta {
    /// Append to the output channel.
    Output(String),
    /// Append reasoning text and/or attach an opaque signature.
    Thinking {
        text: String,
        signature: Option<String>,
    },
    /// A legacy tool-call channel delta.
    ToolCall(ToolCallDelta),
    /// Token accounting observed so far.
    Usage(TokenUsage),
    /// The response ended.
    Stop(StopReason),
    /// The response stream failed.
    Error(ChannelError),
    /// A new assistant message begins.
    MessageStart { message_id: Option<String> },
}

impl ChannelDelta {
    /// The channel this delta writes to, or `None` for channel-less bookkeeping.
    pub fn channel(&self) -> Option<Channel> {
        todo!()
    }
}

/// A legacy tool-call channel delta.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCallDelta {
    /// Provider tool-call id, once the provider has sent one.
    pub id: Option<String>,
    /// Tool name, once the provider has sent one.
    pub name: Option<String>,
    /// Raw (possibly incomplete) JSON argument bytes.
    pub partial_json: String,
    /// Whether this is the final delta for the call.
    pub complete: bool,
}

/// A terminal failure of the response stream.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ChannelError {
    /// The stream ended without a [`ChannelDelta::Stop`].
    EndedUnexpectedly,
    /// A delta could not be interpreted.
    Malformed { detail: String },
    /// The provider rejected the request.
    Provider { message: String },
}

/// Why the model stopped generating.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    ToolUse,
    Refusal,
    /// The response was cancelled.
    Cancelled,
}

/// Token accounting for one model response.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
}

impl TokenUsage {
    /// Sum of all token counters.
    pub fn total_tokens(&self) -> u64 {
        todo!()
    }
}

impl std::ops::Add for TokenUsage {
    type Output = Self;

    fn add(self, _rhs: Self) -> Self {
        todo!()
    }
}
