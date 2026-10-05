# AK4H — Design

**A**sync **K**ernel **4** agentic **H**arness.

AK4H is a **session kernel** library. It brings async tool-call capabilities and async session
management to agentic harnesses, and it is the single owner of one agentic session's state.

This document is the design of AK4H's public API level — what a harness application links against —
together with the protocol semantics that API is built on. Internal machinery is out of scope,
except where the design deliberately exposes it (the escape hatches of §6).

## 1. What AK4H is

The owner of one agentic session's state. One AK4H `Session` is bound 1:1 to one harness session
object.

The parser and protocol owner of the SDAVE channel(s) the session carries — both the envelopes the
model emits and the ones AK4H authors (§4).

The owner of session write boundaries, the pending-event queue, coalescing and rendering policy,
recovery state, and timer *objects* (debounce windows, sync-call idle timeouts).

AK4H is **not**:

- **An LLM API client.** AK4H never talks to an LLM API entrypoint. Request building — provider
  envelope, role merging, cache positioning and ordering — is fully application layer and never
  leaks into AK4H. *AK4H manages the session to talk to the LLM, not to the LLM API entrypoint.*
- **A tool host.** AK4H never owns tool objects and never invokes tools. The harness executes tools
  and pushes lifecycle events in. Tool-execution state persistence is the harness's business.
- **A runtime.** AK4H owns no async runtime, thread, or executor. The application provides the
  motor (tokio, a thread pool, a manual pump — anything).

## 2. The problem

Under the traditional tool-call API, the harness blocks on **all** ongoing tool calls before it can
continue the turn. During that period the model cannot see, reason about, influence, or act on the
partially completed work. This is the difference between a single-processor DOS of the 1990s and a
modern operating system.

AK4H cannot give the model a fully asynchronous experience — emitting a tool call, thinking, or
acting are atomic model steps, a model constraint rather than an API constraint. What AK4H can do is
remove the API-level blocking: let the model keep producing output while tool calls run, let tool
output stream back mid-turn, and let the model cancel or steer during execution.

## 3. Channel model

### 3.1 The traditional tool channel is deprecated

| Traditional channel | In AK4H |
|---|---|
| `tools` declaration field (separate from messages) | Gone — declarations are session content |
| Structured tool_call channel (LLM → harness) | Gone — tool calls are SDAVE envelopes in the output stream |
| Structured tool_result channel (harness → LLM) | Gone — tool events are session content, rendered user-role |
| system, user (harness → LLM) | Unchanged, application-owned |
| output (∋ SDAVE), thinking (LLM → harness) | Streamed through AK4H |

Consequences that shape the API:

- History is a plain transcript. There is no tool_use/tool_result pairing, no orphan repair, no
  canceled sentinels, no partial-JSON stream assembly, no provider tool-name rules.
- Two transcript-level constraints survive: content is never empty or whitespace-only (AK4H
  validates everything it writes to history, including rejecting empty/whitespace-only user messages
  at intake); and, if the harness replays thinking natively, thinking-block signatures round-trip
  (supported via thinking metadata, §8).

### 3.2 Two SDAVE-carrying channels

The session's transcript carries SDAVE in **two** channels, with distinct owners and directions:

| Channel | Emitted by | Consumed by | Carries |
|---|---|---|---|
| **Output channel** | the model | AK4H | tool calls and other model-facing protocol messages |
| **User-message channel** | **AK4H** | **AK4H** (and the model) | AK4H's **designed control constants** (§8.2), rendered user-role |

The user channel is where AK4H keeps its own model-meaningful session state *inside the transcript*,
so that state survives a restart by construction.

## 4. The SDAVE layer

SDAVE (*Streamable Delimiter-Adaptive Verbatim Envelope*) is a flat, payload-verbatim envelope
protocol. It reserves a delimiter slice from a configurable set and decodes the payload byte-identical
to what was written, so it can tell envelopes, pending/partial envelopes, and plain non-envelope
slices apart in one mixed, streamed channel.

AK4H embeds SDAVE and uses three of its layers:

- **Framing primitives** — mixed-channel scanning: envelope vs pending vs non-envelope, with verbatim
  payload offsets. Pure, buffer-in/bytes-out.
- **The typed codec** — one type-marked value per document, for *complete* documents (static
  protocol messages, and persisted/reparsed messages).
- **Grammar primitives** — the metadata scanners an application uses to re-scan a *streaming* payload.

