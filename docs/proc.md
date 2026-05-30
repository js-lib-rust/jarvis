# Proc Module

The `proc` module is the **execution engine**. When the router classifies a prompt with high confidence, it returns a multi-line action plan. The `Interpreter` parses that plan and executes each action step by step, accumulating facts along the way. The final streamed response includes both the reasoning trace and any LLM-generated text.

## Key Characteristics

| Property | Value |
|---|---|
| Input | Multi-line routing text (`agent:prompt` pairs) |
| Execution | Sequential — each step can depend on the output of previous steps |
| Variable injection | `${key}` placeholders in prompts are replaced from the facts accumulator |
| Output | `StringStream` — reasoning chunks followed by any LLM text streams |

## Supported Agents

| Agent name | Handler |
|---|---|
| `time-service` | `TimeServiceAgent` |
| `user-profile` | `UserProfileAgent` |
| `measure-units` | `MeasurementUnitAgent` |
| `health` | `HealthAgent` |
| `weather` | `WeatherAgent` |
| `home-automation` | `HeraAgent` |
| `query` | `QueryAgent` (returns `StringStream`) |
| `printer` | `PrinterAgent` (returns `StringStream`) |

## Class Diagram

```mermaid
classDiagram
    class Interpreter {
        -tool_client : &ToolClient
        +new(tool_client: &ToolClient)$ Interpreter
        +eval(text: &str) StringStream
    }

    class Action {
        <<struct>>
        +agent  : &str
        +prompt : &str
    }

    class FactsStack {
        +context : String
        +facts   : HashMap~String, Value~
        +new()$ FactsStack
        +push_prompt(prompt: &str)
        +push_facts(facts: &str)
        +inject_variables(prompt: &str) String
        +get_reasoning_stream() StringStream
    }

    Interpreter "1" *-- "1" FactsStack  : owns per eval() call
    Interpreter ..> Action              : parses from routing text
    FactsStack  ..> StringStream        : emits reasoning chunks
```

## Sequence Diagram

### eval() — Multi-Step Execution

```mermaid
sequenceDiagram
    participant API as api::chat
    participant Interp as Interpreter
    participant FS as FactsStack
    participant Agent as Domain Agent

    API->>Interp: eval(routing_text)
    Interp->>Interp: parse lines → Vec~Action~

    loop for each Action
        Interp->>FS: inject_variables(action.prompt)
        FS-->>Interp: resolved prompt
        Interp->>FS: push_prompt(resolved_prompt)
        Interp->>Agent: exec(SlmRequest { prompt, system=context })
        Agent-->>Interp: Result~String~ (facts)
        Interp->>FS: push_facts(facts)

        alt agent is "query" or "printer"
            Interp->>Agent: exec(SlmRequest { prompt, system=context })
            Agent-->>Interp: StringStream
            Interp->>Interp: enqueue stream
        end
    end

    Interp->>FS: get_reasoning_stream()
    FS-->>Interp: LlmChunk stream (reasoning trace)
    Interp->>Interp: chain(reasoning_stream, ...queued_streams)
    Interp-->>API: combined StringStream
```

## FactsStack

`FactsStack` maintains two parallel representations of accumulated knowledge:

- **`context`** (`String`): a human-readable prompt/answer log passed as the `system` message to subsequent agents.
- **`facts`** (`HashMap<String, Value>`): a key-value map built from single-entry JSON objects returned by agents, used for `${key}` variable substitution.

### Variable Injection

Prompts may contain `${key}` placeholders. Before an action is executed, `inject_variables` scans the prompt and substitutes any keys found in `facts`. Unresolved placeholders are left as-is.

Example:

```
Router plan:
  time-service: Get the date for today.
  health: Read the medical records for ${username} on ${date}.
```

After `time-service` pushes `{"date": "2026-05-30"}` to the facts stack, the second prompt becomes:

```
  health: Read the medical records for ${username} on 2026-05-30.
```

### Reasoning Stream

After all synchronous agents complete, `get_reasoning_stream()` serialises the accumulated `context` into a sequence of `LlmChunk` objects (with `reasoning_content` set) and emits them as the first part of the response stream. This lets the client observe the chain-of-thought before the final LLM-generated answer arrives.
