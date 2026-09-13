//! The session handle.
//!
//! A session owns the transcript. The application drives it in two directions:
//!
//! * **inbound** — [`send_user_message`](Session::send_user_message),
//!   [`push_delta`](Session::push_delta), [`end_post`](Session::end_post), and
//!   [`push_tool_event`](Session::push_tool_event) feed the session; and
//! * **outbound** — [`SessionEvent`]s are delivered through the receiver
//!   returned by [`take_events`](Session::take_events).

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::event::SessionEvent;
use crate::kernel::KernelToolSet;
use crate::message::{ContentPart, Message};
use crate::policy::{CoalescingPolicy, PriorityPolicy, RenderingPolicy};
use crate::request::ToolDefinition;
use crate::stream::{ChannelDelta, StopReason};
use crate::tool::{ToolCallId, ToolCallView, ToolEvent};

/// A session identifier.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

/// Options used to create a session.
#[derive(Clone, Debug, Default)]
pub struct SessionOptions {
    /// The tools the application owns and executes.
    pub host_tools: Vec<ToolDefinition>,
    pub rendering: RenderingPolicy,
    pub priority: PriorityPolicy,
    pub coalescing: CoalescingPolicy,
}

/// The outbound event receiver.
pub type EventReceiver = futures::channel::mpsc::UnboundedReceiver<SessionEvent>;

/// A session. Owns the append-only transcript and the transient event queue.
#[allow(dead_code)]
pub struct Session {
    id: SessionId,
}

impl Session {
    /// This session's identifier.
    pub fn id(&self) -> &SessionId {
        todo!()
    }

    /// Take the outbound event receiver. Call once.
    pub fn take_events(&mut self) -> EventReceiver {
        todo!()
    }

    /// Append a user message and advance the session if a write boundary is
    /// reached.
    pub fn send_user_message(&mut self, content: Vec<ContentPart>) -> Result<()> {
        todo!()
    }

    /// Replace the set of application-owned tool definitions offered to the
    /// model.
    pub fn set_host_tools(&mut self, tools: Vec<ToolDefinition>) {
        todo!()
    }

    /// Feed one delta of the in-flight model response.
    pub fn push_delta(&mut self, delta: ChannelDelta) -> Result<()> {
        todo!()
    }

    /// End the in-flight model response.
    pub fn end_post(&mut self, reason: StopReason) -> Result<()> {
        todo!()
    }

    /// Accept a tool lifecycle event from the application.
    ///
    /// Accepted for any `id` present in the history, including calls dispatched
    /// by a previous process. The first [`ToolEvent::Finish`] for an id wins;
    /// later events for that id are dropped.
    pub fn push_tool_event(&mut self, event: ToolEvent) -> Result<()> {
        todo!()
    }

    /// Cancel a dispatched tool call.
    pub fn cancel_tool_call(&mut self, id: &ToolCallId) -> Result<()> {
        todo!()
    }

    /// Tool calls recorded in the history that have no terminal result.
    ///
    /// Used to reconcile after a restart.
    pub fn pending_tool_calls(&self) -> Vec<ToolCallId> {
        todo!()
    }

    /// A read-only view of one recorded tool call.
    pub fn tool_call(&self, id: &ToolCallId) -> Option<ToolCallView> {
        todo!()
    }

    /// The session transcript.
    pub fn history(&self) -> &[Message] {
        todo!()
    }

    /// Serialize the session history.
    pub fn serialize(&self) -> Result<Vec<u8>> {
        todo!()
    }

    /// Restore a session from [`serialize`](Session::serialize) output.
    pub fn deserialize(bytes: &[u8]) -> Result<Self> {
        todo!()
    }

    /// The kernel-tool management surface.
    pub fn kernel_tools(&self) -> &KernelToolSet {
        todo!()
    }

    /// The kernel-tool management surface, mutably.
    pub fn kernel_tools_mut(&mut self) -> &mut KernelToolSet {
        todo!()
    }

    /// The rendering policy.
    pub fn rendering_policy(&self) -> &RenderingPolicy {
        todo!()
    }

    /// Replace the rendering policy.
    pub fn set_rendering_policy(&mut self, policy: RenderingPolicy) {
        todo!()
    }

    /// The priority policy.
    pub fn priority_policy(&self) -> &PriorityPolicy {
        todo!()
    }

    /// Replace the priority policy.
    pub fn set_priority_policy(&mut self, policy: PriorityPolicy) {
        todo!()
    }

    /// The coalescing policy.
    pub fn coalescing_policy(&self) -> &CoalescingPolicy {
        todo!()
    }

    /// Replace the coalescing policy.
    pub fn set_coalescing_policy(&mut self, policy: CoalescingPolicy) {
        todo!()
    }
}