SDAVE is a general-purpose library, not a private dependency of AK4H; §19 states the boundary.

## 5. Grammar

### 5.1 The frozen value

The AK4H SDAVE payload grammar's **reserved slice** is:

- the SDAVE `Variant` (`V1` | `V2`), and
- an **ordered** list of limiter pairs (`least_repeat`, `limiter`, `delimiter`).

Order is semantically significant: SDAVE selects the first matching pair, so two grammars with the
same pairs in a different order are different grammars. AK4H references SDAVE's `Variant` and
`LimiterPair<u8>`; it does not redefine them.

### 5.2 Immutability shape

The grammar is application-runtime-configurable but **frozen per session**: the safe surface exposes
it read-only (`&Grammar`), and there is no safe mutable accessor. "Frozen" is a *shape* — a value
that, once set, is never mutated through the safe surface — not a required `std::sync::OnceLock`.
Mutable access exists only through an `unsafe` escape hatch (§6).

### 5.3 Canon, not constraint

AK4H and SDAVE **may** offer canonical renderers of the grammar (a profile-spec text) for teaching
the model, for documentation, or for persistence. Neither **forces** a system prompt: **the
application owns the system prompt** and may ignore any canon.

Consistency between "what the model was taught" and "what the parser accepts" is enforced by the
**parser against the frozen grammar**, not by the prompt. An envelope inconsistent with the frozen
grammar fails structurally and enters the normal recovery path (§16).

### 5.4 Self-application of the config

The SDAVE config itself (`variant`, `limiter_pairs`) is expressible as a supported SDAVE struct
(§8.2). When it is present in session history, `from_history` **self-applies** it to the AK4H object,
so on the canon path the frozen grammar is recovered from the session itself.

If instead the application customizes its own representations and does not use AK4H's canons, it
**must** maintain the equivalent consistency itself — typically in its own database.

### 5.5 The fingerprint is a helper

A stable grammar fingerprint (variant + ordered pairs) is:

- **meaningless to the model**, so it is **not** serialized into the user channel;
- exposed **only as an integration-check helper**, for the case where the application patches its
  own canons into the system prompt and wants to verify its view agrees with the session.

Applications that do not need it ignore it.

## 6. Interaction shape

### 6.1 The safe surface is immutable in shape

The designed fields are private, reachable through `&`-returning accessors, and have **no** safe
`&mut` accessor. All state transitions go through sanctioned operations (intake methods, `fire_due`,
…). "Immutable" means *the fields are not exposed for mutation*, not that the session never changes.

### 6.2 `unsafe` escape hatches

Following SDAVE's idiom (safe decomposition such as `FlatParser::decouple`; `unsafe` recomposition
such as `FlatParser::new_unchecked`), AK4H marks with `unsafe` the accessors whose semantic
precondition AK4H **cannot verify** — calling them steps outside AK4H's designed flow.

Every `unsafe` item documents its obligations under two labels:

```
/// # Safety
/// - **UB**: aliasing / lifetime obligations.
/// - **AK4H invariants**: flow/semantic obligations the caller now owns.
```

**All** designed fields are exposed through the escape hatch. `SessionParts` (safe, read-only) is the
field-level specification of AK4H's design:

```rust
pub struct SessionParts<'a> {
    pub grammar:   &'a Grammar,
    pub history:   &'a [Entry],
    pub manifest:  &'a Manifest,
    pub pending:   &'a PendingQueue,
    pub recovery:  &'a RecoveryState,
    pub sync:      &'a SyncState,
    pub parse:     &'a ParseState,
    pub ids:       IdCounters,
}

impl Session {
    pub fn parts(&self) -> SessionParts<'_>;            // safe, read-only
    pub fn decouple(self) -> OwnedSessionParts;         // safe decomposition
    /// # Safety — **UB**: none beyond ownership. **AK4H invariants**: `ids` consistent with
    /// `history`; `recovery` carries markers iff live; `parse` matches the history tail; etc.
    pub unsafe fn from_parts(parts: OwnedSessionParts) -> Self;
    /// # Safety — **UB**: views must not be held across `drain_signals`/`fire_due`.
    /// **AK4H invariants** as above for any field mutated.
    pub unsafe fn parts_mut(&mut self) -> SessionPartsMut<'_>;
}
```

### 6.3 State ownership split

