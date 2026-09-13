//! The transcript data model.
//!
//! A [`Message`] is one entry of the append-only session history. That history
//! *is* the session state and the on-disk format (see
//! [`Session::serialize`](crate::Session::serialize)).

use serde::{Deserialize, Serialize};

use crate::tool::{ToolCallId, ToolInput, ToolName};

/// Who produced a message.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// An image payload.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Image {
    /// Base64-encoded image source.
    pub source: String,
}

/// A verbatim SDAVE envelope.
///
/// The bytes are stored exactly as produced by the SDAVE codec. The application
/// may read them but can never mutate them: the inner buffer is private and
/// only the codec can mint an envelope.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SdaveEnvelope {
    raw: bytes::Bytes,
}

impl SdaveEnvelope {
    /// Mint an envelope. Only the SDAVE codec may call this.
    pub(crate) fn from_codec(raw: bytes::Bytes) -> Self {
        todo!()
    }

    /// Borrow the verbatim bytes.
    pub fn as_bytes(&self) -> &[u8] {
        todo!()
    }

    /// Length in bytes.
    pub fn len(&self) -> usize {
        todo!()
    }

    /// Whether the envelope is empty.
    pub fn is_empty(&self) -> bool {
        todo!()
    }
}

impl AsRef<[u8]> for SdaveEnvelope {
    fn as_ref(&self) -> &[u8] {
        todo!()
    }
}

impl std::ops::Deref for SdaveEnvelope {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        todo!()
    }
}

/// One part of a message.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ContentPart {
    /// Plain, kernel-visible text.
    Text(String),
    /// A verbatim SDAVE envelope.
    Sdave(SdaveEnvelope),
    /// An image.
    Image(Image),
    /// The assistant requested a tool call.
    ToolCall(ToolCallPart),
    /// A tool result fed back to the model.
    ToolResult(ToolResultPart),
}

/// An assistant-requested tool call.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCallPart {
    /// The kernel-assigned identity of this call.
    pub id: ToolCallId,
    /// The model-assigned name, if any.
    pub name: Option<ToolName>,
    /// The call arguments.
    pub input: ToolInput,
    /// Whether `input` is fully received.
    pub input_complete: bool,
    /// Opaque provider state attached to the call, kept verbatim.
    pub thought: Option<SdaveEnvelope>,
}

/// A tool result fed back to the model.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolResultPart {
    /// The call this result answers.
    pub id: ToolCallId,
    /// Whether the result represents an error.
    pub is_error: bool,
    /// The model-visible content.
    pub content: Vec<ToolResultContent>,
    /// Raw tool output retained for replay.
    pub output: Option<serde_json::Value>,
}

/// One content block of a tool result.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ToolResultContent {
    Text(String),
    Image(Image),
}

/// The single model-visible payload of a finished tool call.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    pub is_error: bool,
    pub content: Vec<ToolResultContent>,
    pub output: Option<serde_json::Value>,
}

/// One entry of the append-only session history.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// Kernel-assigned, monotonic within the session.
    pub seq: u64,
    /// Who produced the message.
    pub role: Role,
    /// The message contents.
    pub parts: Vec<ContentPart>,
}
