//! # AK4H — **A**sync **K**ernel **4** (for) agentic **H**arness
//!
//! AK4H is a **session kernel**. It owns the agent session and drives the
//! turn / event flow, but performs **no I/O** of its own. The embedding
//! application is the *driver layer* and supplies the two external effects:
//!
//! * the **LLM API request** — the kernel emits [`SessionEvent::PostRequest`],
//!   the application performs the request and feeds the response back through
//!   [`Session::push_delta`]; and
//! * **tool execution** — the kernel emits
//!   [`SessionEvent::ToolCallDispatch`], the application runs the call and
//!   reports progress through [`Session::push_tool_event`].
//!
//! ## Core properties
//!
//! * The **transcript is the state.** A session is an append-only history of
//!   [`Message`]s. There is no separate machinery-state store; persistence is
//!   [`Session::serialize`] / [`Session::deserialize`] of that history, which is
//!   naturally consistent with what the language model sees.
//! * **SDAVE is the single encode/decode layer.** SDAVE envelope bytes are kept
//!   verbatim ([`SdaveEnvelope`]) and must never be modified by the application.
//! * At most **one LLM request is in flight** per session.
//! * Every tool call committed to history receives **exactly one** terminal
//!   [`ToolEvent`].
//!
//! ## Boundary
//!
//! ```text
//! application ──post / execute──▶ AK4H ──SessionEvent──▶ application
//!             (push_delta,               (PostRequest,
//!              push_tool_event)           ToolCallDispatch, …)
//! ```

#![allow(dead_code, unused_variables)] // API skeleton: bodies are `todo!()`.

pub mod error;
pub mod event;
pub mod kernel;
pub mod message;
pub mod policy;
pub mod request;
pub mod session;
pub mod stream;
pub mod tool;

pub use error::{Error, Result};
pub use event::{
    Diagnostic, Notice, NoticeId, NoticeKind, NoticeSeverity, SdaveRejection, SessionEvent,
};
pub use kernel::{
    CancelConfig, CancelTarget, CancelTerminal, DbQueryConfig, DbQueryKind, EscapeConfig, Kernel,
    KernelConfig, KernelTool, KernelToolConfig, KernelToolSet, LimiterOverrideConfig, LimiterPair,
    OverrideScope,
};
pub use message::{
    ContentPart, Image, Message, Role, SdaveEnvelope, ToolCallPart, ToolResult, ToolResultContent,
    ToolResultPart,
};
pub use policy::{
    AttentionProtection, BudgetOverflow, CoalescingPolicy, Debounce, DeliveryMode, Priority,
    PriorityPolicy, Producer, ProducerPriorities, RenderingPolicy,
};
pub use request::{
    CompletionParams, CustomToolFormat, GrammarSyntax, ReasoningEffort, SessionMessage,
    SessionRequest, Speed, ThinkingParams, ToolChoice, ToolDefinition, ToolDefinitionInput,
};
pub use session::{EventReceiver, Session, SessionId, SessionOptions};
pub use stream::{Channel, ChannelDelta, ChannelError, StopReason, TokenUsage, ToolCallDelta};
pub use tool::{
    AssertionExpr, AssertionPath, CompareOp, InterruptionReason, ParseToolCallIdError,
    ToolCallFinish, ToolCallId, ToolCallMessage, ToolCallMode, ToolCallStart, ToolCallView,
    ToolEvent, ToolInput, ToolInputError, ToolKind, ToolMessagePayload, ToolName, ToolOutcome,
};