| State | Owner | Rebuild path |
|---|---|---|
| Application-owned, or safely mutable without the `unsafe` API | application | AK4H provides **canon builder APIs** (§6.4) |
| Semantically owned by AK4H (e.g. `IdAllocator`s, recovery state, sync state) | AK4H | **must** be serialized to the user channel or derived from it (§8.3, §9.4) |

### 6.4 Canon builder APIs

For application-owned state (and state the application may safely mutate without the `unsafe` API),
AK4H provides **canon builder APIs**: helpers that construct the canonical (SDAVE-serialized) form of
a value so the application can place it in the user channel itself. AK4H does not own or interpret
that content beyond the canonical envelope framing.

## 7. The `Session` object

```rust
impl Session {
    pub fn new(config: Config) -> Result<Self, Error>;
    /// Rebuild the session from history. Recovers exactly what AK4H needs (§8.3).
    pub fn from_history(history: Vec<Entry>, config: Config) -> Result<Self, HistoryError>;

    /// The only persistent thing. serde, versioned, `#[serde(default)]` everywhere.
    pub fn history(&self) -> &[Entry];
    pub fn entry(&self, id: EntryId) -> Option<&Entry>;

    /// The frozen grammar, read-only. §5.
    pub fn grammar(&self) -> &Grammar;
    /// Integration-check helper only; not serialized to the user channel. §5.5.
    pub fn grammar_fingerprint(&self) -> GrammarFingerprint;

    // ── intake: model side (streaming) ──
    pub fn push_llm_delta(&mut self, channel: Channel, text: &str);
    pub fn push_thinking_metadata(&mut self, meta: serde_json::Value);
    pub fn end_llm_turn(&mut self, stop: StopReason);

    // ── intake: harness side ──
    pub fn push_user_message(&mut self, msg: UserMessage) -> Result<(), Error>;
    pub fn push_tool_event(&mut self, ev: ToolEvent);
    pub fn push_tool_declaration(&mut self, decl: ToolDecl);
    pub fn retract_tool_declaration(&mut self, name: &str);
    /// Application-authored canonical content (§6.4), placed in the user channel.
    pub fn push_control(&mut self, control: ControlExt) -> Result<(), Error>;

    // ── egress ──
    /// Drain all pending signals. The result is owned, so a handler may reenter the session.
    pub fn drain_signals(&mut self) -> Vec<Signal>;
    pub fn can_post(&self) -> Result<(), PostBlocked>;

    // ── timers ──
    pub fn next_deadline(&self) -> Option<Instant>;
    pub fn fire_due(&mut self, now: Instant);
}
```

### 7.1 Truncation is a rebuild

AK4H provides **no** in-place truncate operation. To edit-and-resubmit or branch, the application
truncates in its own store and hands AK4H the new history: `from_history(new_history, config)`. This
keeps a single, well-defined rule for in-place history surgery (§9.2).

### 7.2 Integration pattern

Runtime-agnostic:

```
loop {
    wait_for(any external event, or next_deadline());
    match event {
        LlmDelta(ch, text) => session.push_llm_delta(ch, text),
        ToolEvent(ev)      => session.push_tool_event(ev),
        Deadline(now)      => session.fire_due(now),
        ...
    }
    for signal in session.drain_signals() { handle(signal); }   // owned; reentrant
}
```

### 7.3 Threading and affinity

`Session` is single-thread-affine and is not required to be `Send`. All intake is serialized on one
thread, and no `&mut` is held across an `await`. An application that receives model output on another
task hops it onto the session's thread before calling in.

## 8. History

### 8.1 Entry kinds

```rust
pub struct Entry { pub id: EntryId, pub kind: EntryKind }

