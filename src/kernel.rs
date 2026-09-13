//! The kernel and its session-wide configuration.

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::policy::{CoalescingPolicy, PriorityPolicy, RenderingPolicy};
use crate::session::{Session, SessionOptions};

/// One SDAVE limiter pair: `repeat` copies of `limiter` open an envelope and
/// `repeat` copies of `delimiter` close it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LimiterPair {
    pub limiter: char,
    pub delimiter: char,
    pub repeat: usize,
}

/// Session-wide kernel configuration.
#[derive(Clone, Debug, Default)]
pub struct KernelConfig {
    /// The SDAVE limiter pairs in effect for new messages.
    pub limiter_pairs: Vec<LimiterPair>,
    pub rendering: RenderingPolicy,
    pub priority: PriorityPolicy,
    pub coalescing: CoalescingPolicy,
    pub kernel_tools: KernelToolSet,
}

impl KernelConfig {
    /// Kernel configuration with AK4H's recommended defaults.
    pub fn new() -> Self {
        todo!()
    }
}

/// The AK4H session kernel.
///
/// The kernel holds no I/O. The application drives it entirely through
/// [`Session`].
#[derive(Clone, Debug, Default)]
pub struct Kernel {
    config: KernelConfig,
}

impl Kernel {
    /// Create a kernel with `config`.
    pub fn new(config: KernelConfig) -> Self {
        todo!()
    }

    /// The kernel's configuration.
    pub fn config(&self) -> &KernelConfig {
        todo!()
    }

    /// Create a new, empty session.
    pub fn create_session(&self, options: SessionOptions) -> Session {
        todo!()
    }

    /// Restore a session from bytes produced by
    /// [`Session::serialize`](crate::Session::serialize).
    pub fn open_session(&self, bytes: &[u8]) -> Result<Session> {
        todo!()
    }
}

/// A tool implemented by the kernel itself.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelTool {
    /// SDAVE escape.
    Escape,
    /// SDAVE limiter-pair override.
    LimiterOverride,
    /// Tool-call cancellation.
    Cancel,
    /// Read-only query over the session history.
    DbQuery,
}

/// Per-tool configuration of a [`KernelTool`].
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum KernelToolConfig {
    Escape(EscapeConfig),
    LimiterOverride(LimiterOverrideConfig),
    Cancel(CancelConfig),
    DbQuery(DbQueryConfig),
}

/// Configuration for [`KernelTool::Escape`].
#[derive(Clone, Debug, PartialEq)]
pub struct EscapeConfig {
    pub enabled: bool,
}

/// Configuration for [`KernelTool::LimiterOverride`].
#[derive(Clone, Debug, PartialEq)]
pub struct LimiterOverrideConfig {
    pub enabled: bool,
    /// The limiter pairs the tool installs for new messages.
    pub pairs: Vec<LimiterPair>,
    /// Whether the override reverts after a single message.
    pub one_shot: bool,
}

/// Configuration for [`KernelTool::Cancel`].
#[derive(Clone, Debug, PartialEq)]
pub struct CancelConfig {
    pub enabled: bool,
    /// Which call the tool targets when the model does not name one.
    pub default_target: CancelTarget,
    /// Which terminal outcome a cancellation produces.
    pub terminal: CancelTerminal,
}

/// Configuration for [`KernelTool::DbQuery`].
#[derive(Clone, Debug, PartialEq)]
pub struct DbQueryConfig {
    pub enabled: bool,
    pub allowed: Vec<DbQueryKind>,
    pub max_rows: usize,
    /// Enforced read-only.
    pub read_only: bool,
}

/// Default target of [`KernelTool::Cancel`].
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancelTarget {
    WholeTurn,
    OldestInFlight,
    NewestInFlight,
}

/// Terminal outcome of a cancellation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancelTerminal {
    Cancelled,
    Interrupted,
}

/// A permitted [`KernelTool::DbQuery`] query.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DbQueryKind {
    SessionMetadata,
    MessageRange,
    ToolCallHistory,
    TokenUsage,
}

/// The scope of a [`KernelTool::LimiterOverride`] application.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverrideScope {
    ThisSession,
    ThisTurn,
    ThisToolCall,
}

/// The mutable, per-session management surface for the kernel tools.
#[derive(Clone, Debug, Default)]
pub struct KernelToolSet {
    policies: Vec<(KernelTool, KernelToolConfig)>,
}

impl KernelToolSet {
    /// The configuration for `tool`, if set.
    pub fn policy(&self, tool: KernelTool) -> Option<&KernelToolConfig> {
        todo!()
    }

    /// Set the configuration for `tool`.
    pub fn set_policy(&mut self, tool: KernelTool, config: KernelToolConfig) {
        todo!()
    }

    /// Whether `tool` is enabled.
    pub fn is_enabled(&self, tool: KernelTool) -> bool {
        todo!()
    }

    /// Enable or disable `tool`.
    pub fn set_enabled(&mut self, tool: KernelTool, enabled: bool) {
        todo!()
    }
}
