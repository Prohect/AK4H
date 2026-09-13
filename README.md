# AK4H

**A**sync **K**ernel **4** (for) agentic **H**arness.

AK4H is a kernel that brings async tool call capabilities and async session management to agentic harnesses.

## Traditional API (OpenAI, Anthropic)

_LLM API backends today are doing transfer between the json envelope in and what LLM really see and between the json envelope out and what LLM really output. The json serialization is ONLY live between harness wire protocol and LLM API backend wire protocol, different LLM API backends may have different internal LLM <--> backend protocol for tool call and other implementations._

        Harness
           |
           |session:{messages{[{role, content}..]}, tools{[{name, description, schema(also contains descriptions)}..]}, ...(enable thinking, reasoning effort, cache key, temperature, and other API supported parameters)}
           ↓
        LLM (API entrypoint)
           |
           | reasoning, emit tool call envelope(s)
           ↓
        Harness ---> parse tool call envelope(s), execute tool call, wait for tool call result, push result to session.messages (synchronous, [A])
           |
           |
           ↓
        LLM (API entrypoint)

Every tool call, on LLM API level, has at most ONE tool call response message in session.

Three traditional channels that LLM is producer and harness is consumer: 1. output 2. thinking 3. tool call.

Four traditional channels that harness is producer and LLM is consumer: 1. system 2. tools([{name, description, schema}..]) 3. user 4. tool call response.

These channels come from LLM API backend using traditional API protocol and internal protocol to split LLM-harness IO into different channels.

### Problem

- After emitting tool call envelope(s), harness blocks on waiting ALL ongoing tool call(s) to complete.

- **During the execution(period [A]), LLM CANNOT do anything.**

For general purpose, sometimes, you cannot avoid making use of side effects of one tool call.

e.g. During 'walking', a person can see things and make decisions and do things midturn.
While for traditional API today, it would be LLM
not being able to get information during the execution (period [A]),
not being able to reason during the execution (period [A]),
not being able to cancel the 'walk' during the execution (period [A]),
not being able to do other things(sequentially after call A and need partial result(s) or side effects from call A) during the execution (period [A]).

One super useful thing acquiring capabilities above missing today is: use a live debugger to debug one application.

The operating system LLM is using(agent interface) is similar to a DOS built for single processor hardware back in 1990s, not a modern operating system like Linux or Windows.

## AK4H design

- AK4H CANNOT bring LLM full real world alike fully asynchronous experience, atomic LLM actions like emitting a single tool call, thinking about next step or new operations -- those are Model constraints, not LLM API constraints, and thus cannot be resolved by AK4H.

### SDAVE channel