pub enum EntryKind {
    User(UserMessage),                 // end-user message; attachments pre-resolved by the harness
    Assistant(AssistantSegment),       // verbatim output, incl. verbatim SDAVE
    Thinking(ThinkingSegment),         // typed; provider-opaque `metadata`
    ToolEvent(ToolEventEntry),         // tool lifecycle grouped by ToolCallId
    Control(Control),                  // AK4H-authored designed constants (§8.2), user-role
}
```

`EntryId`s are stable and monotonic per session. Ids — not indices — are how every delta, write, and
harness-DB reference points at history, because history surgery shifts indices and ids survive it.

### 8.2 Designed control constants

AK4H owns a **closed, versioned** set of designed control constants. They are AK4H-authored,
user-role session content, they serialize and deserialize via SDAVE (they are SDAVE envelopes in the
user channel, §3.2), and `from_history` recognizes and re-applies them.

The membership rule is **model-meaningfulness**:

| Model-meaningful → serialized to the user channel | Model-meaningless → NOT in the user channel |
|---|---|
| allocated ids (tool-call id, order/sequence ids) | the grammar fingerprint (§5.5) |
| tool-call states (started / output / finished) | coalescing / debounce machinery state |
| tool-call results | the pending queue between flushes |
| recovery rules / diagnostics | |

### 8.3 Rebuild

`from_history` recovers **exactly what AK4H needs**:

- AK4H-owned state that is model-meaningful is present as designed control constants and
  **self-applied** (recovery-in-progress, sync markers, manifest markers, the SDAVE config of §5.4).
- AK4H-owned state that is derivable is **derived from the model-meaningful content already in the
  transcript** — e.g. `IdAllocator`s from the existing ids (§9.4).
- Everything else (timers, the not-yet-flushed pending queue) is volatile. The durability horizon is
  the last write boundary, which is correct because the harness's DB is the tool-execution source of
  truth.

Because the excision of polluting content and its markers are history content, a rebuild after a
restart reproduces the *same cleaned session*: removed content does not reappear.

## 9. Tool lifecycle, ids, and the id/excision rule

### 9.1 `ToolEvent`

```rust
pub enum ToolEvent {
    Started  { id: ToolCallId, meta: ToolEventMeta },
    Output   { id: ToolCallId, chunk: String },
    Partial  { id: ToolCallId, chunk: String },
    Finished { id: ToolCallId, outcome: Outcome },   // Success | Error | Canceled | Shutdown
}
```

`meta` carries kind, subagent link, locations, and an extensible map.

### 9.2 The unified id/excision rule (load-bearing)

1. A monotonic 0-based `ToolCallId` is consumed **only** by a schema-valid envelope — at the same
   moment the envelope completes parsing *and* validates against its declaration's schema.
2. `Signal::ToolCallRequested` fires at exactly that moment, and only then.
3. AK4H excises history **in place** only in id-free regions, and only during recovery (§16).

Because truncation is a rebuild (§7.1), rule 3 is unconditional: no AK4H operation ever excises an
id-consuming region. Removal of id-consuming regions happens only via an application-driven
`from_history` rebuild.

LLM-named ids are grammar-level aliases inside the transcript — advisory, collision-tolerated;
unknown or duplicate references are fed back through the protocol (recovery), never fatal. Provider
ids are not identity; the AK4H `ToolCallId` is.

### 9.3 Id assignment is a pure function of history

`from_history(history, config)` takes **no id floor** and re-derives the allocator from the supplied
history. Therefore: within a given history version, id assignment is deterministic and stable; across
an application-driven truncation and rebuild, ids **may** be reassigned, and disambiguating them
across history versions is the **application database's** concern.

### 9.4 `IdAllocator`s

`IdAllocator`s are semantically AK4H-owned and are **rebuilt** from the model-meaningful ids already
serialized in the user channel (existing tool-call ids / event ids / order ids). They are not stored
as a separate authoritative constant.

## 10. Signals (egress)

```rust
pub enum Signal {
    /// A complete, schema-valid SDAVE tool-call envelope. Carries the parsed payload AND the
    /// verbatim payload TEXT (authoritative — offsets dangle after removal, so the text is copied
    /// out). Offsets into the assistant segment are advisory, valid only within the same flush.
    ToolCallRequested(ToolCallRequest),
    /// Optional streaming: an envelope is forming. Invalidated by end_llm_turn(Canceled).
    ToolCallForming(ToolCallForming),
    /// The model used the first-class cancel tool.
    ToolCallCancelRequested(ToolCallId),
    /// Mirror of tool state transitions, for the harness's DB/UI.
    ToolCallUpdated(ToolCallPatch),
    /// A write boundary was reached and the queue was flushed: the harness should POST now.
    PostNow,
    /// A sync tool call is blocking session resume (also see `can_post`).
    SyncToolBlocking(ToolCallId),
    /// Idle timeout fired; the call downgraded to async.
    SyncToolDowngraded(ToolCallId),
    RecoveryEntered { diagnostics: RecoveryDiag },
    RecoveryExited,
    /// What changed in history — id-keyed inserts / updates / removals.
    HistoryChanged(HistoryDelta),
}
```

Egress is a drained queue, not per-push return values, because signals can also fire from timers with
no push in flight. The queue is handed out owned (§7) so handlers may reenter the session.

## 11. Tool declarations and the serializer

With the `tools` API field gone, declarations are session content:

```rust
pub struct ToolDecl { pub name: String, pub description: String, pub schema: Schema }
```

AK4H provides a **struct serializer** (derive) that turns ordinary Rust structs into the in-session
tool-call format, with optional per-node descriptions. It does two jobs:

1. It **teaches the model** the call format (manifest rendering: byte-stable, deterministic order,
   memoized). The *reserved slice* comes from the frozen grammar (§5); the manifest teaches the
   *message/input format* on top of it.
2. The same description **validates** parsed payloads — the payload-against-schema discipline rule
   (1) of §9.2 depends on. Validation failures route through the same recovery pipeline as malformed
   envelopes.

Manifest changes are deliberate events: a marker control constant is recorded in history and the
harness is signalled.

## 12. Streamable tool inputs

### 12.1 Shape

Execution mode is **not** a runtime field. It is a **trait marker** on the tool input type —
implemented or not — refined **per field** by derive attributes. An input type without the marker is
**atomic**: AK4H holds the call until the envelope is complete and schema-valid. An input type with
the marker declares which fields may be delivered incrementally.

### 12.2 Delivery-only

§12 is deliberately thin:

- AK4H delivers **raw per-field byte chunks** with field identity and location. It performs **no**
  integrity checking and **no** known-good/pending split.
- The **application** constructs the callback object (so it can close over its environment) and
  **owns** the progress state — the single source of truth. The application does its own reassembly.
- AK4H owns only: (i) **keyed lifecycle** — the progress state is created on the first chunk for a
  field, addressed by `ToolCallId`, and dropped at `Finished` / `end_llm_turn` / cancel; and (ii) a
  **type-erased dispatch path** so registered `dyn` tools can be routed, not only the dynamic-tool
  layer (§13).

### 12.3 Contracts

- A `chunk` is **raw, still-escaped payload bytes**; it may end mid-escape or mid-UTF-8. Delivery is
  verbatim, in order, with monotone offset, and with **no** trimming or coalescing.
- AK4H exposes **no** authoritative "bytes delivered" counter; it reports offsets only *as* it
  delivers. The application's state is the truth.
- Fields not listed as streamable are delivered complete-only.
- The per-field malformed and cancel callbacks let the application roll back a field's incremental
  work.

This is the opposite of the *output* path, where coalescing and rendering are intentional (§17).

## 13. Layers above AK4H: dynamic tools

Dynamic tool ecosystems (MCP is the motivating case) are an **optional layer above AK4H**, not AK4H
core. AK4H core's protocol messages are static Rust types. A dynamic layer defines its own protocol
additions — a dedicated input struct and codec, reusing SDAVE's generic readers and grammar
primitives — and a runtime callback table as the runtime equivalent of the §12 marker.

## 14. Completion ladder

Because SDAVE cannot assert that a streamed block is final, completeness is an AK4H concept with an
explicit ladder. Each rung is owned by exactly one party:

```
framing confirms a frame              ← SDAVE (primitive; may be partial / phantom)
  ↓
