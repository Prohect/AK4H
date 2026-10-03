# AK4H — Public API Design

Status: design draft, pre-implementation. This document defines the **public API level** of AK4H —
what a harness application links against. Internal machinery is out of scope here.

## 1. Positioning

AK4H (**A**sync **K**ernel **4** agentic **H**arness) is a **session kernel** library.

AK4H is:

- The owner of one agentic session's state. One AK4H `Session` object is bound 1:1 to one harness session object.
- The parser and protocol owner of the SDAVE channel (tool calls and AK4H protocol messages live as SDAVE envelopes inside the LLM's output channel).
- The owner of session write boundaries, the pending-event queue, coalescing/rendering policy, recovery state, and timer *objects* (debounce, sync-call idle timeout).

AK4H is **not**:

- An LLM API client. AK4H never talks to an LLM API entrypoint. Request building (provider envelope, role merging, cache positioning, ordering) is fully application layer and never leaks into AK4H. *AK4H manages the session to talk to the LLM, not to the LLM API entrypoint.*
- A tool host. AK4H never owns tool objects and never invokes tools. The harness executes tools and pushes lifecycle events in. Tool execution state persistence is the harness's DB business.
- A runtime. AK4H owns no async runtime, thread, or executor. The application provides the motor (tokio, thread pool, manual pump — anything).

## 2. Channel model

The traditional tool channel is **fully deprecated**:

| Traditional channel | In AK4H |
|---|---|
| `tools` declaration field (separate from messages) | Gone — declarations are session content, rendered by AK4H from harness-pushed `ToolDecl`s |
| Structured tool_call channel (LLM → harness) | Gone — tool calls are SDAVE envelopes in the output stream, parsed by AK4H |
| Structured tool_result channel (harness → LLM) | Gone — tool events are session content, rendered user-role at write boundaries |
| system, user (harness → LLM) | Unchanged, application-owned |
| output (∋ SDAVE), thinking (LLM → harness) | Streamed through AK4H |

Consequences that shape the API:

- History is a plain transcript. No tool_use/tool_result pairing, no orphan repair, no canceled sentinels, no partial-JSON stream assembly, no provider tool-name rules. (Verified against Zed's agent stack: all of that machinery is provider-envelope artifact.)
- Surviving transcript-level constraints are exactly two: (i) no empty/whitespace-only content — AK4H validates everything it writes to history, including rejecting empty/whitespace-only user messages at intake; (ii) thinking-block signature round-trip *if* the harness replays thinking natively — harness business, supported via thinking metadata (§4).

## 3. The `Session` object

```rust
impl Session {
    pub fn new(config: Config) -> Self;
    /// Full state recovery: history IS the state. Machinery is derived by replay
    /// or stored as history entries (recovery markers, manifest markers, sync-call markers).
    pub fn from_history(history: Vec<Entry>, config: Config) -> Self;

    /// The only persistent thing. serde, versioned, `#[serde(default)]` everywhere.
    pub fn history(&self) -> &[Entry];
    pub fn entry(&self, id: EntryId) -> Option<&Entry>;

    // ── intake: LLM side (streaming) ──
    pub fn push_llm_delta(&mut self, channel: Channel, text: &str);
    /// Provider-opaque metadata (e.g. Anthropic signatures); attaches to the
    /// current thinking segment. See §4 Thinking.
    pub fn push_thinking_metadata(&mut self, meta: serde_json::Value);
    /// `StopReason::Canceled` = user/harness abort: commits the partial output
    /// verbatim (including a partial envelope), resets parser/forming state,
    /// and resolves any SyncToolBlocking immediately. See §9.
    pub fn end_llm_turn(&mut self, stop: StopReason);

    // ── intake: harness side ──
    /// Rejects empty/whitespace-only messages (provider-400 trap).
    /// Mid-stream calls are QUEUED until the next write boundary, exactly like
    /// tool events — this is the steering path.
    pub fn push_user_message(&mut self, msg: UserMessage) -> Result<(), Error>;
    pub fn push_tool_event(&mut self, ev: ToolEvent);
    pub fn push_tool_declaration(&mut self, decl: ToolDecl);
    pub fn retract_tool_declaration(&mut self, name: &str);

    /// In-place truncation: excise the history SUFFIX starting at `from`
    /// (edit-and-resubmit / branching). Keeps session identity and the
    /// ToolCallId space the harness DB correlates against. Subject to the
    /// unified id/excision rule (§5): the excised suffix must not contain
    /// regions that consumed an id, unless the harness accepts retracting
    /// those calls (signaled via ToolCallUpdated).
    pub fn truncate(&mut self, from: EntryId) -> Result<(), Error>;

    // ── egress ──
    /// Drain all pending signals. Call after every push and every fire_due().
    pub fn drain_signals(&mut self) -> impl Iterator<Item = Signal>;
    /// Sync tool calls (P3) may block session resume.
    pub fn can_post(&self) -> Result<(), PostBlocked>;

    // ── timers: owned semantically, motor provided by the application ──
    /// Earliest pending internal deadline (debounce flush, sync-call idle timeout, ...).
    pub fn next_deadline(&self) -> Option<Instant>;
    /// Fire all timers due at `now`. The application's runtime sleeps until
    /// next_deadline() (or any external event) and calls this.
    pub fn fire_due(&mut self, now: Instant);
}
```

Integration pattern (runtime-agnostic):

```
loop {
    wait_for(any external event, or next_deadline());
    match event {
        LlmDelta(ch, text) => session.push_llm_delta(ch, text),
        ToolEvent(ev)      => session.push_tool_event(ev),
        Deadline(now)      => session.fire_due(now),
        ...
    }
    for signal in session.drain_signals() { handle(signal); }
}
```

## 4. History model

```rust
/// Stable, monotonic per session. Ids — not indices — are how every delta,
/// truncation, and harness DB reference points at history, because excision
/// shifts indices and ids survive it.
pub struct Entry {
    pub id: EntryId,
    pub kind: EntryKind,
}

pub enum EntryKind {
    /// End-user message; attachments pre-resolved by the harness
    /// (Text | Mention { uri, resolved_content } | Image). AK4H never fetches context.
    User(UserMessage),

    /// Assistant output text, including VERBATIM SDAVE segments.
    /// What the LLM said is what is stored is what the LLM sees next POST.
    Assistant(AssistantSegment),

    /// Typed thinking segment. `metadata` is provider-opaque (e.g. Anthropic
    /// signatures), serde'd with history so from_history stays self-contained.
    Thinking(ThinkingSegment),

    /// Tool lifecycle as session content: start / message(+partial) / finish,
    /// grouped and rendered by ToolCallId. This replaces the tool_result channel.
    ToolEvent(ToolEventEntry),

    /// AK4H-authored session content: tool manifest markers, recovery
    /// markers/diagnostics/rules, reserialized patched blocks (§9),
    /// sync-call markers, continuation fillers. Renders user-role.
    System(SystemEntry),
}
```

Properties:

- **State = history, exactly.** No independent serializable store. Machinery state (next ids, tool table, coalescing state, recovery state, sync-call state) is derivable by replaying history or is itself stored as entries (recovery/sync/manifest markers are entries, precisely so reload derivation is unambiguous).
- **Not append-only.** Recovery (§9) and `truncate` *excise*. `HistoryDelta` expresses removals in `EntryId`s.
- **Naturally consistent with what the LLM sees**: every AK4H history operation defines exactly the transcript the harness will serialize next POST.
- **Reload derivations (pinned)**: recovery-in-progress and open sync calls are reconstructed from their marker entries (never inferred heuristically); in-flight timers (debounce windows, idle timeouts) restart from load time; the pending queue between the last flush and a crash is volatile — durability horizon is the last write boundary, which is correct because the harness's DB is the tool-execution source of truth.

## 5. Tool lifecycle, ids, and the unified id/excision rule

```rust
pub enum ToolEvent {
    Started  { id: ToolCallId, meta: ToolEventMeta }, // meta: kind, subagent link, locations, + extensible map
    Output   { id: ToolCallId, chunk: String },
    Partial  { id: ToolCallId, chunk: String },       // debounced/coalesced by AK4H timers
    Finished { id: ToolCallId, outcome: Outcome },    // Success | Error | Canceled | Shutdown
}
```

**The unified id/excision rule (pinned, load-bearing):**

1. A monotonic 0-based `ToolCallId` is **consumed only by a schema-valid envelope** — i.e., at the same moment the envelope completes parsing AND validates against its declaration's schema (§7).
2. `Signal::ToolCallRequested` fires at exactly that moment, and only then.
3. **Excision (recovery or truncation) only ever targets regions that consumed no id** — malformed or never-validated output.

Consequences: excision can never orphan an execution signal; replay-under-excision is deterministic by construction (`from_history` reproduces the live id map); the harness's execution DB never correlates against an id the reloaded session would reassign.

- LLM-named ids (P1) are grammar-level aliases inside the transcript — advisory, collision-tolerated; unknown/duplicate references are fed back through the AK4H protocol (recovery), never fatal.
- The harness echoes `ToolCallId` in its events. Tool execution state lives in the harness's DB; AK4H keeps only what the session needs.
- P0 property tests: (a) replay stability — `from_history(H)` reproduces the live id map under arbitrary legal excision sequences; (b) golden transcript — every `ToolEvent` entry's id was announced by exactly one un-retracted `ToolCallRequested`.

## 6. Signals (egress)

```rust
pub enum Signal {
    /// A complete, schema-valid SDAVE tool-call envelope. Carries the parsed
    /// payload AND the verbatim payload TEXT (authoritative — offsets dangle
    /// after excision, so the text is copied out). `offsets` into the
    /// assistant segment are advisory, valid only within the same flush cycle.
    ToolCallRequested(ToolCallRequest),

    /// Optional, streaming: an envelope is forming. Carries incremental payload
    /// prefixes (SDAVE partial/phantom regions) for live harness UI.
    /// Invalidated by end_llm_turn(Canceled).
    ToolCallForming(ToolCallForming),

    /// The LLM used the first-class cancel tool (P2).
    ToolCallCancelRequested(ToolCallId),

    /// Mirror of tool state transitions for the harness's DB/UI.
    /// Also the retraction path if truncation removes a referenced call.
    ToolCallUpdated(ToolCallPatch),

    /// A session write boundary was reached and the queue was flushed into
    /// history: the harness should POST now.
    PostNow,

    /// A sync tool call (P3) is blocking session resume (also see can_post).
    SyncToolBlocking(ToolCallId),
    /// Idle timeout fired; the call downgraded to async.
    SyncToolDowngraded(ToolCallId),

    RecoveryEntered { diagnostics: RecoveryDiag },
    RecoveryExited,

    /// What changed in history — EntryId-keyed inserts / updates / removals.
    /// Suitable for persistent mirrors (harness DB) and live UI alike.
    HistoryChanged(HistoryDelta),
}
```

Egress is a drained queue, not per-push return values, because signals can also fire from timers with no push in flight.

## 7. Tool declarations & the serializer

With the `tools` API field gone, declarations are session content:

```rust
pub struct ToolDecl {
    pub name: String,
    pub description: String,
    pub schema: Schema,   // produced by AK4H's serializer
}
```

AK4H provides a **struct serializer** (derive) that turns ordinary Rust structs into the in-session tool-call format descriptions, with **optional per-node descriptions** (doc-comment style). This replaces the lost JSON-schema channel twice over:

1. The serialized description teaches the LLM the call format (manifest rendering, system-position, byte-stable, deterministic order, memoized).
2. The same description validates parsed payloads — this is the payload-against-schema discipline the structured channel used to enforce for free, and rule (1) of §5 depends on it. Validation failures route through the same error-render/recovery pipeline as malformed envelopes.

Manifest changes are deliberate events: a marker entry is recorded in history (so replay stays consistent) and the harness is signaled (it implies a cache flush on the harness side).

## 8. Streaming

The output channel is streamable end-to-end; AK4H is a streaming layer:

- `push_llm_delta` accepts arbitrary chunking; SDAVE's incremental `FlatParser` never tears on chunk boundaries and yields verbatim payload offsets.
- `ToolCallRequested` fires exactly once per complete, schema-valid envelope (execution semantics).
- `ToolCallForming` gives partial visibility (streamed-partial-input UX is worth the complexity).
- Tool output streams back in via `ToolEvent::Output`/`Partial`, coalesced by AK4H's timers before hitting history.

## 9. Cancellation and recovery

**Cancellation (user/harness abort).** The harness aborts its HTTP stream itself, then informs the session via `end_llm_turn(StopReason::Canceled)`:

- partial output — including a partially-formed envelope — is committed to history **verbatim** (it is what the LLM said);
- parser/forming state resets; any outstanding `ToolCallForming` is invalidated;
- any `SyncToolBlocking` resolves immediately (cancellation never waits for the idle-timeout downgrade).

**Recovery (malformed/invalid SDAVE).** On malformed SDAVE channel input:

1. Record a recovery marker entry; attach diagnostics (piling up across failed recoveries) and recovery rules as `EntryKind::System` content; loop until a valid same-type SDAVE message or an escape message arrives; record the exit marker.
2. **Excise-and-reserialize, never mask-in-place**: AK4H *fully removes* polluting thinking/output segments from history and re-serializes the patched content as **user-role, AK4H-authored `System` entries**. (Anthropic forbids mutating produced thinking blocks; the user channel has no pairing constraint — removal + user-role re-serialization is legal everywhere.) Per §5, excision targets only regions that consumed no id.
3. Masked thinking is replaced by the `[AK4H]Filtered due to misleading` marker in the reserialized block.
4. Resume: handle any newly produced call first, then POST; if the transcript needs it, AK4H inserts a continuation user-role message (non-empty, non-whitespace, semantics in content only — providers merge filler segmentation away and reject whitespace-only user text).

## 10. Write boundaries, queue, rendering policy

Session writes happen only at boundaries: **session idle**; **output→thinking channel switch**. No interruption within output.SDAVE, within thinking, or on thinking→output switch.

Between boundaries, tool events AND mid-stream user messages accumulate in the pending queue. At flush:

- **flush ordering (pinned)**: queued tool events and queued user messages flush in arrival order, THEN the thinking segment commits at an output→thinking switch (tool events were produced during the output period; thinking follows);
- ordering metadata (monotonic sequence) preserved; tool messages sorted/grouped by `ToolCallId`;
- coalescing: consecutive same-call messages merge; partial-output debounced by AK4H timers; meaningless start/finish pairs elided;
- **rendering policy** (verbatim / brief / archived, per-call token budgets) decides how tool output becomes history text. Token *counting* is harness-supplied (closure/trait) with a naive built-in default — AK4H owns no tokenizer.
- **rendering is write-once (pinned)**: content is rendered at flush time and persisted verbatim in history. Policy changes are non-retroactive; re-rendering means harness-side rebuild from its own DB via `from_history`.

## 11. Harness responsibilities (explicitly out of AK4H)

- LLM API envelope: request building, role merging, cache positioning/ordering, provider quirks. (AK4H docs will carry guidance: stable-prefix discipline, TTL ordering, filler traps.)
- Tool execution, tool execution state DB, permission gating (re-checked at execution time, not snapshot time).
- The runtime motor: driving `next_deadline()`/`fire_due()`.
- Token counting for rendering budgets (pluggable).

## 12. Open questions

- **Image/multi-modal tool output**: `ToolEvent` chunks are text-only in P0. Add an image variant (with model-capability gating) or declare text-only a permanent non-goal. Needs a decision before 1.0; the intake enum will gain a variant if in scope.
- Schema serializer: derive macro in-crate vs companion `ak4h-derive` crate.
- `ToolEventMeta` shape: fixed struct + extension map (current stub) vs fully generic.
- Priority-ordered backlog beyond P0: assertion-guarded calls (P3), message pushing priority policy + attention protection (P3), DB query first-class tool (P4).