Split traditional output channel into two channels: normal output channel and [SDAVE](https://github.com/Prohect/SDAVE/blob/main/README.md) channel using SDAVE, then design AK4H protocol in SDAVE channel.

AK4H would own external syntax requirements for SDAVE channel messages.

#### P1 SDAVE escape

Escape mechanism to allow LLM to say a SDAVE literally(human interface rendering layer, still a SDAVE on the wire layer).

#### P2 SDAVE limiter pairs overrides

First class tool to override SDAVE limiter pairs, for new messages in current session.

#### P2 Malformed/Invalid SDAVE channel message

A recovery state.
On entering this state, last known good session(current session, every before HEAD of the malformed SDAVE) is snapshotted,
diagnostics(pile up if previous recoveries failed) are attached to current session, recovery rules are attached to current session, LLM MUST provide and only provide valid SDAVE channel message of same AK4H message type OR SDAVE escape message in traditional output channel[1], loop until [1] exceeds, then switch back to normal state.
On switching back to normal state, attach last known good session with valid SDAVE channel message(or rendered escape message(reason)) from LLM, mask the thinking channel message producing that malformed/invalid SDAVE(if exists) with `[AK4H]Filtered due to misleading`, if new tool call is produced or other SDAVE message needs handling produced handle it first then resume(POST(attach a meaningless continuation user roled message if LLM API requires user role or tool call role message as LAST message when posting)) the session.

### Tool call

Define new tool call format for SDAVE channel.
LLM is trained to say what he is doing(briefly, midturn between reasoning and tool call emitting), report what he has done, reply user question, via traditional output channel. Thus having tool call channel inside SDAVE channel in traditional output channel could bypass the traditional blocking flow.
And, fully moving the tool call ecosystem out of traditional API flow could avoid LLM being confused by two different accessible tool call channels.

#### P0 AK4H Tool call identification

SessionOnce, when a NEW tool call is referenced in session, index the tool call with a unique identifier(monotonic, 0-based) adding tool call context(brief(token budget limited)) information in place.

#### P1 Tool call identification by LLM

When emitting a tool call, optionally, LLM could name the tool call with a unique identifier(string from reserved naming chars).
This would make future feature like [Assertion guarded tool call](#### P3 Assertion guarded tool call) possible without requiring external harness POST that only letting LLM know the AK4H tool call id.

#### P2 Tool call cancellation

First class tool, to cancel tool call by tool call id.

#### P3 Sync tool call

Sync tool call, blocking session from being resumed(POST) until tool call response is received, with optional session idle timeout(fallback to async).

#### P3 Assertion guarded tool call

Queue tool call on previous tool call(previous but same batch/within same turn) but with specific expectation/assertion guarding queued call from being executed under unexpected conditions.

### Event system

#### P0 Tool call start event

Queue a tool call start event.

#### P0 Tool call message event

Queue a tool call message.

##### P1 Tool call message order identification

Order metadata for tool call messages, monotonic 0-based sequence number, or timestamp.

##### P1 Partial tool call message

Queue a partial tool call message.

##### P2 Tool call message order identification coalescing

Maintain state: last tool call id(AK4H) pushing a tool call message.
Merge tool call messages on tool call id eq to last tool call id and avoid incrementing monotonic 0-based sequence number id and being separated when event consumed and message pushed to session.

##### P2 Partial tool call message coalescing

Debounce partial tool call messages by time.

#### P0 Tool call finish event

Queue a tool call finish event.

### Database

#### P0 DB: event & machinery state persistence 

This would allow recovery on restart possible.
This would allow future feature like LLM querying event messages possible.

### Session management

No interruption should be allowed during/within:
output.SDAVE channel;
traditional tool call channel(if embedding application supports both output.SDAVE.async tool call and sync tool call channels);
thinking channel;
- on switching from thinking channel to output channel(probably new tool call prepared in mind);

Thus, `session write boundaries`:
- session idle(KV cache hit);
- on switching from ANY to traditional tool call response channel (already blocking and require new POST to LLM api, KV cache hit)(if embedding application supports both output.SDAVE.async tool call and sync tool call channels);
- on switching from output channel to thinking channel(cancel current streaming from LLM API entrypoint, new(current) output contents KV cache miss, old shared prefix KV cache hit);

#### P0 Consume events from queue

Consume queued events from the queue, on `session write boundaries`, write to session then POST.

#### P1 Message formatting

Sort messages by tool call id(AK4H).

#### P2 Coalesce messages

Remove push independent meaningless tool call start event(tool call finish event found for same tool call) messages to session.

Debouncing by time, merge more tool call messages into a single POST.

#### P3 Message pushing priority policy

Prioritize tool call messages, with AK4H declaring AK4H produced events and LLM declaring LLM produced events(tool call related events ...).

##### P3 Attention protection

Protect against attention exhaustion by limiting how much message pushing priorities can be drawn from the queue.

#### P3 Message rendering policy

Render(push to agent interface AKA session) policy, how would LLM like to see results from a tool call.

Multiple dimensions:
- Actively push tool call messages to session OR passively wait for FULL tool call results to be available OR i.e. debounce by given time or by given lines or by given length....
- - How much tokens(roughly) can be used to render tool call results from a single tool call.
  - What should AK4H do with new tool call messages from a tool call running out of budget.
  - e.g. verbatim, verbose, brief, archived ...

#### P4 DB query tool

First class tools, to query the database for messages(mainly for rendering truncated messages).
