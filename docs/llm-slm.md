# LLM — SLM Client Module

The `llm::slm` module provides a lightweight HTTP client for the **Small Language Model (SLM)** service — a local, low-latency model used for tool selection, function-call generation, and quick contextual responses. The SLM server streams its response as plain text chunks over HTTP.

## Key Characteristics

| Property | Value |
|---|---|
| Transport | HTTP POST, streaming response body |
| Default URL | `http://jarvis.local:1964/` |
| Request format | JSON body (`SlmRequest`) |
| Response format | Plain text stream (UTF-8 chunks) |
| Blocking variant | `block_exec` collects the full stream synchronously |

## Class Diagram

```mermaid
classDiagram
    class SlmRequest {
        <<struct>>
        +prompt      : String
        +system      : Option~String~
        +tools       : Option~String~
        +context     : Option~String~
        +use_history : bool
        +new(prompt: &str)$ SlmRequest
        +with_tools(prompt: &str, tools: &str)$ SlmRequest
        +exec() Result~String~
        +add_system(system: &str)
        +set_use_history(use_history: bool)
        +get_prompt() &str
        +get_system() &Option~String~
    }

    class SlmClient {
        -slm_url     : String
        -http_client : &Client
        +new()$ SlmClient
        +for_url(slm_url: &str)$ SlmClient
        +for_client(slm_url: &str, http_client: &Client)$ SlmClient
        +exec(request: &SlmRequest) StringStream
        +block_exec(request: &SlmRequest) Result~String~
    }

    SlmRequest ..> SlmClient : creates via new() and calls exec()
    SlmClient ..> StringStream : yields
```

## Sequence Diagrams

### Async Streaming Request

`exec()` returns a `StringStream` immediately. The caller drives the stream by polling it.

```mermaid
sequenceDiagram
    participant Caller
    participant SlmClient
    participant SLM as SLM Server

    Caller->>SlmClient: exec(request)
    SlmClient->>SLM: POST / (JSON body)
    SLM-->>SlmClient: HTTP 200 — streaming body

    loop for each chunk
        SLM-->>SlmClient: bytes
        SlmClient-->>Caller: yield Ok(text)
    end
```

### Blocking Request

`block_exec` drives the async `exec` to completion via `futures::executor::block_on`, accumulating all chunks into a single `String`.

```mermaid
sequenceDiagram
    participant Caller
    participant SlmClient

    Caller->>SlmClient: block_exec(request)
    SlmClient->>SlmClient: block_on(exec(request))
    loop accumulate chunks
        SlmClient->>SlmClient: result.push_str(chunk)
    end
    SlmClient-->>Caller: Result~String~
```

## HTTP Client Strategy

A `lazy_static` global `reqwest::Client` is used when `SlmClient::new()` or `for_url()` is called. `for_client()` accepts a borrowed client, allowing the caller to supply a shared instance (used by `ToolClient`).

## Request Payload

```json
{
    "prompt": "What is the temperature in the kitchen?",
    "system": "You are a helpful assistant.",
    "tools": "hera",
    "context": null,
    "use_history": false
}
```

All fields except `prompt` are optional. `tools` names the tool-set that the SLM should consider when generating a function call. `use_history` controls whether the SLM uses its conversation history.
