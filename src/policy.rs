//! Session policies.
//!
//! These are AK4H-specific. They decide how queued tool-call events are pushed
//! into the transcript at a write boundary. The embedding application may adjust
//! them per session.

use std::time::Duration;

/// How a tool call's messages are rendered into the transcript.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderingPolicy {
    pub delivery: DeliveryMode,
    /// Token budget for one tool call's rendered output.
    pub token_budget_per_tool_call: Option<u32>,
    /// What to emit once a tool call exceeds its budget.
    pub on_budget_exceeded: BudgetOverflow,
}

impl Default for RenderingPolicy {
    fn default() -> Self {
        todo!()
    }
}

/// When a tool call's messages are delivered.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum DeliveryMode {
    /// Push chunks as they arrive.
    Push,
    /// Buffer and emit only the finished result.
    WaitForFullResult,
    /// Buffer and flush on a trigger.
    Debounce(Debounce),
}

/// A debounce trigger.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum Debounce {
    ByTime(Duration),
    ByLines(usize),
    ByLength(usize),
    Any(Vec<Debounce>),
}

/// What to render when a tool call exceeds its token budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetOverflow {
    /// Pass through unchanged.
    Verbatim,
    /// Generous head/tail, elided middle.
    Verbose,
    /// A single summary line.
    Brief,
    /// Omit from the stream; keep in history only.
    Archived,
}

/// How queued messages are prioritised when drawn into the transcript.
#[derive(Clone, Debug, PartialEq)]
pub struct PriorityPolicy {
    pub priorities: ProducerPriorities,
    pub attention_protection: AttentionProtection,
}

impl Default for PriorityPolicy {
    fn default() -> Self {
        todo!()
    }
}

/// Who produced a queued message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Producer {
    Kernel,
    Tool,
}

/// Per-producer priorities.
#[derive(Clone, Debug, PartialEq)]
pub struct ProducerPriorities {
    pub kernel: Priority,
    pub tool: Priority,
    /// Whether tool-call messages outrank other queued messages.
    pub prioritize_tool_calls: bool,
}

impl Default for ProducerPriorities {
    fn default() -> Self {
        todo!()
    }
}

/// A message priority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

/// Limits on how much a single priority may draw from the queue.
#[derive(Clone, Debug, PartialEq)]
pub struct AttentionProtection {
    pub max_messages_per_priority: Option<usize>,
    pub max_tokens_per_priority: Option<u32>,
    /// Fraction of the queue one priority may consume, in `(0.0, 1.0]`.
    pub max_queue_fraction: Option<f32>,
}

impl Default for AttentionProtection {
    fn default() -> Self {
        todo!()
    }
}

/// How queued tool-call messages are coalesced before being committed.
#[derive(Clone, Debug, PartialEq)]
pub struct CoalescingPolicy {
    /// Merge successive tool-call messages that share an id.
    pub merge_by_id: bool,
    /// Debounce streamed partials by time.
    pub partial_debounce: Option<Duration>,
    /// Force-flush buffered partials above this size.
    pub partial_max_bytes: Option<usize>,
}

impl Default for CoalescingPolicy {
    fn default() -> Self {
        todo!()
    }
}
