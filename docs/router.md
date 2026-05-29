# Router Module

The router module provides a client for communicating with a remote LLM routing server over a persistent TCP connection. Given a user prompt, the routing server quickly classifies it and returns the name of the appropriate downstream service along with a confidence score — allowing the application to dispatch the request without invoking a full large language model.

## Key Characteristics

| Property | Value |
|---|---|
| Transport | TCP, length-prefixed JSON frames |
| Connection | Persistent, automatic reconnect on loss |
| Concurrency | Single request in flight at a time |
| Request timeout | 2 seconds (covers full round-trip) |
| Reconnect delay | 4 seconds |
| Expected latency | < 100 ms (miss), < 1 s (match) |

## Class Diagram

```mermaid
classDiagram

    %% ── Public API ─────────────────────────────────────────────────────────────

    class RouterClient {
        -control_channel_sender : mpsc::Sender~ControlMessage~
        -get_routing_semaphore  : Arc~Semaphore~
        +connect(router_address: &str)$ RouterClient
        +shutdown()
        +get_routing(prompt: &str) RouterResponse
        -do_routing(prompt: &str) RouterResponse
    }

    class RouterResponse {
        <<struct>>
        +text       : String
        +confidence : f32
    }

    %% ── Internal connection manager ────────────────────────────────────────────

    class TcpConnection {
        -tcp_reader              : ReadHalf~TcpStream~
        -tcp_writer              : WriteHalf~TcpStream~
        -response_channel_sender : Option~Sender~TcpResponse~~
        +new(tcp_stream: TcpStream)$ TcpConnection
        -run(control_channel_receiver: Receiver~ControlMessage~)
        -on_heartbeat()
        -on_control_message(control_message: ControlMessage)
        -on_tcp_message(tcp_response: TcpResponse)
        -write_tcp_message(tcp_request: TcpRequest)
        -read_tcp_message() TcpResponse
    }

    %% ── Internal message types ─────────────────────────────────────────────────

    class ControlMessage {
        <<enumeration>>
        Send~TcpRequest, Sender~TcpResponse~~
        Shutdown
    }

    class TcpRequest {
        <<enumeration>>
        Ping
        Request~payload: Value~
    }

    class TcpResponse {
        <<enumeration>>
        Pong
        Response~text: String, confidence: f32~
    }

    %% ── Channels (synthetic — tokio library types) ─────────────────────────────

    class ControlChannel {
        <<mpsc>>
        +send(msg: ControlMessage)
        +recv() ControlMessage
    }

    class ResponseChannel {
        <<oneshot>>
        +send(msg: TcpResponse)
        +recv() TcpResponse
    }

    %% ── Relationships ──────────────────────────────────────────────────────────

    RouterClient "1" *-- "1" TcpConnection : spawns and owns
    RouterClient ..> RouterResponse        : returns
    RouterClient --> ControlChannel        : sends into (Sender end)

    TcpConnection --> ControlChannel       : receives from (Receiver end)
    TcpConnection ..> TcpRequest           : writes to TCP wire
    TcpConnection ..> TcpResponse          : reads from TCP wire
    TcpConnection --> ResponseChannel      : sends into (Sender end, stored per request)

    RouterClient --> ResponseChannel       : receives from (Receiver end, per do_routing call)

    TcpResponse ..> RouterResponse         : TryFrom conversion

    ControlMessage ..> TcpRequest          : carries in Send variant
    ControlMessage ..> ResponseChannel     : carries Sender in Send variant
```

## Sequence Diagrams

### Connection Establishment

`connect()` is called once at startup. It creates the control channel, spawns the background connection manager, and returns immediately. The manager loop runs for the lifetime of the application, reconnecting automatically whenever the TCP connection is lost.

```mermaid
sequenceDiagram
    participant App as Application
    participant Client as RouterClient
    participant CC as ControlChannel (mpsc)
    participant Manager as TcpConnection (background task)
    participant Server as Router Server

    App->>Client: connect(router_address)
    activate Client
    Client->>CC: create channel (capacity 32)
    Client->>Manager: tokio::spawn background loop
    Client-->>App: RouterClient handle
    deactivate Client

    loop on connection loss — retry every 4 s
        Manager->>Server: TCP connect(router_address)
        alt connection established
            Server-->>Manager: TCP stream
            Manager->>Manager: run() event loop
        else connection failed
            Manager->>Manager: sleep 4 s, retry
        end
    end
```

### Request Lifecycle

Each call to `get_routing()` must complete within 2 seconds. A semaphore ensures only one request is in flight at a time — concurrent callers receive an immediate error rather than silently queuing. The response channel is a one-shot: created per request, passed through the control channel inside the `Send` command, and consumed exactly once when the TCP response arrives.

```mermaid
sequenceDiagram
    participant App as Application
    participant Client as RouterClient
    participant CC as ControlChannel (mpsc)
    participant RC as ResponseChannel (oneshot)
    participant Manager as TcpConnection (background task)
    participant Server as Router Server

    App->>Client: get_routing(prompt)
    activate Client

    alt semaphore already taken
        Client-->>App: Err — Router is busy
    else permit acquired
        Client->>RC: create one-shot channel
        Client->>CC: send ControlMessage::Send { TcpRequest, RC sender }
        Note right of Client: await RC receiver (2 s timeout)

        CC-->>Manager: recv ControlMessage::Send
        activate Manager
        Manager->>Server: write TcpRequest::Request (length-prefixed JSON)
        Manager->>Manager: store RC sender
        deactivate Manager

        Server-->>Manager: TcpResponse::Response (length-prefixed JSON)
        activate Manager
        Manager->>RC: send TcpResponse
        deactivate Manager

        RC-->>Client: TcpResponse
        Client->>Client: TryFrom TcpResponse → RouterResponse
        Client-->>App: Ok(RouterResponse)
    end

    deactivate Client
```

### Heartbeat

While idle, `TcpConnection` sends a `Ping` to the server every 10 seconds to keep the connection alive and detect silent drops early.

```mermaid
sequenceDiagram
    participant Manager as TcpConnection (background task)
    participant Server as Router Server

    loop every 10 s
        Manager->>Server: TcpRequest::Ping
        Server-->>Manager: TcpResponse::Pong
    end
```

### Shutdown

```mermaid
sequenceDiagram
    participant App as Application
    participant Client as RouterClient
    participant CC as ControlChannel (mpsc)
    participant Manager as TcpConnection (background task)

    App->>Client: shutdown()
    Client->>CC: send ControlMessage::Shutdown
    CC-->>Manager: recv ControlMessage::Shutdown
    Manager->>Manager: return Err(AppError::Shutdown)
    Note over Manager: outer loop detects Shutdown,<br/>breaks without reconnecting
```

## Wire Protocol

Messages are exchanged as **length-prefixed JSON frames**:

```
┌─────────────────┬──────────────────────────────┐
│  4 bytes        │  N bytes                     │
│  u32 big-endian │  UTF-8 JSON payload          │
│  (length = N)   │                              │
└─────────────────┴──────────────────────────────┘
```

The JSON uses a `"type"` discriminant field (lowercase variant name):

```json
// client → server
{ "type": "ping" }
{ "type": "request", "payload": { "prompt": "what is the weather?" } }

// server → client
{ "type": "pong" }
{ "type": "response", "text": "weather", "confidence": 0.97 }
```

