# Router Module

The router module provides a client implementation for communicating with a remote LLM routing server over TCP. It handles automatic reconnections and manages the request-response lifecycle through a background connection manager.

## Class Diagram

```mermaid
classDiagram
    class RouterResponse {
        <<struct>>
        +text: String
        +confidence: f32
    }

    class TcpMessage {
        <<enumeration>>
        Ping
        Pong
        Request(prompt: String)
        Response(text: String, confidence: f32, duration: f32)
    }

    class ControlMessage {
        <<enumeration>>
        Send(request: TcpMessage, response_channel_sender: oneshot::Sender~TcpMessage~)
        Shutdown
    }

    class RouterClient {
        -control_channel_sender: mpsc::Sender~ControlMessage~
        +connect(router_address: &str) Result~Self~
        +request(prompt: &str) Result~RouterResponse~
    }

    class TcpConnection {
        -tcp_reader: ReadHalf~TcpStream~
        -tcp_writer: WriteHalf~TcpStream~
        -response_channel_sender: Option~oneshot::Sender~TcpMessage~~
        +new(tcp_stream: TcpStream) TcpConnection
        ~run(control_channel_receiver: &mut mpsc::Receiver~ControlMessage~) Result~()~
        ~on_heartbeat() Result~()~
        ~on_control_message(control_message: ControlMessage) Result~()~
        ~on_tcp_message(tcp_message: &TcpMessage) Result~()~
        ~write_tcp_message(tcp_message: &TcpMessage) Result~()~
        ~read_tcp_message() Result~TcpMessage~
    }

    class MpscChannel {
        <<interface>>
        +send(msg: T)
        +recv() T
    }

    class OneshotChannel {
        <<interface>>
        +send(val: T)
        +recv() T
    }

    %% Relationships
    RouterClient --> MpscChannel : uses (control_channel_sender)
    TcpConnection --> MpscChannel : uses (control_channel_receiver)
    RouterClient ..> OneshotChannel : creates (for request)
    TcpConnection ..> OneshotChannel : uses (response_channel_sender)
    
    RouterClient ..> ControlMessage : sends
    RouterClient ..> TcpMessage : uses
    RouterClient --> TcpConnection : manages (via background task)
    TcpConnection ..> ControlMessage : processes
    TcpConnection ..> TcpMessage : sends/receives
    TcpMessage ..> RouterResponse : converts to
```

## Sequence Flow

### Connection Establishment
The `connect` method starts a background task that handles the connection loop and automatic retries.

```mermaid
sequenceDiagram
    participant App as Application
    participant Client as RouterClient
    participant Mpsc as MpscChannel (Control)
    participant Manager as TcpConnection (Background Task)
    participant Server as Remote Router Server

    App->>Client: connect(router_address)
    activate Client
    Client->>Mpsc: create channel
    Client->>Manager: spawn background task
    Client-->>App: return RouterClient
    deactivate Client

    Note over Manager, Server: Background Loop
    loop Every 5 seconds (if connection lost)
        Manager->>Server: TCP connect(router_address)
        alt success
            Server-->>Manager: established connection
            Manager->>Manager: run() event loop
        else failure
            Manager->>Manager: retry after interval
        end
    end
```

### Request Lifecycle
The following sequence diagram illustrates the flow of a single request from the `RouterClient` to the remote server and back.

```mermaid
sequenceDiagram
    participant App as Application
    participant Client as RouterClient
    participant Mpsc as MpscChannel (Control)
    participant Manager as TcpConnection (Background Task)
    participant Oneshot as OneshotChannel (Response)
    participant Server as Remote Router Server

    App->>Client: request(prompt)
    activate Client
    Client->>Oneshot: create channel
    Client->>Mpsc: send(ControlMessage::Send { request, oneshot_sender })
    activate Mpsc
    deactivate Mpsc
    
    Mpsc-->>Manager: recv(ControlMessage)
    activate Manager
    Manager->>Server: write_tcp_message(TcpMessage::Request)
    Manager->>Oneshot: send(TcpMessage::Response)
    deactivate Manager
    
    Server-->>Manager: read_tcp_message(TcpMessage::Response)
    activate Manager
    Manager->>Oneshot: send(TcpMessage::Response)
    deactivate Manager
    
    Oneshot-->>Client: recv(TcpMessage::Response)
    Client->>Client: convert to RouterResponse
    Client-->>App: return RouterResponse
    deactivate Client
```

### Request Lifecycle:
1.  **Application** calls `request()` on the `RouterClient`.
2.  **`RouterClient`** creates a new `OneshotChannel` for the response.
3.  **`RouterClient`** wraps the request in a `ControlMessage::Send` and sends it via the `MpscChannel`.
4.  **`TcpConnection`** (running in a background task) receives the control message from the `MpscChannel`.
5.  **`TcpConnection`** serializes the `TcpMessage::Request` and writes it to the **Remote Router Server** via the TCP stream.
6.  **`TcpConnection`** stores the `OneshotChannel` sender.
7.  **Remote Router Server** processes the prompt and sends back a `TcpMessage::Response`.
8.  **`TcpConnection`** reads the response from the TCP stream.
9.  **`TcpConnection`** uses the stored `OneshotChannel` sender to pass the `TcpMessage` back to the `RouterClient`.
10. **`RouterClient`** receives the message, converts it into a `RouterResponse`, and returns it to the **Application**.
