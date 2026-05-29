# Learning Journal

## 2026-05-29 — Module `llm::router`

---

### 1. Prefer `TryFrom` over `impl Into<Result<...>>`

Implementing `Into` directly on a type is discouraged in Rust. The idiomatic way to express a fallible conversion is the `TryFrom` trait, which the standard library recognises and for which the compiler can derive the complementary `TryInto` automatically.

```rust
// ❌ before
impl Into<Result<RouterResponse>> for TcpMessage { ... }

// ✅ after
impl TryFrom<TcpResponse> for RouterResponse {
    type Error = AppError;
    fn try_from(r: TcpResponse) -> Result<Self> { ... }
}
```

---

### 2. Use named error variants instead of `AppError::Fatal` strings

Using `AppError::Fatal("Shutdown".into())` to signal a known, structured condition is fragile — the string can be misspelled and the match arm `matches!(e, AppError::Fatal(...))` cannot pattern-match on the content. A dedicated variant is the right tool:

```rust
// ❌ before
return Err(AppError::Fatal("Shutdown".into()));
// caller: matches!(e, AppError::Fatal(ref s) if s == "Shutdown")

// ✅ after
return Err(AppError::Shutdown);
// caller: matches!(e, AppError::Shutdown)
```

Reserve `AppError::Fatal` for truly unclassified failures.

---

### 3. Propagate structured errors through return types

When `run()` returned `()`, the `Shutdown` signal from `on_control_message` had nowhere to go — the outer reconnect loop had no way to distinguish "connection lost, reconnect" from "intentional shutdown, stop". Changing `run()` to return `Result<()>` let `AppError::Shutdown` propagate cleanly up the call stack and be matched in the manager loop:

```rust
match tcp_connection.run(&mut control_receiver).await {
    Err(AppError::Shutdown) => break,        // clean exit
    _                       => reconnect,    // any other error
}
```

**Lesson:** when a function needs to communicate *why* it exited, `()` is not enough. Use `Result` or a dedicated return enum.

---

### 4. `tokio::time::interval` fires immediately — use `sleep` for retry delays

`interval(Duration::from_secs(5))` fires its **first tick at t=0**. Placing it at the bottom of a retry loop gives you no delay before the first reconnect attempt after a failure. `tokio::time::sleep` is the right primitive when you simply want to wait before the next iteration:

```rust
// ❌ interval at bottom of loop — first tick is immediate
let mut retry = interval(Duration::from_secs(5));
loop { ...; retry.tick().await; }

// ✅ sleep — always waits the full duration
loop { ...; sleep(Duration::from_secs(4)).await; }
```

---

### 5. `static` vs `const` for compile-time constants

`Duration::from_secs()` is a `const fn`, so a compile-time duration should be declared `const`, not `static`. `const` values are inlined at the call site and carry no runtime address; `static` allocates a memory location, which is unnecessary overhead for a plain value.

```rust
// ❌ static CONNECTION_RETRY_DELAY: Duration = Duration::from_secs(4);
// ✅ const CONNECTION_RETRY_DELAY: Duration = Duration::from_secs(4);
```

---

### 6. Split enums by direction to enforce protocol correctness at compile time

A single `TcpMessage` enum covering both outbound and inbound variants meant the compiler could not prevent sending a `Pong` or receiving a `Ping`. Splitting by direction makes illegal states unrepresentable:

```rust
// ✅ only Serialize needed — only ever written to the wire
enum TcpRequest  { Ping, Request { payload: Value } }

// ✅ only Deserialize needed — only ever read from the wire
enum TcpResponse { Pong, Response { text: String, confidence: f32 } }
```

Side effects: match arms in `on_tcp_message` became exhaustive (no `_ =>` catch-all needed), and unused derives were eliminated.

---

### 7. Concurrent requests in a single-connection design cause silent data corruption

With one TCP connection and one pending-response slot, accepting multiple concurrent requests looks safe but is not:

- Caller A's `oneshot::Sender` gets evicted when Caller B arrives.
- The TCP responses arrive in server order, not caller order.
- The wrong caller gets the wrong response; others get nothing.

The old "busy protection" (sending a fake `TcpMessage::Response { text: "Error: Busy" }`) masked the problem rather than fixing it — the caller received a plausible-looking success response with no way to detect the error.

---

### 8. `mpsc(1)` is not equivalent to enforcing single in-flight requests

Reducing channel capacity to 1 only prevents two messages sitting in the **buffer** at once. It does not prevent two requests being in flight simultaneously, because the buffer slot is freed the moment the worker dequeues the message — well before the TCP round-trip completes.

The right primitive is a **`Semaphore` with 1 permit**, held for the entire duration of the request:

```rust
let _permit = self.semaphore
    .try_acquire()
    .map_err(|_| AppError::Fatal("Router is busy".into()))?;
// permit is held until end of get_routing(), covering the full round-trip
```

---

### 9. `try_acquire` vs `acquire` — design intent matters

| Strategy | Method | Behaviour |
|---|---|---|
| Reject if busy | `try_acquire()` | Returns `Err` immediately |
| Queue if busy | `acquire()` | Awaits until permit is free |

For a routing server designed to respond in under 1 second on a local LAN, **rejecting immediately** is the right choice. Queuing would hide backpressure and make timeouts less predictable. The caller should know fast that routing is unavailable.

---

### 10. Place a single timeout at the highest useful level

Having a timeout inside `do_routing()` and another at the call site creates confusion about what is being timed. A single `timeout` wrapping the entire operation — including semaphore acquisition — is cleaner and easier to reason about:

```rust
pub async fn get_routing(&self, prompt: &str) -> Result<RouterResponse> {
    let _permit = self.semaphore.try_acquire()...?;
    timeout(ROUTING_TIMEOUT, self.do_routing(prompt)).await?
}
```

The timeout value itself belongs in a named constant (`ROUTING_TIMEOUT`) alongside other configuration constants, so it is easy to find and adjust.

---

### 11. The `response_channel_sender` stored on `TcpConnection` bridges two separate event-loop iterations

It is not immediately obvious why the `oneshot::Sender` needs to be stored on the struct rather than used directly inside `on_control_message`. The reason is that `on_control_message` and `on_tcp_message` run in **separate iterations** of the `select!` loop:

- **Iteration N:** control branch fires → request is written to TCP wire, sender is **stored**.
- **Iteration N+k:** TCP branch fires → response arrives, sender is **retrieved and consumed**.

The struct field is a one-slot bridge across that gap. This pattern is idiomatic for async event loops where the producer and consumer of a value are decoupled by I/O.
