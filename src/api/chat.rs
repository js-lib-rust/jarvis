use crate::types::ByteStream;
use crate::util::string::ellipsis;
use crate::{
    error::AppError,
    llm::LlmRequest,
    llm::RouterMessage,
    proc::Interpreter,
    types::{AppState, Result, StringStream},
};
use crate::{
    llm::{LlmModel, LlmModelData, LlmModels},
    types::EventStream,
};
use askama::Template;
use axum::{
    Json, Router,
    extract::State,
    response::{IntoResponse, Sse, sse::Event},
    routing::{get, post},
};
use futures::{StreamExt, TryStreamExt};
use log::{debug, trace};
use serde_json::Value;
use std::sync::Arc;

enum ChatResponse {
    Stream(Sse<EventStream>),
    Json(Json<Value>),
}

impl IntoResponse for ChatResponse {
    fn into_response(self) -> axum::response::Response {
        trace!("into_response(self) -> axum::response::Response");
        match self {
            ChatResponse::Stream(sse) => sse.into_response(),
            ChatResponse::Json(json) => json.into_response(),
        }
    }
}

impl From<StringStream> for ChatResponse {
    fn from(string_stream: StringStream) -> Self {
        let event_stream = string_stream.map_ok(|s| Event::default().data(s)).boxed();
        ChatResponse::Stream(Sse::new(event_stream))
    }
}

impl From<ByteStream> for ChatResponse {
    fn from(byte_stream: ByteStream) -> Self {
        let string_stream = byte_stream
            .map_ok(|bytes| String::from_utf8_lossy(&bytes).to_string())
            .map_ok(|string| string.trim_start_matches("data:").to_string())
            .boxed();
        ChatResponse::from(string_stream)
    }
}

impl From<Value> for ChatResponse {
    fn from(json_value: Value) -> Self {
        ChatResponse::Json(Json(json_value))
    }
}

pub(crate) fn router(app_context: Arc<AppState>) -> Router {
    Router::new().route("/models", get(get_models)).route(
        "/chat/completions",
        post(post_chat_completions).with_state(app_context),
    )
}

// REQUEST HANDLERS

async fn get_models() -> Json<LlmModels> {
    trace!("get_models() -> Json<Models>");

    let model = LlmModel::new("JARVIS");
    let data = LlmModelData::new("JARVIS");
    Json(LlmModels::new(vec![model], vec![data]))
}

async fn post_chat_completions(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<LlmRequest>,
) -> Result<ChatResponse> {
    trace!(
        "post_chat_completions(State(app_context): State<Arc<AppContext>>, Json(request): Json<Request>) -> ChatResponse"
    );

    let prompt = request.get_prompt();
    if let RouterMessage::Response {
        text,
        confidence,
        duration,
    } = get_routing(app_state.clone(), &prompt).await?
    {
        if confidence > 0.98 {
            debug!("prompt: {}", ellipsis(prompt, 100));
            debug!("text: {text}, confidence: {confidence}, duration: {duration}");

            let mut interpreter = Interpreter::new(&app_state.tool_client);
            let string_stream = interpreter.eval(&text).await;
            return Ok(ChatResponse::from(string_stream));
        } else {
            debug!("prompt: {}", ellipsis(prompt, 100));
            debug!("confidence: {confidence}, duration: {duration}");
        }
    }

    let response = app_state
        .http_client
        .post(&app_state.model_url)
        .json(&request)
        .send()
        .await?;

    if request.is_stream() {
        let byte_stream = response
            .bytes_stream()
            .map_err(|err| AppError::Fatal(err.to_string()))
            .boxed();
        Ok(ChatResponse::from(byte_stream))
    } else {
        let bytes = response.bytes().await?;
        let json_value: Value = serde_json::from_slice(&bytes)?;
        Ok(ChatResponse::from(json_value))
    }
}

// UTILS

async fn get_routing(app_context: Arc<AppState>, prompt: &str) -> Result<RouterMessage> {
    trace!("get_routing(app_context: Arc<AppContext>, prompt: &str) -> llm_router::Message");
    app_context.router_client.request(prompt).await
}

fn _router_report(prompt: &str, response: &str, confidence: f32, duration: f32) -> Result<String> {
    trace!(
        "router_report(prompt: &str, response: &str, confidence: f32, duration: f32) -> Result<String>"
    );

    struct Action<'a> {
        domain: &'a str,
        operation: &'a str,
    }

    #[derive(Template)]
    #[template(path = "router_report.md")]
    struct Markdown<'a> {
        prompt: &'a str,
        actions: Vec<Action<'a>>,
        confidence: f32,
        duration: f32,
    }

    let response = response.replace("$", "");
    let actions: Vec<Action> = response
        .lines()
        .map(|line| line.split_once(":"))
        .flatten()
        .map(|(domain, operation)| Action { domain, operation })
        .collect();

    let markdown = Markdown {
        prompt,
        actions,
        confidence,
        duration,
    };
    Ok(markdown.render()?)
}
