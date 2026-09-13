//! The POST payload — what the kernel asks the application to send.

use serde::{Deserialize, Serialize};

use crate::message::{ContentPart, Role};
use crate::tool::ToolName;

/// The rendered session handed to the application in a
/// [`SessionEvent::PostRequest`](crate::SessionEvent::PostRequest).
///
/// This is the exact payload the application ships to the LLM API. It is built
/// from the transcript with the session policies applied.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionRequest {
    /// The rendered transcript.
    pub messages: Vec<SessionMessage>,
    /// The tools offered to the model.
    pub tools: Vec<ToolDefinition>,
    /// The tool-choice constraint, if any.
    pub tool_choice: Option<ToolChoice>,
    /// Sampling / thinking / cache controls.
    pub params: CompletionParams,
}

/// One message of a rendered [`SessionRequest`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: Role,
    pub content: Vec<ContentPart>,
    /// Whether this message ends a prompt-cache span.
    pub cache: bool,
}

/// A tool offered to the model.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: ToolName,
    pub description: String,
    pub input: ToolDefinitionInput,
}

/// The input form of a tool definition.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ToolDefinitionInput {
    /// A JSON-schema function.
    Function {
        input_schema: serde_json::Value,
        use_input_streaming: bool,
    },
    /// A free-form custom tool.
    Custom { format: Option<CustomToolFormat> },
}

/// The format of a custom tool.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CustomToolFormat {
    Text,
    Grammar {
        syntax: GrammarSyntax,
        definition: String,
    },
}

/// The grammar syntax of a custom tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GrammarSyntax {
    Lark,
    Regex,
}

/// A tool-choice constraint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoice {
    Auto,
    Any,
    None,
}

/// Sampling / thinking / cache controls.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CompletionParams {
    pub stop: Vec<String>,
    pub temperature: Option<f32>,
    pub thinking: ThinkingParams,
    pub speed: Option<Speed>,
    pub compact_at_tokens: Option<u64>,
    pub max_output_tokens: Option<u64>,
    /// Provider prompt-cache affinity key.
    pub prompt_cache_key: Option<String>,
}

impl CompletionParams {
    /// The effective output-token limit given a model maximum.
    pub fn effective_max_output_tokens(&self, model_max: Option<u64>) -> Option<u64> {
        todo!()
    }
}

/// Thinking / reasoning controls.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ThinkingParams {
    pub allowed: bool,
    pub effort: Option<ReasoningEffort>,
    pub budget_tokens: Option<u32>,
}

/// Provider request speed tier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Speed {
    Standard,
    Fast,
}

/// Reasoning effort level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
    Max,
}
