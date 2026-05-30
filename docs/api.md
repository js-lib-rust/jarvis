# API Module

The `api` module is the HTTP server layer. It creates an Axum router and exposes two endpoints that emulate the OpenAI chat API, allowing any OpenAI-compatible client to talk to Jarvis.

## Endpoints

| Method | Path | Handler |
|---|---|---|
| `GET` | `/models` | Returns a static list of available models |
| `POST` | `/chat/completions` | Accepts a chat request, routes it, and returns a response |

## Key Characteristics

| Property | Value |
|---|---|
| Framework | Axum |
| Response format | SSE stream (when `stream: true`) or JSON |
| Routing threshold | Confidence > 0.98 — requests below this are forwarded to the LLM |
| Routing condition | Only prompts ≤ 200 characters are eligible for routing |

## Class Diagram

```mermaid
classDiagram
    class ChatResponse {
        <<enumeration>>
        Stream~Sse~EventStream~~
        Json~Json~Value~~
    }

    class LlmRequest {
        +get_routable_prompt() Option~&str~
        +is_stream() bool
    }

    ChatResponse ..> LlmRequest : built from
```

## Sequence Diagrams

### GET /models

```mermaid
sequenceDiagram
    participant Client
    participant Handler as get_models()

    Client->>Handler: GET /models
    Handler-->>Client: JSON { models: [...], data: [...], object: "list" }
```

### POST /chat/completions — Routed Request

When the last user message is short enough and the router returns high confidence, the request is handled entirely by the `Interpreter` without touching the downstream LLM.

```mermaid
sequenceDiagram
    participant Client
    participant Handler as post_chat_completions()
    participant Router as RouterClient
    participant Interpreter

    Client->>Handler: POST /chat/completions { messages, stream }
    Handler->>Handler: get_routable_prompt() → Some(prompt)
    Handler->>Router: get_routing(prompt)
    Router-->>Handler: RouterResponse { text, confidence > 0.98 }
    Handler->>Interpreter: eval(routing.text)
    Interpreter-->>Handler: StringStream
    Handler-->>Client: SSE stream
```

### POST /chat/completions — LLM Fallback

When the prompt is not routable (too long, no user message, or low router confidence), the request is proxied directly to the local LLM.

```mermaid
sequenceDiagram
    participant Client
    participant Handler as post_chat_completions()
    participant Router as RouterClient
    participant LLM as Local LLM

    Client->>Handler: POST /chat/completions { messages, stream }

    alt prompt not routable (missing or > 200 chars)
        Note over Handler: skip routing
    else confidence ≤ 0.98
        Handler->>Router: get_routing(prompt)
        Router-->>Handler: RouterResponse { confidence ≤ 0.98 }
        Note over Handler: fall through to LLM
    end

    Handler->>LLM: POST model_url (forward original request)
    alt stream: true
        LLM-->>Handler: streaming byte chunks
        Handler-->>Client: SSE stream
    else stream: false
        LLM-->>Handler: JSON response body
        Handler-->>Client: JSON
    end
```

## ChatResponse

An internal enum that unifies the two possible response shapes. Axum's `IntoResponse` is implemented so that both variants serialise correctly to the HTTP response.

| Variant | Produced from | HTTP response |
|---|---|---|
| `Stream(Sse<EventStream>)` | `StringStream` or `ByteStream` | `text/event-stream` |
| `Json(Json<Value>)` | `serde_json::Value` | `application/json` |