streamed payload decoded              ← AK4H's incremental decoder over SDAVE FLAT primitives
  ↓                                     (the SDAVE *typed* codec is the complete-document path only)
payload validates vs schema           ← AK4H
  ↓
message-kind arity satisfied          ← AK4H
  ↓
ToolCallRequested fires (id consumed) ← AK4H, exactly once (§9.2)
  ↓
execution gate: atomic vs incremental ← AK4H protocol, per operation (§12)
```

## 15. Streaming

The output channel is streamable end-to-end; AK4H is a streaming layer:

- `push_llm_delta` accepts arbitrary chunking; SDAVE's incremental parser never tears on chunk
  boundaries and yields verbatim payload offsets.
- `ToolCallRequested` fires exactly once per complete, schema-valid envelope.
- `ToolCallForming` gives partial visibility, on/off per `Config`.
- Tool output streams back via `ToolEvent::Output` / `Partial` and is coalesced by AK4H's timers
  before hitting history.
- **Input-side** streaming to streamable tool inputs (§12) is raw and uncoalesced — a different
  contract from output-side rendering.

## 16. Cancellation and recovery

### 16.1 Cancellation

The harness aborts its HTTP stream itself, then reports it via `end_llm_turn(StopReason::Canceled)`:

- partial output — including a partially formed envelope — is committed to history **verbatim** (it
  is what the model said);
- parser and forming state reset; any outstanding `ToolCallForming` is invalidated;
- any `SyncToolBlocking` resolves immediately (cancellation never waits for the idle-timeout
  downgrade);
- each in-flight streamable field receives its cancel callback (§12).

### 16.2 Recovery

On malformed or invalid SDAVE channel input:

1. Record a **designed recovery control constant** (§8.2) carrying diagnostics (piling up across
   failed recoveries) and recovery rules; loop until a valid same-type SDAVE message or an escape
   message arrives; record the exit constant.
2. **Excise-and-reserialize, never mask-in-place**: fully remove the polluting thinking/output
   segments and re-serialize the patched content as user-role AK4H-authored content. Per §9.2,
   excision targets only id-free regions.
3. Masked thinking is replaced by the `[AK4H]Filtered due to misleading` marker.
4. Resume: handle any newly produced call first, then POST; if the transcript needs it, AK4H inserts
   a continuation user-role message (non-empty, non-whitespace, semantics in content only).

Because the removal and the markers are history content (§8.3), the cleaned state is reproduced on
rebuild after a restart.

## 17. Write boundaries, queue, and rendering

Session writes happen only at boundaries: **session idle**, and the **output → thinking** channel
switch. There is no interruption within output.SDAVE, within thinking, or on thinking → output.

Between boundaries, tool events and mid-stream user messages accumulate in the pending queue. At
flush:

- **ordering**: queued tool events and queued user messages flush in arrival order; the thinking
  segment commits at an output → thinking switch (tool events were produced during the output period;
  thinking follows);
- ordering metadata (monotonic sequence) is preserved; tool messages are sorted and grouped by
  `ToolCallId`;
- coalescing: consecutive same-call messages merge; partial output is debounced by AK4H timers;
  meaningless start/finish pairs are elided;
- **rendering policy** (verbatim / brief / archived, per-call token budgets) decides how tool output
  becomes history text. Token counting is harness-supplied (a closure or trait) with a naive built-in
  default — AK4H owns no tokenizer;
- **rendering is write-once**: content is rendered at flush time and persisted verbatim in history.
  Policy changes are non-retroactive; re-rendering means an application-side rebuild from its own DB
  via `from_history`.

## 18. Harness responsibilities

Explicitly out of AK4H:

- LLM API envelope: request building, role merging, cache positioning and ordering, provider quirks.
- Tool execution, the tool-execution DB, permission gating.
- The runtime motor: driving `next_deadline()` / `fire_due()`.
- Token counting for rendering budgets.
- The system prompt, and the integrity of partial inputs.
- Cross-version id disambiguation.

## 19. SDAVE boundary

SDAVE is a general-purpose framing and serialization library, not a private dependency of AK4H.

| Concern | SDAVE |
|---|---|
| Framing primitives (mixed channel, pending/phantom, verbatim offsets) | owns |
| Typed codec (one typed value per document) | owns |
| Grammar primitives (`is_formatting`, `trim_metadata`, `parse_field_header`) | owns |
| `Config` (variant + pairs + limits) and `Config::fingerprint()` | owns |
| Generic profile-spec text ("canon") | may own generically |
| IO / stream ownership | never |
| Streaming-input lifecycle policy | never |
| Asserting a streamed block is final | never |
| Application-level protocol sanity check | never |
| Channel assembly / message completeness | never |
| AK4H's message grammars (tool-call and control constants) | never |
| `GrammarFingerprint` | never (AK4H computes it over SDAVE's accessors) |

The governing rule: **SDAVE owns pure, general, bytes-in/bytes-out primitives. It never owns IO,
streaming policy, protocol sanity, or application message semantics.**

## 20. Not yet decided

- `Config`'s exact fields (grammar, rendering policy, token counter, clock policy, `ToolCallForming`
  on/off).
- The exact `Control` variants and the naming of `EntryKind::Control`.
- Whether `grammar_fingerprint()` is exposed at all, given it is only a helper.
- Image / multi-modal tool output (`ToolEvent` chunks are text-only today).
- The serializer's home: an in-crate derive macro or a companion crate.
- `ToolEventMeta`: a fixed struct plus an extension map, or fully generic.
- The dynamic-tool layer's shape: a separate crate or a feature-gated module.
- The backlog beyond the current scope: assertion-guarded calls, message-pushing priority and
  attention protection, and a database-query tool.
